// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::path::Path;

use quent_analyzer::AnalyzerResult;
use quent_events::Event;
use quent_model::exporter::ImporterResult;
use quent_query_engine_ui as ui;
use quent_ui::timeline::{
    request::{BulkChunkedTimelineRequest, BulkTimelineRequest, SingleTimelineRequest},
    response::{
        BulkChunkedTimelinesResponse, BulkTimelinesResponse, BulkTimelinesResponseEntry,
        SingleTimelineResponse,
    },
};
use uuid::Uuid;

use crate::QueryEngineModel;

/// Trait for types that can analyze query engine telemetry for the purpose of
/// visualization in a UI.
pub trait UiAnalyzer {
    type Event;
    type EntityRef;

    fn try_new(
        engine_id: Uuid,
        events: impl Iterator<Item = Event<Self::Event>>,
    ) -> AnalyzerResult<Self>
    where
        Self: Sized;

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

    /// Deliver a UI-friendly `QueryBundle` with all high-level yet
    /// non-volumous information related to this query.
    fn query_bundle(&self, query_id: Uuid) -> AnalyzerResult<ui::QueryBundle<Self::EntityRef>>;

    /// Access the underlying query engine model of this analyzer.
    fn query_engine_model(&self) -> &impl QueryEngineModel;

    /// Return a resource timeline for a single resource (or resource group).
    fn single_resource_timeline(
        &self,
        request: SingleTimelineRequest<ui::QueryFilter, ui::OperatorFilter>,
    ) -> AnalyzerResult<SingleTimelineResponse>;

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
}

/// Boxed owned stream of an analyzer's [`UiAnalyzer::Event`] from
/// [`QuentViewer::import_events`].
pub type ViewerEventStream<A> = Box<dyn Iterator<Item = Event<<A as UiAnalyzer>::Event>>>;

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

    /// Reconstruct the model's event stream from one context directory, yielding
    /// events of the [`Analyzer`](Self::Analyzer)'s event type. Wraps the model
    /// marker's generated `import_events`.
    fn import_events(dir: &Path) -> ImporterResult<ViewerEventStream<Self::Analyzer>>;
}
