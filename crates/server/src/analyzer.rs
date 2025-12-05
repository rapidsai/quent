use axum::{Json, Router, extract::Path, http::StatusCode, routing::get};
use quent_analyzer::Analyzer;
use quent_entities::{coordinator::Coordinator, engine::Engine, query::Query};
use quent_exporter_ndjson::NdjsonImporter;
use tracing::error;
use uuid::Uuid;

// TODO(johanpel): pagination
#[tracing::instrument]
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

#[tracing::instrument]
async fn engine(Path(engine_id): Path<Uuid>) -> Result<Json<Engine>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.engine().clone()))
}

// TODO(johanpel): pagination
#[tracing::instrument]
async fn list_coordinators(Path(engine_id): Path<Uuid>) -> Result<Json<Vec<Uuid>>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.coordinator_ids()))
}

#[tracing::instrument]
async fn coordinator(
    Path((engine_id, coordinator_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Option<Coordinator>>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.coordinator(coordinator_id).cloned()))
}

// TODO(johanpel): pagination
#[tracing::instrument]
async fn list_queries(
    Path((engine_id, coordinator_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<Uuid>>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.query_ids(coordinator_id)))
}

#[tracing::instrument]
async fn query(
    Path((engine_id, query_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Option<Query>>, StatusCode> {
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.query(query_id).cloned()))
}

// TODO(johanpel): add a context and really cache these analyzers :this-is-fine:
pub fn routes() -> Router<()> {
    Router::new()
        .route("/list_engines", get(list_engines))
        .route("/engine/{engine_id}", get(engine))
        .route(
            "/engine/{engine_id}/list_coordinators",
            get(list_coordinators),
        )
        .route(
            "/engine/{engine_id}/coordinator/{coordinator_id}",
            get(coordinator),
        )
        .route(
            "/engine/{engine_id}/coordinator/{coordinator_id}/list_queries",
            get(list_queries),
        )
        .route("/engine/{engine_id}/query/{query_id}", get(query))
}
