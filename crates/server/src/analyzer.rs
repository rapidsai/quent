use std::num::NonZero;

use axum::{
    Json, Router,
    extract::{Path, Query},
    routing::get,
};
use quent_analyzer::{Analyzer, query_bundle::QueryBundle};
use quent_entities::{
    Span,
    engine::Engine,
    query_group::QueryGroup,
    timeline::{ResourceTimelineBinned, ResourceTimelineBinnedByState},
    worker::Worker,
};
use quent_exporter_ndjson::NdjsonImporter;
use quent_time::{SpanNanoSec, TimeUnixNanoSec, bin::BinnedSpan};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};

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
async fn engine(Path(engine_id): Path<Uuid>) -> ServerResult<Json<Engine>> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))?;
    let analyzer = Analyzer::try_new(engine_id, importer)?;
    Ok(Json(analyzer.engine().clone()))
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all)]
async fn list_workers(Path(engine_id): Path<Uuid>) -> ServerResult<Json<Vec<Uuid>>> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))?;
    let analyzer = Analyzer::try_new(engine_id, importer)?;
    Ok(Json(analyzer.worker_ids()))
}

#[tracing::instrument(skip_all)]
async fn worker(
    Path((engine_id, worker_id)): Path<(Uuid, Uuid)>,
) -> ServerResult<Json<Option<Worker>>> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))?;
    let analyzer = Analyzer::try_new(engine_id, importer)?;
    Ok(Json(analyzer.worker(worker_id).cloned()))
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all)]
async fn list_query_groups(Path(engine_id): Path<Uuid>) -> ServerResult<Json<Vec<Uuid>>> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))?;
    let analyzer = Analyzer::try_new(engine_id, importer)?;
    Ok(Json(analyzer.query_group_ids()))
}

#[tracing::instrument(skip_all)]
async fn query_group(
    Path((engine_id, query_group_id)): Path<(Uuid, Uuid)>,
) -> ServerResult<Json<Option<QueryGroup>>> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))?;
    let analyzer = Analyzer::try_new(engine_id, importer)?;
    Ok(Json(analyzer.query_group(query_group_id).cloned()))
}

// TODO(johanpel): pagination
#[tracing::instrument(skip_all)]
async fn list_queries(
    Path((engine_id, query_group_id)): Path<(Uuid, Uuid)>,
) -> ServerResult<Json<Vec<Uuid>>> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))?;
    let analyzer = Analyzer::try_new(engine_id, importer)?;
    Ok(Json(analyzer.query_ids(query_group_id)))
}

#[tracing::instrument(skip_all)]
async fn query(Path((engine_id, query_id)): Path<(Uuid, Uuid)>) -> ServerResult<Json<QueryBundle>> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))?;
    let analyzer = Analyzer::try_new(engine_id, importer)?;
    let query_bundle = analyzer.query_bundle(query_id)?;
    Ok(Json(query_bundle))
}

#[derive(Deserialize)]
struct ResourceUsageAggregatedQuery {
    /// The number of bins.
    ///
    /// u16::MAX is large enough when bins are plotted as single pixel wide
    /// bars, even for insane screen resolutions.
    num_bins: u16,
    /// Start time in seconds.
    start: f64,
    /// End time in seconds.
    end: f64,
}

impl ResourceUsageAggregatedQuery {
    fn try_make_binned_span(&self, epoch: TimeUnixNanoSec) -> ServerResult<BinnedSpan> {
        let start = epoch + (self.start * 1e9) as u64;
        let end = epoch + (self.end * 1e9) as u64;
        let span = SpanNanoSec::try_new(start, end)
            .map_err(|e| ServerError::UrlQueryParams(format!("invalid time span: {e}")))?;
        let num_bins = NonZero::<u64>::try_from(self.num_bins as u64)
            .map_err(|e| ServerError::UrlQueryParams(format!("number of bins must be > 0: {e}")))?;
        let binned_span = BinnedSpan::try_new(span, num_bins).map_err(|e| {
            ServerError::UrlQueryParams(format!("unable to construct binned span: {e}"))
        })?;
        Ok(binned_span)
    }
}

#[tracing::instrument(skip_all)]
async fn resource_use_timeline_aggregated(
    Path((engine_id, query_id, resource_id)): Path<(Uuid, Uuid, Uuid)>,
    Query(query): Query<ResourceUsageAggregatedQuery>,
) -> ServerResult<Json<ResourceTimelineBinned>> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))?;
    let analyzer = Analyzer::try_new(engine_id, importer)?;
    let query_entity = analyzer.entities().query(query_id)?;
    let config = query.try_make_binned_span(query_entity.span()?.start())?;
    let timeline = analyzer.resource_usage_aggregated(resource_id, config)?;
    Ok(Json(timeline))
}

#[derive(Deserialize)]
struct ResourceUsageStatesAggregatedQuery {
    fsm_type_name: String,

    // TODO(johanpel): figure out why
    // https://codeberg.org/jplatte/serde_html_form used by Axum doesn't seem to
    // properly support serde(flatten), so for now repeat:
    num_bins: u16,
    start: f64,
    end: f64,
}
impl ResourceUsageStatesAggregatedQuery {
    fn try_make_binned_span(&self, epoch: TimeUnixNanoSec) -> ServerResult<BinnedSpan> {
        ResourceUsageAggregatedQuery {
            num_bins: self.num_bins,
            start: self.start,
            end: self.end,
        }
        .try_make_binned_span(epoch)
    }
}

#[tracing::instrument(skip_all)]
async fn resource_use_timeline_state_aggregated(
    Path((engine_id, query_id, resource_id)): Path<(Uuid, Uuid, Uuid)>,
    Query(query): Query<ResourceUsageStatesAggregatedQuery>,
) -> ServerResult<Json<ResourceTimelineBinnedByState>> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))?;
    let analyzer = Analyzer::try_new(engine_id, importer)?;
    let query_entity = analyzer.entities().query(query_id)?;
    let config = query.try_make_binned_span(query_entity.span()?.start())?;
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
            "/engine/{engine_id}/query/{query_id}/resource/{resource_id}/timeline/agg/all",
            get(resource_use_timeline_aggregated),
        )
        .route(
            "/engine/{engine_id}/query/{query_id}/resource/{resource_id}/timeline/agg/fsm",
            get(resource_use_timeline_state_aggregated),
        )
}
