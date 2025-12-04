use std::net::ToSocketAddrs;

use axum::{Json, Router, extract::Path, http::StatusCode, routing::get};
use quent_analyzer::Analyzer;
use quent_collector::{proto::collector_server::CollectorServer, server::CollectorService};
use quent_entities::engine::Engine;
use quent_exporter_ndjson::NdjsonImporter;
use tokio::net::TcpListener;
use tonic::transport::Server as GrpcServer;
use tracing::info;
use uuid::Uuid;

const QUENT_ANALYZER_PORT_DEFAULT: u16 = 8080;

fn initialize_tracing() {
    use tracing_subscriber::{
        layer::SubscriberExt,
        registry,
        util::SubscriberInitExt,
        {
            EnvFilter,
            fmt::{self, format::FmtSpan},
        },
    };
    registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("debug,h2=off,tonic=off")),
        )
        .with(
            fmt::layer()
                .with_target(true)
                .with_span_events(FmtSpan::ENTER)
                .with_writer(std::io::stderr),
        )
        .init();
}

#[tracing::instrument]
async fn list_engines() -> Result<Json<Vec<Uuid>>, StatusCode> {
    let entries = std::fs::read_dir("data").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();

    // Collector service
    let collector_addr = format!("[::]:{}", quent_collector::QUENT_COLLECTOR_PORT_DEFAULT)
        .to_socket_addrs()?
        .next()
        .unwrap();
    let collector = CollectorService::default();
    let collector_service = async {
        GrpcServer::builder()
            .add_service(CollectorServer::new(collector))
            .serve(collector_addr)
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
    };
    info!("collector listening on {collector_addr}");

    // Analyzer service
    let analyzer_addr = format!("[::]:{QUENT_ANALYZER_PORT_DEFAULT}")
        .to_socket_addrs()?
        .next()
        .unwrap();
    let analyzer_listener = TcpListener::bind(analyzer_addr).await?;

    let analyzer_routes = Router::new()
        .route("/{engine_id}/engine", get(engine))
        .route("/list_engines", get(list_engines));
    let analyzer_app = Router::new().nest("/analyzer", analyzer_routes);
    let analyzer_service = async {
        axum::serve(analyzer_listener, analyzer_app.into_make_service()).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    info!("analyzer listening on {analyzer_addr}");

    info!("send SIGINT (e.g. ctrl+c) to exit");

    tokio::try_join!(collector_service, analyzer_service)?;

    Ok(())
}
