// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Utilities for server implementations

use crate::{analyzer_cache::AnalyzerCache, state::ServiceState, timeline_cache::TimelineCache};
use axum::Router as AxumRouter;
use quent_collector::server::{CollectorService, CollectorServiceOptions};
use quent_collector_proto::collector_server::CollectorServer;
use quent_query_engine_analyzer::ui::UiAnalyzer;

use serde::{Deserialize, Serialize};
use tonic::transport::{Server as GrpcServer, server::Router};
use tower_http::cors::CorsLayer;

pub mod analyzer_cache;
pub mod error;
#[cfg(feature = "mcp")]
pub mod mcp;
mod state;
mod timeline_cache;
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
    E: Serialize + Send + Sync + 'static + quent_collector::ModelSource,
    for<'de> E: Deserialize<'de>,
{
    let collector = CollectorService::<E>::new(options);
    Ok(GrpcServer::builder().add_service(CollectorServer::new(collector)))
}

pub fn analyzer_service_router<A>(
    importer: Box<analyzer_cache::ImporterFn<A>>,
    lister: Box<analyzer_cache::ListerFn>,
    cors: Option<String>,
) -> Result<AxumRouter, Box<dyn std::error::Error>>
where
    A: UiAnalyzer + Send + Sync + 'static,
    <A as UiAnalyzer>::EntityRef: serde::Serialize,
{
    let state = ServiceState {
        analyzers: AnalyzerCache::<A>::new(importer, lister),
        timelines: TimelineCache::new(),
    };

    #[cfg(feature = "mcp")]
    let mcp_service = mcp::http_service(state.clone());

    let mut http_routes = axum::Router::new().nest("/api/engines", ui::routes(state));

    #[cfg(feature = "mcp")]
    {
        http_routes = http_routes.nest_service("/mcp", mcp_service);
    }

    #[cfg(feature = "swagger")]
    {
        use utoipa::OpenApi;
        use utoipa_swagger_ui::SwaggerUi;
        let api = ui::ApiDoc::openapi();
        http_routes =
            http_routes.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api));
    }

    if let Some(cors) = cors {
        let mut cors = CorsLayer::new()
            .allow_origin(cors.parse::<axum::http::HeaderValue>().unwrap())
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([axum::http::header::CONTENT_TYPE]);

        // The streamable HTTP MCP transport adds session/protocol headers, uses
        // DELETE to tear down a session, and returns the session id to the
        // client, so browser-based MCP clients need these allowed and exposed.
        #[cfg(feature = "mcp")]
        {
            use axum::http::{HeaderName, Method, header::CONTENT_TYPE};
            const MCP_SESSION_ID: HeaderName = HeaderName::from_static("mcp-session-id");
            const MCP_PROTOCOL_VERSION: HeaderName =
                HeaderName::from_static("mcp-protocol-version");
            // Sent when a browser client resumes an interrupted SSE stream.
            const LAST_EVENT_ID: HeaderName = HeaderName::from_static("last-event-id");
            cors = cors
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS, Method::DELETE])
                .allow_headers([
                    CONTENT_TYPE,
                    MCP_SESSION_ID,
                    MCP_PROTOCOL_VERSION,
                    LAST_EVENT_ID,
                ])
                .expose_headers([MCP_SESSION_ID]);
        }

        http_routes = http_routes.layer(cors);
    }

    #[cfg(feature = "ui")]
    {
        http_routes = http_routes.fallback(axum::routing::get(ui::embedded::serve));
    }

    Ok(http_routes)
}
