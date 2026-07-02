// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::{sync::Arc, time::Duration};

use moka::future::Cache;
use quent_events::{EntityEvent, Event};
use quent_exporter::{
    FileSystemFormat, FileSystemImporterOptions, ImporterOptions, create_importer,
};
use quent_query_engine_analyzer::ui::UiAnalyzer;
use quent_query_engine_model::{engine::EngineEvent, worker::WorkerEvent};
use quent_query_engine_ui as ui;
use tracing::info_span;
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};

/// Reads one source's events for a context id. Called once per context that
/// makes up a root; the cache chains the results.
pub type ImporterFn<A> = dyn Fn(Uuid) -> ServerResult<Box<dyn Iterator<Item = Event<<A as UiAnalyzer>::Event>>>>
    + Send
    + Sync;

/// Produces the [`EngineIndex`] of available engines and the contexts backing
/// each.
pub type ListerFn = dyn Fn() -> ServerResult<EngineIndex> + Send + Sync;

/// Which contexts make up each query-engine instance's telemetry.
///
/// An engine instance spans multiple processes — the engine itself and its
/// workers — each of which may hold its own context. This index ties them
/// together by engine id: the engine's own context plus every context in which
/// a worker of that engine appears.
///
/// Built by scanning context directories (see [`index_query_engines`]).
#[derive(Debug, Default)]
pub struct EngineIndex {
    /// Engine id to the contexts holding its telemetry (its own and its
    /// workers'). `BTreeSet` so it is deduplicated and deterministically ordered.
    contexts: HashMap<Uuid, BTreeSet<Uuid>>,
    /// Engine id to its workers and the context each was found in.
    workers: HashMap<Uuid, Vec<(Uuid, Uuid)>>,
}

impl EngineIndex {
    fn attribute_context(&mut self, engine_id: Uuid, context_id: Uuid) {
        self.contexts
            .entry(engine_id)
            .or_default()
            .insert(context_id);
    }

    fn add_worker(&mut self, engine_id: Uuid, worker_id: Uuid, context_id: Uuid) {
        self.attribute_context(engine_id, context_id);
        self.workers
            .entry(engine_id)
            .or_default()
            .push((worker_id, context_id));
    }

    /// All known engine ids.
    pub fn engine_ids(&self) -> impl Iterator<Item = Uuid> + '_ {
        self.contexts.keys().copied()
    }

    /// The contexts whose events make up `engine_id`'s telemetry (engine plus
    /// workers).
    pub fn contexts_of(&self, engine_id: Uuid) -> Vec<Uuid> {
        self.contexts
            .get(&engine_id)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// `engine_id`'s workers as `(worker_id, context_id)` pairs.
    pub fn workers_of(&self, engine_id: Uuid) -> &[(Uuid, Uuid)] {
        self.workers.get(&engine_id).map_or(&[], Vec::as_slice)
    }
}

/// A dumb lister for query-engine-domain models: scan every `<output_dir>/<ctx>/`
/// directory and, from its engine and worker streams, index each engine to the
/// contexts that make up its telemetry (the engine's own context plus the
/// contexts of its workers, found via each worker's `parent_engine_id`).
///
/// "Dumb" because it re-scans and rebuilds from scratch on every call — a real
/// history/indexing service replaces this later.
pub fn index_query_engines(output_dir: &Path) -> ServerResult<EngineIndex> {
    let mut index = EngineIndex::default();
    for entry in std::fs::read_dir(output_dir)? {
        let context_dir = entry?.path();
        let Some(context_id) = context_dir
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| Uuid::parse_str(s).ok())
        else {
            continue;
        };
        // Each context's serialization format is detected from its own streams.
        let Some(format) = FileSystemFormat::detect(&context_dir) else {
            continue;
        };

        // Engines living in this context.
        let engine_dir = context_dir.join(<EngineEvent as EntityEvent>::NAME);
        if engine_dir.is_dir() {
            let importer = create_importer::<EngineEvent>(&ImporterOptions::FileSystem(
                FileSystemImporterOptions {
                    format,
                    path: engine_dir,
                },
            ))?;
            let mut seen = HashSet::new();
            for event in importer {
                if seen.insert(event.id) {
                    index.attribute_context(event.id, context_id);
                }
            }
        }

        // Workers living in this context attribute it to their parent engine.
        let worker_dir = context_dir.join(<WorkerEvent as EntityEvent>::NAME);
        if worker_dir.is_dir() {
            let importer = create_importer::<WorkerEvent>(&ImporterOptions::FileSystem(
                FileSystemImporterOptions {
                    format,
                    path: worker_dir,
                },
            ))?;
            let mut seen = HashSet::new();
            for event in importer {
                if let WorkerEvent::Init(init) = &event.data
                    && seen.insert(event.id)
                {
                    index.add_worker(init.parent_engine_id.uuid(), event.id, context_id);
                }
            }
        }
    }
    Ok(index)
}

