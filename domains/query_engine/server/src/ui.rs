// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};

use quent_analyzer::AnalyzerResult;
use quent_query_engine_analyzer::{QueryEngineModel, query_group::QueryGroup, ui::UiAnalyzer};
use quent_query_engine_ui as ui;
use quent_ui::timeline::{
    request::{BulkTimelineRequest, SingleTimelineRequest},
    response::{BulkTimelinesResponse, SingleTimelineResponse},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{error::ServerResult, state::ServiceState};

#[cfg(feature = "ui")]
pub(crate) mod embedded {
    use axum::{
        http::{StatusCode, header},
        response::IntoResponse,
    };
    use rust_embed::Embed;

    #[derive(Embed)]
    #[folder = "../../../ui/dist/"]
    struct UiAssets;

    pub async fn serve(uri: axum::http::Uri) -> impl IntoResponse {
        let path = uri.path().trim_start_matches('/');
        let file = UiAssets::get(path).or_else(|| UiAssets::get("index.html"));
        match file {
            Some(content) => {
                let mime = mime_guess::from_path(if path.is_empty() || !path.contains('.') {
                    "index.html"
                } else {
                    path
                })
                .first_or_octet_stream();
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                    content.data.into_owned(),
                )
                    .into_response()
            }
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }
}

#[derive(Deserialize)]
struct ListEnginesQuery {
    #[serde(default)]
    with_metadata: bool,
}

// TODO(johanpel): pagination
/// List all available engines.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/engines",
    tag = "engines",
    responses(
        (status = 200, description = "List of engines, optionally with metadata via ?with_metadata=true", body = [Object])
    )
))]
#[tracing::instrument(skip_all, err)]
async fn list_engines<A>(
    State(state): State<ServiceState<A>>,
    Query(query): Query<ListEnginesQuery>,
) -> ServerResult<Json<Vec<ui::Engine>>>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    if query.with_metadata {
        Ok(Json(state.analyzers.list_with_metadata().await?))
    } else {
        let ids = state.analyzers.list()?;
        Ok(Json(ids.into_iter().map(ui::Engine::new).collect()))
    }
}

/// Get details for a specific engine.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/engines/{engine_id}",
    tag = "engines",
    params(
        ("engine_id" = Uuid, Path, description = "The engine ID")
    ),
    responses(
        (status = 200, description = "Engine details", body = Object)
    )
))]
#[tracing::instrument(skip_all, err)]
async fn engine<A>(
    State(state): State<ServiceState<A>>,
    Path(engine_id): Path<Uuid>,
) -> ServerResult<Json<ui::Engine>>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    let analyzer = state.analyzers.get(engine_id).await?;
    Ok(Json(analyzer.query_engine_model().engine()?.to_ui()?))
}

// TODO(johanpel): pagination
/// List all query groups for a given engine.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/engines/{engine_id}/query-groups",
    tag = "engines",
    params(
        ("engine_id" = Uuid, Path, description = "The engine ID")
    ),
    responses(
        (status = 200, description = "List of query groups for the engine", body = [Object])
    )
))]
#[tracing::instrument(skip_all, err)]
async fn list_query_groups<A>(
    State(state): State<ServiceState<A>>,
    Path(engine_id): Path<Uuid>,
) -> ServerResult<Json<Vec<ui::QueryGroup>>>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    let analyzer = state.analyzers.get(engine_id).await?;
    Ok(Json(
        analyzer
            .query_engine_model()
            .query_groups()
            .map(QueryGroup::to_ui)
            .collect::<Vec<_>>(),
    ))
}

// TODO(johanpel): pagination
/// List all queries for a specific query group.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/engines/{engine_id}/query_group/{query_group_id}/queries",
    tag = "engines",
    params(
        ("engine_id" = Uuid, Path, description = "The engine ID"),
        ("query_group_id" = Uuid, Path, description = "The query group ID")
    ),
    responses(
        (status = 200, description = "List of queries in the query group", body = [Object])
    )
))]
#[tracing::instrument(skip_all, err)]
async fn list_queries<A>(
    State(state): State<ServiceState<A>>,
    Path((engine_id, query_group_id)): Path<(Uuid, Uuid)>,
) -> ServerResult<Json<Vec<ui::Query>>>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    let analyzer = state.analyzers.get(engine_id).await?;
    let queries = analyzer
        .query_engine_model()
        .queries()
        .filter(|q| q.query_group_id() == Some(query_group_id))
        .map(|q| q.to_ui())
        .collect::<AnalyzerResult<_>>()?;
    Ok(Json(queries))
}

