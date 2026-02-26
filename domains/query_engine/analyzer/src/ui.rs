use quent_analyzer::AnalyzerResult;
use quent_query_engine_ui as ui;
use quent_ui::timeline::{
    request::{BulkTimelineRequest, SingleTimelineRequest},
    response::{BulkTimelinesResponse, SingleTimelineResponse},
};
use uuid::Uuid;

use crate::QueryEngineModel;

/// Trait for types that can analyze query engine telemetry for the purpose of
/// visualization in a UI.
pub trait UiAnalyzer {
    type EntityRef;
    type TimelineGlobalParams;
    type TimelineParams;

    fn try_new(engine_id: Uuid) -> AnalyzerResult<Self>
    where
        Self: Sized;

    /// Deliver a UI-friendly [`QueryBundle`] with all high-level yet
    /// non-volumous information related to this query.
    fn query_bundle(&self, query_id: Uuid) -> AnalyzerResult<ui::QueryBundle<Self::EntityRef>>;

    /// Access the underlying query engine model of this analyzer.
    fn query_engine_model(&self) -> &impl QueryEngineModel;

    /// Return a resource timeline for a single resource (or resource group).
    ///
    /// The type F may contain additional application-specific entity filters.
    fn single_resource_timeline(
        &self,
        request: SingleTimelineRequest<Self::TimelineGlobalParams, Self::TimelineParams>,
    ) -> AnalyzerResult<SingleTimelineResponse>;

    /// Return a set of resource timelines in bulk.
    ///
    /// The type F may contain additional application-specific entity filters.
    fn bulk_resource_timeline(
        &self,
        request: BulkTimelineRequest<Self::TimelineGlobalParams, Self::TimelineParams>,
    ) -> AnalyzerResult<BulkTimelinesResponse>;
}
