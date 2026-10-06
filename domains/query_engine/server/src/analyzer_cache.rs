// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use quent_analyzer::context::{ContextId, ContextIndex};
use quent_analyzer::service::{AnalysisCache, BlockingTasks};
use quent_query_engine_analyzer::ui::{ContextEvent, UiAnalyzer, ViewerContext};
use quent_query_engine_ui as ui;
use tracing::info_span;
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};

/// Imports one context through the model's common event store.
pub type ImporterFn<A> = dyn Fn(Uuid) -> ServerResult<ViewerContext<A>> + Send + Sync;

type AnalyzerImporterFn<A> = dyn Fn(ContextId) -> ServerResult<Box<dyn Iterator<Item = ContextEvent<<A as UiAnalyzer>::Event>>>>
    + Send
    + Sync;

/// Produces the [`ContextIndex`] used to locate the contexts backing each analysis target.
pub type ListerFn = dyn Fn() -> ServerResult<ContextIndex> + Send + Sync;

/// Chain one source-importer call per context into a single event stream.
fn chain_context_events<A: UiAnalyzer>(
    importer: &AnalyzerImporterFn<A>,
    context_ids: &[ContextId],
) -> ServerResult<Box<dyn Iterator<Item = ContextEvent<A::Event>>>>
where
    A::Event: 'static,
{
    let mut streams: Vec<Box<dyn Iterator<Item = ContextEvent<A::Event>>>> =
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
    analyzers: AnalysisCache<Uuid, A, ServerError>,
    importer: Arc<AnalyzerImporterFn<A>>,
    lister: Arc<ListerFn>,
    tasks: BlockingTasks,
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
            tasks: self.tasks.clone(),
        }
    }
}

impl<A> AnalyzerCache<A>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    pub(crate) fn new(importer: Box<ImporterFn<A>>, lister: Box<ListerFn>) -> Self {
        let importer = move |context_id: ContextId| {
            Ok(
                Box::new(importer(context_id.into_uuid())?.into_events(context_id))
                    as Box<dyn Iterator<Item = ContextEvent<A::Event>>>,
            )
        };
        Self::from_context_event_importer(Arc::new(importer), lister)
    }

    fn from_context_event_importer(
        importer: Arc<AnalyzerImporterFn<A>>,
        lister: Box<ListerFn>,
    ) -> Self {
        let tasks = BlockingTasks::new(NonZeroUsize::new(4).unwrap());
        Self {
            analyzers: AnalysisCache::new(32, Duration::from_hours(24), tasks.clone()),
            importer,
            lister: Arc::from(lister),
            tasks,
        }
    }

    pub(crate) fn list(&self) -> ServerResult<Vec<Uuid>> {
        Ok((self.lister)()?.analysis_target_ids().collect())
    }

    /// List an engine's contributing contexts without blocking the async executor.
    pub(crate) async fn contexts(&self, engine_id: Uuid) -> ServerResult<ui::EngineContexts> {
        let lister = Arc::clone(&self.lister);
        self.tasks
            .run(move || {
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
        self.tasks
            .run(move || {
                let _span = info_span!("list_with_metadata").entered();
                let index = lister()?;
                index
                    .analysis_target_ids()
                    .map(|engine_id| {
                        let events = chain_context_events::<A>(
                            &*importer,
                            &index.contexts_of_analysis_target(engine_id),
                        )?;
                        Ok(A::extract_engine_from_contexts(engine_id, events)?)
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
            .get_with(engine_id, move || -> ServerResult<A> {
                let _span = info_span!("load_engine", %engine_id).entered();
                let context_ids = lister()?.contexts_of_analysis_target(engine_id);
                let events = chain_context_events::<A>(&*importer, &context_ids)?;
                Ok(A::try_new_from_contexts(engine_id, events)?)
            })
            .await
            .map_err(|error| ServerError::Cache(error.to_string()))
    }

    /// Return a representative aggregate analyzer containing `context_id`.
    ///
    /// A context may contribute to several analysis targets. Each such
    /// analyzer receives the same context-local events, so context-owned views
    /// can use the lowest target UUID deterministically. `None` means the
    /// context is absent from the current inventory.
    pub async fn get_for_context(&self, context_id: Uuid) -> ServerResult<Option<Arc<A>>> {
        let lister = Arc::clone(&self.lister);
        let target_id = self
            .tasks
            .run(move || -> ServerResult<Option<Uuid>> {
                Ok(lister()?
                    .analysis_targets_of_context(context_id.into())
                    .into_iter()
                    .next())
            })
            .await
            .map_err(|error| ServerError::Cache(format!("blocking task panicked: {error}")))??;

        match target_id {
            Some(target_id) => self.get(target_id).await.map(Some),
            None => Ok(None),
        }
    }
}