/// Chain one source-importer call per context into a single event stream.
fn chain_context_events<A: UiAnalyzer>(
    importer: &ImporterFn<A>,
    context_ids: &[Uuid],
) -> ServerResult<Box<dyn Iterator<Item = Event<A::Event>>>>
where
    A::Event: 'static,
{
    let mut streams: Vec<Box<dyn Iterator<Item = Event<A::Event>>>> =
        Vec::with_capacity(context_ids.len());
    for &context_id in context_ids {
        streams.push(importer(context_id)?);
    }
    Ok(Box::new(streams.into_iter().flatten()))
}

/// Cache for analyzer instances, keyed by engine ID.
pub struct AnalyzerCache<A>
where
    A: UiAnalyzer,
{
    analyzers: Cache<Uuid, Arc<A>>,
    importer: Arc<ImporterFn<A>>,
    lister: Arc<ListerFn>,
}

impl<A> Clone for AnalyzerCache<A>
where
    A: UiAnalyzer,
{
    fn clone(&self) -> Self {
        Self {
            analyzers: self.analyzers.clone(),
            importer: Arc::clone(&self.importer),
            lister: Arc::clone(&self.lister),
        }
    }
}

impl<A> AnalyzerCache<A>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    pub(crate) fn new(importer: Box<ImporterFn<A>>, lister: Box<ListerFn>) -> Self {
        Self {
            analyzers: Cache::builder()
                .max_capacity(32)
                .time_to_idle(Duration::from_hours(24))
                .build(),
            importer: Arc::from(importer),
            lister: Arc::from(lister),
        }
    }

    pub(crate) fn list(&self) -> ServerResult<Vec<Uuid>> {
        Ok((self.lister)()?.engine_ids().collect())
    }

    pub(crate) async fn list_with_metadata(&self) -> ServerResult<Vec<ui::Engine>> {
        let lister = Arc::clone(&self.lister);
        let importer = Arc::clone(&self.importer);
        tokio::task::spawn_blocking(move || {
            let _span = info_span!("list_with_metadata").entered();
            let index = lister()?;
            index
                .engine_ids()
                .map(|engine_id| {
                    let events =
                        chain_context_events::<A>(&*importer, &index.contexts_of(engine_id))?;
                    Ok(A::extract_engine(engine_id, events)?)
                })
                .collect()
        })
        .await
        .map_err(|e| ServerError::Cache(format!("blocking task panicked: {e}")))?
    }

    pub(crate) async fn get(&self, engine_id: Uuid) -> ServerResult<Arc<A>> {
        let lister = Arc::clone(&self.lister);
        let importer = Arc::clone(&self.importer);
        self.analyzers
            .entry(engine_id)
            .or_try_insert_with(async {
                tokio::task::spawn_blocking(move || -> ServerResult<Arc<A>> {
                    let _span = info_span!("load_engine", %engine_id).entered();
                    let context_ids = lister()?.contexts_of(engine_id);
                    let events = chain_context_events::<A>(&*importer, &context_ids)?;
                    Ok(A::try_new(engine_id, events).map(Arc::new)?)
                })
                .await
                .map_err(|e| ServerError::Cache(format!("blocking task panicked: {e}")))?
            })
            .await
            .map(|v| v.into_value())
            .map_err(|e: Arc<ServerError>| ServerError::Cache(format!("{e:?}")))
    }
}
