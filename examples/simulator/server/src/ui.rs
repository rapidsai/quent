use std::num::NonZero;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};

use quent_analyzer::{AnalyzerError, AnalyzerResult};
use quent_query_engine_ui as ui;
use quent_simulator_ui::{
    QueryBundle,
    timeline::{
        BulkTimelinesRequest, BulkTimelinesResponse, ResourceTimelineUrlQueryParams,
        TimelineResponse,
    },
};
use quent_time::{SpanNanoSec, TimeUnixNanoSec, bin::BinnedSpan};
use tracing::error;
use uuid::Uuid;

use crate::{
    cache::AnalyzerCache,
    error::{ServerError, ServerResult},
};

// TODO(johanpel): pagination
#[tracing::instrument(skip_all)]
async fn list_engines() -> ServerResult<Json<Vec<Uuid>>> {
    let entries = match std::fs::read_dir("data") {
        Ok(entries) => entries,
        Err(e) => {
            error!("unable read directory: {e}");
            Err(e)?
        }
    };

    let mut ids = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                error!("entry error: {e}");
                Err(e)?
            }
        };
        let path = entry.path();

        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("ndjson") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            match Uuid::parse_str(stem) {
                Ok(uuid) => ids.push(uuid),
                Err(_) => {
                    continue;
                }
            }
        }
    }

    Ok(Json(ids))
}

#[tracing::instrument(skip_all)]
async fn engine(
    State(state): State<AnalyzerCache>,
    Path(engine_id): Path<Uuid>,
) -> ServerResult<Json<ui::Engine>> {
    let analyzer = state.get(engine_id).await?;
    Ok(Json((&analyzer.model.query_engine.engine).try_into()?))
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all)]
async fn list_query_groups(
    State(state): State<AnalyzerCache>,
    Path(engine_id): Path<Uuid>,
) -> ServerResult<Json<Vec<ui::QueryGroup>>> {
    let analyzer = state.get(engine_id).await?;
    Ok(Json(
        analyzer
            .model
            .query_engine
            .query_groups
            .values()
            .map(Into::into)
            .collect::<Vec<_>>(),
    ))
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all)]
async fn list_queries(
    State(state): State<AnalyzerCache>,
    Path((engine_id, query_group_id)): Path<(Uuid, Uuid)>,
) -> ServerResult<Json<Vec<ui::Query>>> {
    let analyzer = state.get(engine_id).await?;
    let queries = analyzer
        .model
        .query_engine
        .queries
        .values()
        .filter(|q| q.query_group_id.is_some_and(|id| id == query_group_id))
        .map(TryInto::try_into)
        .collect::<AnalyzerResult<_>>()?;
    Ok(Json(queries))
}

#[tracing::instrument(skip_all)]
async fn query(
    State(state): State<AnalyzerCache>,
    Path((engine_id, query_id)): Path<(Uuid, Uuid)>,
) -> ServerResult<Json<QueryBundle>> {
    let analyzer = state.get(engine_id).await?;
    let query_bundle = analyzer.query_bundle(query_id)?;
    Ok(Json(query_bundle))
}

fn try_make_binned_span(
    params: &ResourceTimelineUrlQueryParams,
    epoch: TimeUnixNanoSec,
) -> ServerResult<BinnedSpan> {
    let start = epoch + (params.start * 1e9) as u64;
    let end = epoch + (params.end * 1e9) as u64;
    let span = SpanNanoSec::try_new(start, end)
        .map_err(|e| ServerError::UrlQueryParams(format!("invalid time span: {e}")))?;
    let num_bins = NonZero::<u64>::try_from(params.num_bins as u64)
        .map_err(|e| ServerError::UrlQueryParams(format!("number of bins must be > 0: {e}")))?;
    let binned_span = BinnedSpan::try_new(span, num_bins).map_err(|e| {
        ServerError::UrlQueryParams(format!("unable to construct binned span: {e}"))
    })?;
    Ok(binned_span)
}

#[tracing::instrument(skip_all)]
async fn resource_timeline(
    State(state): State<AnalyzerCache>,
    Path((engine_id, query_id, resource_id)): Path<(Uuid, Uuid, Uuid)>,
    Query(url_query_params): Query<ResourceTimelineUrlQueryParams>,
) -> ServerResult<Json<TimelineResponse>> {
    let analyzer = state.get(engine_id).await?;
    let epoch = analyzer.model.query_engine.query_epoch(query_id)?;
    let config = try_make_binned_span(&url_query_params, epoch)?;
    Ok(Json(analyzer.resource_timeline(
        resource_id,
        url_query_params.fsm_type_name,
        url_query_params.operator_id,
        config,
        epoch,
    )?))
}

#[tracing::instrument(skip_all)]
async fn resource_group_timeline(
    State(state): State<AnalyzerCache>,
    Path((engine_id, query_id, resource_group_id)): Path<(Uuid, Uuid, Uuid)>,
    Query(url_query_params): Query<ResourceTimelineUrlQueryParams>,
) -> ServerResult<Json<TimelineResponse>> {
    let analyzer = state.get(engine_id).await?;
    let epoch = analyzer.model.query_engine.query_epoch(query_id)?;
    let config = try_make_binned_span(&url_query_params, epoch)?;
    if let Some(resource_type_name) = url_query_params.resource_type_name {
        Ok(Json(analyzer.resource_group_timeline(
            resource_group_id,
            &resource_type_name,
            url_query_params.fsm_type_name,
            url_query_params.operator_id,
            config,
            epoch,
        )?))
    } else {
        Err(AnalyzerError::InvalidArgument(
            "resource type name is not set".to_string(),
        ))?
    }
}

#[tracing::instrument(skip_all)]
async fn bulk_timelines(
    State(state): State<AnalyzerCache>,
    Path((engine_id, query_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<BulkTimelinesRequest>,
) -> ServerResult<Json<BulkTimelinesResponse>> {
    let analyzer = state.get(engine_id).await?;
    let epoch = analyzer.model.query_engine.query_epoch(query_id)?;
    Ok(Json(analyzer.bulk_timelines(query_id, request, epoch)?))
}

// TODO(johanpel): add a context and really cache these analyzers :this-is-fine:
pub fn routes(cache: AnalyzerCache) -> Router<()> {
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
        .route(
            "/engine/{engine_id}/query/{query_id}/resource/{resource_id}/timeline",
            get(resource_timeline),
        )
        .route(
            "/engine/{engine_id}/query/{query_id}/resource_group/{resource_group_id}/timeline",
            get(resource_group_timeline),
        )
        .route(
            "/engine/{engine_id}/query/{query_id}/bulk_timelines",
            post(bulk_timelines),
        )
        .with_state(cache)
}
