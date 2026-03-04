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

use crate::{cache::AnalyzerCache, error::ServerResult};

#[derive(Deserialize)]
struct ListEnginesQuery {
    #[serde(default)]
    with_metadata: bool,
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all, err)]
async fn list_engines<A>(
    State(state): State<AnalyzerCache<A>>,
    Query(query): Query<ListEnginesQuery>,
) -> ServerResult<Json<Vec<ui::Engine>>>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    if query.with_metadata {
        Ok(Json(state.list_with_metadata().await?))
    } else {
        let ids = state.list()?;
        Ok(Json(ids.into_iter().map(ui::Engine::new).collect()))
    }
}

#[tracing::instrument(skip_all, err)]
async fn engine<A>(
    State(state): State<AnalyzerCache<A>>,
    Path(engine_id): Path<Uuid>,
) -> ServerResult<Json<ui::Engine>>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    let analyzer = state.get(engine_id).await?;
    Ok(Json(analyzer.query_engine_model().engine()?.to_ui()?))
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all, err)]
async fn list_query_groups<A>(
    State(state): State<AnalyzerCache<A>>,
    Path(engine_id): Path<Uuid>,
) -> ServerResult<Json<Vec<ui::QueryGroup>>>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    let analyzer = state.get(engine_id).await?;
    Ok(Json(
        analyzer
            .query_engine_model()
            .query_groups()
            .map(QueryGroup::to_ui)
            .collect::<Vec<_>>(),
    ))
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all, err)]
async fn list_queries<A>(
    State(state): State<AnalyzerCache<A>>,
    Path((engine_id, query_group_id)): Path<(Uuid, Uuid)>,
) -> ServerResult<Json<Vec<ui::Query>>>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    let analyzer = state.get(engine_id).await?;
    let queries = analyzer
        .query_engine_model()
        .queries()
        .filter(|q| q.query_group_id == query_group_id)
        .map(|q| q.to_ui())
        .collect::<AnalyzerResult<_>>()?;
    Ok(Json(queries))
}

#[tracing::instrument(skip_all, err)]
async fn query<A>(
    State(state): State<AnalyzerCache<A>>,
    Path((engine_id, query_id)): Path<(Uuid, Uuid)>,
) -> ServerResult<Json<ui::QueryBundle<<A as UiAnalyzer>::EntityRef>>>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    let analyzer = state.get(engine_id).await?;
    let query_bundle = analyzer.query_bundle(query_id)?;
    Ok(Json(query_bundle))
}

#[tracing::instrument(skip_all, err)]
async fn single_timeline<A>(
    State(state): State<AnalyzerCache<A>>,
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
{
    let analyzer = state.get(engine_id).await?;
    Ok(Json(analyzer.single_resource_timeline(request)?))
}

#[tracing::instrument(skip_all, err)]
async fn bulk_timelines<A>(
    State(state): State<AnalyzerCache<A>>,
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
{
    let analyzer = state.get(engine_id).await?;
    Ok(Json(analyzer.bulk_resource_timeline(request)?))
}

pub fn routes<A>(cache: AnalyzerCache<A>) -> Router<()>
where
    A: UiAnalyzer + Send + Sync + 'static,
    <A as UiAnalyzer>::EntityRef: serde::Serialize,
    <A as UiAnalyzer>::TimelineGlobalParams: Send + Sync + 'static,
    <A as UiAnalyzer>::TimelineParams: Send + Sync + 'static,
    for<'de> <A as UiAnalyzer>::TimelineGlobalParams: serde::Deserialize<'de>,
    for<'de> <A as UiAnalyzer>::TimelineParams: serde::Deserialize<'de>,
{
    Router::new()
        .route("/list_engines", get(list_engines))
        .route("/engine/{engine_id}", get(engine))
        .route(
            "/engine/{engine_id}/list_query_groups",
            get(list_query_groups),
        )
        .route(
            "/engine/{engine_id}/query_group/{query_group_id}/list_queries",
            get(list_queries),
        )
        .route("/engine/{engine_id}/query/{query_id}", get(query))
        .route("/engine/{engine_id}/timeline/single", post(single_timeline))
        .route("/engine/{engine_id}/timeline/bulk", post(bulk_timelines))
        .with_state(cache)
}
