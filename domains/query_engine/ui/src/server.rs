// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed contract implemented by query-engine API servers.

use quent_ui::{
    entities::{request::EntityListRequest, response::EntityListResponse},
    timeline::{
        categorical::CategoricalTimelineRequest,
        request::{BulkTimelineRequest, SingleTimelineRequest},
        response::{BulkTimelinesResponse, SingleTimelineResponse},
    },
};
use uuid::Uuid;

use crate::{
    DataFlowTimelineBinned, Engine, EngineContexts, OperatorFilter, Query, QueryBundle,
    QueryFilter, QueryGroup,
};

/// Query-engine service operations shared by native and browser-hosted servers.
#[async_trait::async_trait]
pub trait ServerContract: Sync {
    /// Transport-specific failure returned by the server.
    type Error;
    /// Application-specific entity reference embedded in query bundles.
    type EntityRef;

    /// List the available engines, optionally including their metadata.
    async fn list_engines(&self, with_metadata: bool) -> Result<Vec<Engine>, Self::Error>;
    /// Return one engine and its metadata.
    async fn engine(&self, engine_id: Uuid) -> Result<Engine, Self::Error>;
    /// List the telemetry contexts contributing to an engine.
    async fn engine_contexts(&self, engine_id: Uuid) -> Result<EngineContexts, Self::Error>;
    /// List the query groups belonging to an engine.
    async fn query_groups(&self, engine_id: Uuid) -> Result<Vec<QueryGroup>, Self::Error>;
    /// List the queries belonging to a query group.
    async fn queries(
        &self,
        engine_id: Uuid,
        query_group_id: Uuid,
    ) -> Result<Vec<Query>, Self::Error>;
    /// Return the entities and execution plan for one query.
    async fn query(
        &self,
        engine_id: Uuid,
        query_id: Uuid,
    ) -> Result<QueryBundle<Self::EntityRef>, Self::Error>;
    /// Build a timeline for one resource or resource group.
    async fn single_timeline(
        &self,
        engine_id: Uuid,
        request: SingleTimelineRequest<QueryFilter, OperatorFilter>,
    ) -> Result<SingleTimelineResponse, Self::Error>;
    /// Build timelines for multiple resources or resource groups.
    async fn bulk_timelines(
        &self,
        engine_id: Uuid,
        request: BulkTimelineRequest<QueryFilter, OperatorFilter>,
    ) -> Result<BulkTimelinesResponse, Self::Error>;
    /// Build the categorical data-flow timeline for a query.
    async fn data_flow_timeline(
        &self,
        engine_id: Uuid,
        request: CategoricalTimelineRequest<QueryFilter>,
    ) -> Result<DataFlowTimelineBinned, Self::Error>;
    /// List entities matching a resource, time-window, and application filter.
    async fn entities(
        &self,
        engine_id: Uuid,
        request: EntityListRequest<QueryFilter, OperatorFilter>,
    ) -> Result<EntityListResponse, Self::Error>;
}
