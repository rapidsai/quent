// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_query_engine_analyzer::{EngineEntity, QueryEngineModel, QueryEntity, ui::UiAnalyzer};
use quent_query_engine_ui::{self as ui, ServerContract};
use quent_ui::{
    entities::request::EntityListRequest,
    timeline::{
        categorical::CategoricalTimelineRequest,
        request::{BulkTimelineRequest, SingleTimelineRequest},
        response::{BulkTimelinesResponse, SingleTimelineResponse},
    },
};
use uuid::Uuid;

use crate::{
    analyzer_cache::AnalyzerCache,
    error::{ServerError, ServerResult},
    timeline_cache::TimelineCache,
};

/// Combined service state for axum handlers.
pub struct ServiceState<A>
where
    A: UiAnalyzer,
{
    pub analyzers: AnalyzerCache<A>,
    pub timelines: TimelineCache,
}

impl<A> ServiceState<A>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    pub(crate) async fn logs(
        &self,
        engine_id: Uuid,
        request: quent_log::ui::LogListRequest,
    ) -> ServerResult<quent_log::ui::LogListResponse> {
        let analyzer = self.analyzers.get(engine_id).await?;
        tokio::task::spawn_blocking(move || analyzer.list_logs(request))
            .await?
            .map_err(|error| match error {
                quent_analyzer::AnalyzerError::InvalidArgument(message) => {
                    ServerError::BadRequest(message)
                }
                error => ServerError::Analyzer(error),
            })
    }
}

impl<A> Clone for ServiceState<A>
where
    A: UiAnalyzer,
{
    fn clone(&self) -> Self {
        Self {
            analyzers: self.analyzers.clone(),
            timelines: self.timelines.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<A> ServerContract for ServiceState<A>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    type Error = ServerError;

    async fn list_engines(&self, with_metadata: bool) -> ServerResult<ui::EngineListResponse> {
        let items = if with_metadata {
            self.analyzers.list_with_metadata().await?
        } else {
            self.analyzers
                .list()?
                .into_iter()
                .map(ui::Engine::new)
                .collect()
        };
        Ok(ui::EngineListResponse {
            items,
            initial_group_by_attribute: A::engine_initial_group_by_attribute(),
        })
    }

    async fn engine(&self, engine_id: Uuid) -> ServerResult<ui::Engine> {
        let analyzer = self.analyzers.get(engine_id).await?;
        Ok(analyzer.query_engine_model().engine()?.to_ui()?)
    }

    async fn engine_contexts(&self, engine_id: Uuid) -> ServerResult<ui::EngineContexts> {
        self.analyzers.contexts(engine_id).await.map_err(|error| {
            tracing::error!(%error, %engine_id, "engine context inventory failed");
            ServerError::Cache("engine context inventory could not be loaded".to_owned())
        })
    }

    async fn queries(&self, engine_id: Uuid) -> ServerResult<ui::QueryListResponse> {
        let analyzer = self.analyzers.get(engine_id).await?;
        let items = analyzer
            .query_engine_model()
            .queries()
            .map(QueryEntity::to_ui)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ui::QueryListResponse {
            items,
            initial_group_by_attribute: analyzer.query_initial_group_by_attribute(),
        })
    }

    async fn query(&self, engine_id: Uuid, query_id: Uuid) -> ServerResult<ui::QueryBundle> {
        let analyzer = self.analyzers.get(engine_id).await?;
        analyzer.query_bundle(query_id).map_err(Into::into)
    }

    async fn single_timeline(
        &self,
        engine_id: Uuid,
        request: SingleTimelineRequest<ui::QueryFilter, ui::OperatorFilter>,
    ) -> ServerResult<SingleTimelineResponse> {
        let analyzer = self.analyzers.get(engine_id).await?;
        self.timelines
            .cached_single_timeline(analyzer, engine_id, request)
            .await
    }

    async fn bulk_timelines(
        &self,
        engine_id: Uuid,
        request: BulkTimelineRequest<ui::QueryFilter, ui::OperatorFilter>,
    ) -> ServerResult<BulkTimelinesResponse> {
        let analyzer = self.analyzers.get(engine_id).await?;
        self.timelines
            .cached_bulk_timeline(analyzer, engine_id, request)
            .await
    }

    async fn data_flow_timeline(
        &self,
        engine_id: Uuid,
        request: CategoricalTimelineRequest<ui::QueryFilter>,
    ) -> ServerResult<ui::DataFlowTimelineBinned> {
        let analyzer = self.analyzers.get(engine_id).await?;
        Ok(tokio::task::spawn_blocking(move || analyzer.data_flow_timeline(request)).await??)
    }

    async fn entities(
        &self,
        engine_id: Uuid,
        request: EntityListRequest<ui::QueryFilter, ui::OperatorFilter>,
    ) -> ServerResult<ui::EntityListResponse> {
        let analyzer = self.analyzers.get(engine_id).await?;
        analyzer.list_entities(request).map_err(Into::into)
    }
}
