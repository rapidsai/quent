use axum::{Json, Router, extract::Path, http::StatusCode, routing::get};
use quent_analyzer::Analyzer;
use quent_entities::engine::Engine;
use quent_exporter_ndjson::NdjsonImporter;
use tracing::error;
use uuid::Uuid;

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
async fn engine(Path(engine_id): Path<String>) -> Result<Json<Engine>, StatusCode> {
    let engine_id = Uuid::parse_str(&engine_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.engine().clone()))
}

#[tracing::instrument]
async fn list_coordinators(Path(engine_id): Path<String>) -> Result<Json<Vec<Uuid>>, StatusCode> {
    let engine_id = Uuid::parse_str(&engine_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let analyzer =
        Analyzer::try_new(engine_id, importer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(analyzer.coordinator_ids()))
}

// TODO(johanpel): cache analyzers
pub fn routes() -> Router<()> {
    Router::new()
        .route("/engine/list", get(list_engines))
        .route("/engine/{engine_id}", get(engine))
        .route(
            "/engine/{engine_id}/coordinator/list",
            get(list_coordinators),
        )
}