/// Fetch the query plan for a given query.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/engines/{engine_id}/query/{query_id}",
    tag = "engines",
    params(
        ("engine_id" = Uuid, Path, description = "The engine ID"),
        ("query_id" = Uuid, Path, description = "The query ID")
    ),
    responses(
        (status = 200, description = "Query bundle with entities, plan tree, and resource tree", body = Object)
    )
))]
#[tracing::instrument(skip_all, err)]
async fn query<A>(
    State(state): State<ServiceState<A>>,
    Path((engine_id, query_id)): Path<(Uuid, Uuid)>,
) -> ServerResult<Json<ui::QueryBundle<<A as UiAnalyzer>::EntityRef>>>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    let analyzer = state.analyzers.get(engine_id).await?;
    let query_bundle = analyzer.query_bundle(query_id)?;
    Ok(Json(query_bundle))
}

/// Fetch a single resource or resource-group timeline.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/engines/{engine_id}/timeline/single",
    tag = "timelines",
    params(
        ("engine_id" = Uuid, Path, description = "The engine ID")
    ),
    request_body = Object,
    responses(
        (status = 200, description = "Single resource timeline with binned data", body = Object)
    )
))]
#[tracing::instrument(skip_all, err)]
async fn single_timeline<A>(
    State(state): State<ServiceState<A>>,
    Path(engine_id): Path<Uuid>,
    Json(request): Json<
        SingleTimelineRequest<
            <A as UiAnalyzer>::TimelineGlobalParams,
            <A as UiAnalyzer>::TimelineParams,
        >,
    >,
) -> ServerResult<Json<SingleTimelineResponse>>
where
    A: UiAnalyzer + Send + Sync + 'static,
    <A as UiAnalyzer>::TimelineGlobalParams: serde::Serialize + Clone,
    <A as UiAnalyzer>::TimelineParams: serde::Serialize + Clone,
{
    let analyzer = state.analyzers.get(engine_id).await?;
    Ok(Json(
        state
            .timelines
            .cached_single_timeline(&*analyzer, engine_id, request)
            .await?,
    ))
}

/// Fetch multiple resource/resource-group timelines in one request.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/engines/{engine_id}/timeline/bulk",
    tag = "timelines",
    params(
        ("engine_id" = Uuid, Path, description = "The engine ID")
    ),
    request_body = Object,
    responses(
        (status = 200, description = "Bulk resource timelines", body = Object)
    )
))]
#[tracing::instrument(skip_all, err)]
async fn bulk_timelines<A>(
    State(state): State<ServiceState<A>>,
    Path(engine_id): Path<Uuid>,
    Json(request): Json<
        BulkTimelineRequest<
            <A as UiAnalyzer>::TimelineGlobalParams,
            <A as UiAnalyzer>::TimelineParams,
        >,
    >,
) -> ServerResult<Json<BulkTimelinesResponse>>
where
    A: UiAnalyzer + Send + Sync + 'static,
    <A as UiAnalyzer>::TimelineGlobalParams: Send + 'static,
    <A as UiAnalyzer>::TimelineParams: Send + 'static,
{
    let analyzer = state.analyzers.get(engine_id).await?;
    let response =
        tokio::task::spawn_blocking(move || analyzer.bulk_resource_timeline(request)).await??;
    Ok(Json(response))
}

#[cfg(feature = "swagger")]
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        list_engines,
        engine,
        list_query_groups,
        list_queries,
        query,
        single_timeline,
        bulk_timelines,
    ),
    tags(
        (name = "engines", description = "Engine, query group, and query management"),
        (name = "timelines", description = "Resource timeline data"),
    )
)]
pub(crate) struct ApiDoc;

pub fn routes<A>(state: ServiceState<A>) -> Router<()>
where
    A: UiAnalyzer + Send + Sync + 'static,
    <A as UiAnalyzer>::EntityRef: serde::Serialize,
    <A as UiAnalyzer>::TimelineGlobalParams: Send + Sync + Clone + serde::Serialize + 'static,
    <A as UiAnalyzer>::TimelineParams: Send + Sync + Clone + serde::Serialize + 'static,
    for<'de> <A as UiAnalyzer>::TimelineGlobalParams: serde::Deserialize<'de>,
    for<'de> <A as UiAnalyzer>::TimelineParams: serde::Deserialize<'de>,
{
    Router::new()
        .route("/", get(list_engines))
        .route("/{engine_id}", get(engine))
        .route("/{engine_id}/query-groups", get(list_query_groups))
        .route(
            "/{engine_id}/query_group/{query_group_id}/queries",
            get(list_queries),
        )
        .route("/{engine_id}/query/{query_id}", get(query))
        .route("/{engine_id}/timeline/single", post(single_timeline))
        .route("/{engine_id}/timeline/bulk", post(bulk_timelines))
        .with_state(state)
}
