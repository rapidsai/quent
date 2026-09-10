// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;
use std::{sync::Arc, time::Duration};

use moka::future::Cache;
use quent_analyzer::context::{ContextId, ContextIndex, ContextInventory};
use quent_events::Event;
use quent_query_engine_analyzer::ui::UiAnalyzer;
use quent_query_engine_ui as ui;
use tracing::info_span;
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};

/// Reads one source's events for a context id. Called once per context that
/// makes up a root; the cache chains the results.
pub type ImporterFn<A> = dyn Fn(Uuid) -> ServerResult<Box<dyn Iterator<Item = Event<<A as UiAnalyzer>::Event>>>>
    + Send
    + Sync;

/// Produces the [`ContextIndex`] used to locate the contexts backing each analysis target.
pub type ListerFn = dyn Fn() -> ServerResult<ContextIndex> + Send + Sync;

/// Scans every `<output_dir>/<ctx>/` directory and indexes its entities.
///
/// The inventory callback determines how entities and analysis targets are identified for the
/// application's event schema. The index is rebuilt from scratch on every call.
pub fn index_contexts(
    output_dir: &Path,
    inventory: impl Fn(Uuid) -> ServerResult<ContextInventory>,
) -> ServerResult<ContextIndex> {
    let mut index = ContextIndex::default();
    for entry in std::fs::read_dir(output_dir)? {
        let context_dir = entry?.path();
        let Some(context_id) = context_dir
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| Uuid::parse_str(s).ok())
        else {
            continue;
        };

        index.add_inventory(context_id.into(), inventory(context_id)?);
    }
    Ok(index)
}

/// Chain one source-importer call per context into a single event stream.
fn chain_context_events<A: UiAnalyzer>(
    importer: &ImporterFn<A>,
    context_ids: &[ContextId],
) -> ServerResult<Box<dyn Iterator<Item = Event<A::Event>>>>
where
    A::Event: 'static,
{
    let mut streams: Vec<Box<dyn Iterator<Item = Event<A::Event>>>> =
        Vec::with_capacity(context_ids.len());
    for &context_id in context_ids {
        streams.push(importer(context_id.into_uuid())?);
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
        Ok((self.lister)()?.analysis_target_ids().collect())
    }

    /// List an engine's contributing contexts without blocking the async executor.
    pub(crate) async fn contexts(&self, engine_id: Uuid) -> ServerResult<ui::EngineContexts> {
        let lister = Arc::clone(&self.lister);
        tokio::task::spawn_blocking(move || {
            let index = lister()?;
            Ok(ui::EngineContexts {
                engine_id,
                context_ids: index
                    .contexts_of_analysis_target(engine_id)
                    .into_iter()
                    .map(ContextId::into_uuid)
                    .collect(),
            })
        })
        .await
        .map_err(|error| ServerError::Cache(format!("blocking task panicked: {error}")))?
    }

    pub(crate) async fn list_with_metadata(&self) -> ServerResult<Vec<ui::Engine>> {
        let lister = Arc::clone(&self.lister);
        let importer = Arc::clone(&self.importer);
        tokio::task::spawn_blocking(move || {
            let _span = info_span!("list_with_metadata").entered();
            let index = lister()?;
            index
                .analysis_target_ids()
                .map(|engine_id| {
                    let events = chain_context_events::<A>(
                        &*importer,
                        &index.contexts_of_analysis_target(engine_id),
                    )?;
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
                    let context_ids = lister()?.contexts_of_analysis_target(engine_id);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn engine_context_inventory_is_deduplicated_and_sorted() {
        let engine_id = Uuid::from_u128(1);
        let earlier = Uuid::from_u128(2);
        let later = Uuid::from_u128(3);
        let mut index = ContextIndex::default();
        let inventory = || ContextInventory {
            analysis_target_ids: std::collections::BTreeSet::from([engine_id]),
        };
        index.add_inventory(later.into(), inventory());
        index.add_inventory(earlier.into(), inventory());
        index.add_inventory(later.into(), inventory());

        assert_eq!(
            index.contexts_of_analysis_target(engine_id),
            vec![earlier.into(), later.into()]
        );
        assert!(
            index
                .contexts_of_analysis_target(Uuid::from_u128(4))
                .is_empty()
        );
    }

    #[test]
    fn context_inventory_associates_all_contexts_with_the_analysis_target() {
        let engine_id = Uuid::from_u128(1);
        let engine_context = Uuid::from_u128(2);
        let worker_context = Uuid::from_u128(3);
        let mut index = ContextIndex::default();
        index.add_inventory(
            engine_context.into(),
            ContextInventory {
                analysis_target_ids: std::collections::BTreeSet::from([engine_id]),
            },
        );
        index.add_inventory(
            worker_context.into(),
            ContextInventory {
                analysis_target_ids: std::collections::BTreeSet::from([engine_id]),
            },
        );

        assert_eq!(
            index.contexts_of_analysis_target(engine_id),
            vec![engine_context.into(), worker_context.into()]
        );
    }
}
