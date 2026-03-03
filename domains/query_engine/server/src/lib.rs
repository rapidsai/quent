//! Utilities for server implementations

use axum::Router as AxumRouter;
use quent_collector::server::{CollectorService, CollectorServiceOptions};
use quent_collector_proto::collector_server::CollectorServer;
use quent_query_engine_analyzer::ui::UiAnalyzer;
use serde::{Deserialize, Serialize};
use tonic::transport::{Server as GrpcServer, server::Router};
use tower_http::cors::CorsLayer;

use crate::cache::AnalyzerCache;

mod cache;
mod error;
mod ui;

pub fn initialize_tracing(log_level: &str) {
    use tracing_subscriber::{
        EnvFilter,
        fmt::{self, format::FmtSpan},
        layer::SubscriberExt,
        registry,
        util::SubscriberInitExt,
    };
    registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(format!("{log_level},h2=off,tonic=off"))),
        )
        .with(
            fmt::layer()
                .with_target(true)
                .with_span_events(FmtSpan::CLOSE)
                .with_writer(std::io::stderr),
        )
        .init();
}

pub fn collector_service<E>(
    options: CollectorServiceOptions,
) -> Result<Router, Box<dyn std::error::Error>>
where
    E: Serialize + Send + Sync + std::fmt::Debug + 'static,
    for<'de> E: Deserialize<'de>,
{
    let collector = CollectorService::<E>::new(options);
    Ok(GrpcServer::builder().add_service(CollectorServer::new(collector)))
}

pub fn analyzer_service_router<A>(
    cors: Option<String>,
) -> Result<AxumRouter, Box<dyn std::error::Error>>
where
    A: UiAnalyzer + Send + Sync + 'static,
    <A as UiAnalyzer>::EntityRef: serde::Serialize,
    <A as UiAnalyzer>::TimelineGlobalParams: Send + Sync + 'static,
    <A as UiAnalyzer>::TimelineParams: Send + Sync + 'static,
    for<'de> <A as UiAnalyzer>::TimelineGlobalParams: serde::Deserialize<'de>,
    for<'de> <A as UiAnalyzer>::TimelineParams: serde::Deserialize<'de>,
{
    let cache = AnalyzerCache::<A>::new();

    let mut http_routes = axum::Router::new().nest("/analyzer", ui::routes(cache));

    if let Some(cors) = cors {
        let cors = CorsLayer::new()
            .allow_origin(cors.parse::<axum::http::HeaderValue>().unwrap())
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([axum::http::header::CONTENT_TYPE]);
        http_routes = http_routes.layer(cors);
    }

    Ok(http_routes)
}
