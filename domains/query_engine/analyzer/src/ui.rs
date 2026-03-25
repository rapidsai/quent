// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::AnalyzerResult;
use quent_events::Event;
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
    type Event;
    type EntityRef;
    type TimelineGlobalParams;
    type TimelineParams;

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
