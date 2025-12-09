use std::net::ToSocketAddrs;

use axum::Router;
use quent_collector::{proto::collector_server::CollectorServer, server::CollectorService};
use tokio::net::TcpListener;
use tonic::transport::Server as GrpcServer;
use tower_http::cors::CorsLayer;
use tracing::info;

mod defaults {
    pub(crate) const QUENT_ANALYZER_PORT: u16 = 8080;
}

mod analyzer;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();

    // Spawn gRPC services
    let grpc_address = format!("[::]:{}", quent_collector::default::QUENT_COLLECTOR_PORT)
        .to_socket_addrs()?
        .next()
        .unwrap();

    let collector = CollectorService::default();

    let grpc_server = async {
        GrpcServer::builder()
            .add_service(CollectorServer::new(collector))
            .serve(grpc_address)
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
    };
    info!("gRPC server listening on {grpc_address}");

    // HTTP services
    let http_addr = format!("[::]:{}", defaults::QUENT_ANALYZER_PORT)
        .to_socket_addrs()?
        .next()
        .unwrap();

    let cors = CorsLayer::new()
        .allow_origin(
            "http://localhost:5173"
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let http_routes = Router::new()
        .nest("/analyzer", analyzer::routes())
        .layer(cors);
    let http_server = async {
        axum::serve(
            TcpListener::bind(http_addr).await?,
            http_routes.into_make_service(),
        )
        .await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    info!("HTTP server listening on {http_addr}");

    info!("send SIGINT (e.g. ctrl+c) to exit");

    tokio::try_join!(grpc_server, http_server)?;
    info!("shutting down...");

    Ok(())
}
