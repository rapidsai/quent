// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use quent_analyzer::{AnalyzerError, AnalyzerResult, context::ContextId};
use quent_events::Event;
use quent_io_types::ImporterResult;
use quent_query_engine_ui as ui;
use quent_ui::{
    entities::{request::EntityListRequest, response::EntityListResponse},
    timeline::{
        categorical::CategoricalTimelineRequest,
        request::{BulkChunkedTimelineRequest, BulkTimelineRequest, SingleTimelineRequest},
        response::{
            BulkChunkedTimelinesResponse, BulkTimelinesResponse, BulkTimelinesResponseEntry,
            SingleTimelineResponse,
        },
    },
};
use uuid::Uuid;

use crate::QueryEngineModel;

/// Availability of one generated event stream in a loaded runtime context.
///
/// This mirrors the states retained by the common store without coupling
/// analyzers to a particular storage implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextStreamAvailability {
    /// The context's model does not declare this stream.
    Undeclared,
    /// The model declares the stream, but no stream directory is available.
    Missing,
    /// The stream is present and decoded successfully, but contains no events.
    Empty,
    /// The stream contains at least one decoded event.
    Populated,
}

/// Storage metadata retained while a runtime context is analyzed.
#[derive(Clone, Debug, Default)]
pub struct ContextMetadata {
    streams: Arc<HashMap<String, ContextStreamAvailability>>,
}

impl ContextMetadata {
    /// Create metadata from the availability of the named generated streams.
    pub fn new(streams: impl IntoIterator<Item = (String, ContextStreamAvailability)>) -> Self {
        Self {
            streams: Arc::new(streams.into_iter().collect()),
        }
    }

    /// Look up availability recorded by the context importer.
    ///
    /// `None` means the importer did not provide availability metadata for the
    /// stream. It does not imply that the stream is missing.
    pub fn stream_availability(&self, entity: &str) -> Option<ContextStreamAvailability> {
        self.streams.get(entity).copied()
    }
}

/// One model event together with the runtime context that stored it.
///
/// Entity IDs identify application objects, while a context identifies one
/// instrumentation/export pipeline. Keeping both lets analyzers partition
/// source-local protocols before aggregating an engine that spans contexts.
#[derive(Debug)]
pub struct ContextEvent<T> {
    context_id: ContextId,
    event: Event<T>,
    metadata: ContextMetadata,
}

impl<T> ContextEvent<T> {
    pub fn new(context_id: ContextId, event: Event<T>) -> Self {
        Self {
            context_id,
            event,
            metadata: ContextMetadata::default(),
        }
    }

    /// Attach metadata retained by the common context loader.
    pub fn with_metadata(
        context_id: ContextId,
        event: Event<T>,
        metadata: ContextMetadata,
    ) -> Self {
        Self {
            context_id,
            event,
            metadata,
        }
    }

    pub fn context_id(&self) -> ContextId {
        self.context_id
    }

    pub fn event(&self) -> &Event<T> {
        &self.event
    }

    /// Metadata shared by every event imported from this context.
    pub fn metadata(&self) -> &ContextMetadata {
        &self.metadata
    }

    pub fn into_event(self) -> Event<T> {
        self.event
    }
}

/// Trait for types that can analyze query engine telemetry for the purpose of
/// visualization in a UI.
pub trait UiAnalyzer {
    type Event;

    fn try_new(
        engine_id: Uuid,
        events: impl Iterator<Item = Event<Self::Event>>,
    ) -> AnalyzerResult<Self>
    where
        Self: Sized;

    /// Build an analyzer while retaining each event's source context.
    ///
    /// Existing analyzers can keep implementing [`Self::try_new`]; the default
    /// discards context identity and preserves their previous behavior.
    /// Analyzers that consume source-local protocols should override this and
    /// partition those protocols before application-level aggregation.
    fn try_new_from_contexts(
        engine_id: Uuid,
        events: impl Iterator<Item = ContextEvent<Self::Event>>,
    ) -> AnalyzerResult<Self>
    where
        Self: Sized,
    {
        Self::try_new(engine_id, events.map(ContextEvent::into_event))
    }

    /// Extract engine metadata from an event stream without fully building the model.
    ///
    /// Iterates events until the engine init event is found, then returns a
    /// partial [`Engine`](ui::Engine) (without `duration_s`).
    ///
    /// The common case is for this event to be on of the first events ever
    /// flushed, so it will typically be found early.
    // TODO(johanpel): still this function should be used with care. We need
    // some form of an engine index.
    fn extract_engine(
        engine_id: Uuid,
        events: impl Iterator<Item = Event<Self::Event>>,
    ) -> AnalyzerResult<ui::Engine>
    where
        Self: Sized;

    /// Extract engine metadata while retaining source context for analyzers
    /// that need it. The default preserves the legacy context-free behavior.
    fn extract_engine_from_contexts(
        engine_id: Uuid,
        events: impl Iterator<Item = ContextEvent<Self::Event>>,
    ) -> AnalyzerResult<ui::Engine>
    where
        Self: Sized,
    {
        Self::extract_engine(engine_id, events.map(ContextEvent::into_event))
    }

    /// Deliver a UI-friendly `QueryBundle` with all high-level yet
    /// non-volumous information related to this query.
    fn query_bundle(&self, query_id: Uuid) -> AnalyzerResult<ui::QueryBundle>;

