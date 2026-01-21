use std::num::NonZero;

use axum::{
    Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    routing::get,
};
use quent_analyzer::{Analyzer, query::QueryBundle};
use quent_entities::{
    engine::Engine,
    query_group::QueryGroup,
    timeline::{ResourceTimeline, ResourceTimelineBinned, ResourceTimelineBinnedByState},
    worker::Worker,
};
use quent_exporter_ndjson::NdjsonImporter;
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

// TODO(johanpel): pagination
#[tracing::instrument(skip_all)]
async fn list_engines() -> Result<Json<Vec<Uuid>>, StatusCode> {
    let entries = match std::fs::read_dir("data") {
        Ok(entries) => entries,
        Err(e) => {
            error!("unable read directory: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut ids = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                error!("entry error: {e}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
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
async fn engine(Path(engine_id): Path<Uuid>) -> Result<Json<Engine>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.engine().clone()))
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all)]
async fn list_workers(Path(engine_id): Path<Uuid>) -> Result<Json<Vec<Uuid>>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.worker_ids()))
}

#[tracing::instrument(skip_all)]
async fn worker(
    Path((engine_id, worker_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Option<Worker>>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.worker(worker_id).cloned()))
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all)]
async fn list_query_groups(Path(engine_id): Path<Uuid>) -> Result<Json<Vec<Uuid>>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.query_group_ids()))
}

#[tracing::instrument(skip_all)]
async fn query_group(
    Path((engine_id, query_group_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Option<QueryGroup>>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.query_group(query_group_id).cloned()))
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all)]
async fn list_queries(
    Path((engine_id, query_group_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<Uuid>>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.query_ids(query_group_id)))
}

#[tracing::instrument(skip_all)]
async fn query(
    Path((engine_id, query_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<QueryBundle>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let query_bundle = analyzer.query_bundle(query_id)?;
    Ok(Json(query_bundle))
}

#[tracing::instrument(skip_all)]
async fn resource_use_timeline(
    Path((engine_id, resource_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ResourceTimeline>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let timeline = analyzer.resource_usage_spans(resource_id)?;
    Ok(Json(timeline))
}

#[derive(Deserialize)]
struct ResourceUsageAggregatedQuery {
    num_bins: u64,
}

#[tracing::instrument(skip_all)]
async fn resource_use_timeline_aggregated(
    Path((engine_id, resource_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ResourceUsageAggregatedQuery>,
) -> Result<Json<ResourceTimelineBinned>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let num_bins =
        NonZero::try_from(query.num_bins).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let config = quent_time::bin::BinnedSpan::try_new(analyzer.timestamp_span(), num_bins)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let timeline = analyzer.resource_usage_aggregated(resource_id, config)?;
    Ok(Json(timeline))
}

#[derive(Deserialize)]
struct ResourceUsageStatesAggregatedQuery {
    fsm_type_name: String,
    num_bins: u64,
}

#[tracing::instrument(skip_all)]
async fn resource_use_timeline_state_aggregated(
    Path((engine_id, resource_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ResourceUsageStatesAggregatedQuery>,
) -> Result<Json<ResourceTimelineBinnedByState>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let num_bins =
        NonZero::try_from(query.num_bins).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let config = quent_time::bin::BinnedSpan::try_new(analyzer.timestamp_span(), num_bins)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let timeline =
        analyzer.resource_usage_states_aggregated(resource_id, config, query.fsm_type_name)?;
    Ok(Json(timeline))
}

// TODO(johanpel): add a context and really cache these analyzers :this-is-fine:
pub fn routes() -> Router<()> {
    Router::new()
        .route("/list_engines", get(list_engines))
        .route("/engine/{engine_id}", get(engine))
        .route("/engine/{engine_id}/list_workers", get(list_workers))
        .route("/engine/{engine_id}/worker/{worker_id}", get(worker))
        .route(
            "/engine/{engine_id}/list_query_groups",
            get(list_query_groups),
        )
        .route(
            "/engine/{engine_id}/query_group/{query_group_id}",
            get(query_group),
        )
        .route(
            "/engine/{engine_id}/query_group/{query_group_id}/list_queries",
            get(list_queries),
        )
        .route("/engine/{engine_id}/query/{query_id}", get(query))
        .route(
            "/engine/{engine_id}/timeline/resource/{resource_id}/use/all",
            get(resource_use_timeline),
        )
        .route(
            "/engine/{engine_id}/timeline/resource/{resource_id}/agg/all",
            get(resource_use_timeline_aggregated),
        )
        .route(
            "/engine/{engine_id}/timeline/resource/{resource_id}/agg/fsm",
            get(resource_use_timeline_state_aggregated),
        )
}
