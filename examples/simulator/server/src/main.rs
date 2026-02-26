use std::net::ToSocketAddrs;

use clap::Parser;
use quent_query_engine_server::{analyzer_service_router, collector_service, initialize_tracing};
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use quent_simulator_events::SimulatorEvent;
use tokio::net::TcpListener;

mod defaults {
    /// Default collector socket address to listen on.
    pub(crate) const QUENT_COLLECTOR_ADDRESS: &str = "[::]:7836";
    /// Default analyzer socket address to listen on.
    pub(crate) const QUENT_ANALYZER_ADDRESS: &str = "[::]:8080";
}

mod env {
    /// Collector socket address environment variable name.
    pub(crate) const QUENT_COLLECTOR_ADDRESS: &str = "QUENT_COLLECTOR_ADDRESS";
    /// Analyzer socket address environment variable name.
    pub(crate) const QUENT_ANALYZER_ADDRESS: &str = "QUENT_ANALYZER_ADDRESS";
    /// Optional CORS address environment variable name.
    pub(crate) const QUENT_ANALYZER_CORS_ADDRESS: &str = "QUENT_ANALYZER_CORS_ADDRESS";
}

#[derive(Parser)]
struct Args {
    /// Log level filter (e.g. "debug", "info", "warn", "error").
    /// Overridden by the RUST_LOG environment variable if set.
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Socket address for the collector gRPC server (e.g. "[::]:7836").
    /// Overridden by the QUENT_COLLECTOR_ADDRESS environment variable if set.
    #[arg(long, default_value = defaults::QUENT_COLLECTOR_ADDRESS, env = env::QUENT_COLLECTOR_ADDRESS)]
    collector_address: String,

    /// Socket address for the analyzer HTTP server (e.g. "[::]:8080").
    /// Overridden by the QUENT_ANALYZER_ADDRESS environment variable if set.
    #[arg(long, default_value = defaults::QUENT_ANALYZER_ADDRESS, env = env::QUENT_ANALYZER_ADDRESS)]
    analyzer_address: String,

    /// Address to allow CORS requests from (e.g. "http://localhost:5173").
    /// If not set, CORS is disabled.
    /// Overridden by the QUENT_CORS_ADDRESS environment variable if set.
    #[arg(long, env = env::QUENT_ANALYZER_CORS_ADDRESS)]
    cors_address: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Args {
        log_level,
        cors_address,
        collector_address,
        analyzer_address,
    } = Args::parse();

    initialize_tracing(&log_level);

    let collector_addr = collector_address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| format!("unable to resolve socket address: {collector_address}"))?;
    let collector = async {
        collector_service::<SimulatorEvent>()?
            .serve(collector_addr)
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
    };

    let analyzer_addr = analyzer_address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| format!("unable to resolve socket address: {analyzer_address}"))?;
    let analyzer = async {
        axum::serve(
            TcpListener::bind(analyzer_addr).await?,
            analyzer_service_router::<SimulatorUiAnalyzer>(cors_address)?.into_make_service(),
        )
        .await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    tracing::info!("listening on {collector_addr} and {analyzer_addr}");

    tokio::try_join!(collector, analyzer)?;

    Ok(())
}