    /// Access the underlying query engine model of this analyzer.
    fn query_engine_model(&self) -> &impl QueryEngineModel;

    /// Return a resource timeline for a single resource (or resource group).
    fn single_resource_timeline(
        &self,
        request: SingleTimelineRequest<ui::QueryFilter, ui::OperatorFilter>,
    ) -> AnalyzerResult<SingleTimelineResponse>;

    /// List the entities matching a scope, window, and filter, ranked by the
    /// requested sort key and sliced to the requested page.
    fn list_entities(
        &self,
        request: EntityListRequest<ui::QueryFilter, ui::OperatorFilter>,
    ) -> AnalyzerResult<EntityListResponse>;

    /// Return a set of resource timelines in bulk.
    fn bulk_resource_timeline(
        &self,
        request: BulkTimelineRequest<ui::QueryFilter, ui::OperatorFilter>,
    ) -> AnalyzerResult<BulkTimelinesResponse>;

    /// Return chunked bulk timelines: multiple time windows per entry.
    ///
    /// The default implementation falls back to one `bulk_resource_timeline`
    /// call per config — correct, but pays the per-call setup cost N times.
    /// Implementors should override this to amortize expensive per-call work
    /// (e.g. iterating every task in the model) across all configs in a
    /// single pass.
    fn bulk_chunked_resource_timeline(
        &self,
        request: BulkChunkedTimelineRequest<ui::QueryFilter, ui::OperatorFilter>,
    ) -> AnalyzerResult<BulkChunkedTimelinesResponse> {
        let mut entries: HashMap<String, Vec<BulkTimelinesResponseEntry>> = request
            .entries
            .keys()
            .map(|k| (k.clone(), Vec::with_capacity(request.configs.len())))
            .collect();

        for config in &request.configs {
            let inner_entries = request
                .entries
                .iter()
                .map(|(k, e)| (k.clone(), e.clone().with_config(*config)))
                .collect();
            let mut response = self.bulk_resource_timeline(BulkTimelineRequest {
                entries: inner_entries,
                app_params: request.app_params.clone(),
            })?;
            for (k, slot) in entries.iter_mut() {
                let entry = response.entries.remove(k.as_str()).unwrap_or_else(|| {
                    BulkTimelinesResponseEntry::Error {
                        message: format!("missing entry '{k}' in chunked fallback"),
                    }
                });
                slot.push(entry);
            }
        }

        Ok(BulkChunkedTimelinesResponse { entries })
    }

    /// Return, for every operator of a query, a binned categorical timeline
    /// over (entity state, analyzer-defined dimension), for one or more
    /// analyzer-declared measures. Powers the UI's data-flow-over-time view of
    /// the query plan.
    ///
    /// The default implementation returns [`AnalyzerError::Unsupported`]
    /// (served as HTTP 501), so existing analyzers keep compiling and the UI
    /// hides the view.
    fn data_flow_timeline(
        &self,
        _request: CategoricalTimelineRequest<ui::QueryFilter>,
    ) -> AnalyzerResult<ui::DataFlowTimelineBinned> {
        Err(AnalyzerError::Unsupported)
    }
}

/// A context's imported model events and stream availability.
pub struct ImportedContext<T> {
    events: Box<dyn Iterator<Item = Event<T>>>,
    metadata: ContextMetadata,
}

impl<T> ImportedContext<T> {
    pub fn new(events: Box<dyn Iterator<Item = Event<T>>>, metadata: ContextMetadata) -> Self {
        Self { events, metadata }
    }

    pub fn into_events(self, context_id: ContextId) -> impl Iterator<Item = ContextEvent<T>> {
        self.events
            .map(move |event| ContextEvent::with_metadata(context_id, event, self.metadata.clone()))
    }
}

pub type ViewerContext<A> = ImportedContext<<A as UiAnalyzer>::Event>;

/// Model viewer entry point for `quent-open`: connects the event importer to
/// the rendering [`UiAnalyzer`].
///
/// `quent-open` builds a viewer knowing only the analyzer's *crate name*: it
/// names `<crate>::Viewer` in the generated wrapper and reaches the analyzer
/// through the associated [`Analyzer`](Self::Analyzer) type. So the model
/// records only its analyzer package — never the analyzer's concrete type path,
/// which the model couldn't name anyway (the marker's instrumentation crate does
/// not depend on the analyzer crate).
///
/// Implement it on a local unit type named `Viewer` at the analyzer crate root
/// (the path `quent-open` requires). The associated [`Analyzer`](Self::Analyzer)
/// and the model's `import_events` share an event type, so the wiring is checked
/// at compile time.
pub trait QuentViewer {
    /// The analyzer that renders this model's events.
    type Analyzer: UiAnalyzer + Send + Sync + 'static;

    /// Returns lightweight entity associations used to index one runtime context.
    ///
    /// This must avoid full event import and analyzer construction because it is called while
    /// discovering every available context.
    fn context_inventory(dir: &Path) -> ImporterResult<quent_analyzer::context::ContextInventory>;

    /// Import one context through the generated store, retaining its model
    /// events and stream availability for shared analysis.
    fn import_events(dir: &Path) -> ImporterResult<ViewerContext<Self::Analyzer>>;
}
