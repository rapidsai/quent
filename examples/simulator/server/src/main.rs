// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{net::ToSocketAddrs, path::PathBuf};

use clap::Parser;
use quent_collector::server::CollectorServiceOptions;
use quent_exporter::{ExporterOptions, FileSystemExporterOptions};
use quent_query_engine_server::{analyzer_service_router, collector_service, initialize_tracing};
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use quent_simulator_instrumentation::SimulatorEvent;
use quent_simulator_server::{build_importer, build_lister, parse_format};
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
    /// Collector output directory environment variable name.
    pub(crate) const QUENT_COLLECTOR_OUTPUT_DIR: &str = "QUENT_COLLECTOR_OUTPUT_DIR";
    /// Exporter type environment variable name.
    pub(crate) const QUENT_COLLECTOR_EXPORTER: &str = "QUENT_COLLECTOR_EXPORTER";
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

    /// Socket address for the collector gRPC server (e.g. `[::]:7836`).
    /// Overridden by the QUENT_COLLECTOR_ADDRESS environment variable if set.
    #[arg(long, default_value = defaults::QUENT_COLLECTOR_ADDRESS, env = env::QUENT_COLLECTOR_ADDRESS)]
    collector_address: String,

    /// Exporter format for collected event data (ndjson, msgpack, postcard).
    /// Overridden by the QUENT_COLLECTOR_EXPORTER environment variable if set.
    #[arg(long, default_value = "ndjson", env = env::QUENT_COLLECTOR_EXPORTER)]
    exporter: String,

    /// Output directory for collected event data.
    /// Overridden by the QUENT_COLLECTOR_OUTPUT_DIR environment variable if set.
    #[arg(long, default_value = "data", env = env::QUENT_COLLECTOR_OUTPUT_DIR)]
    output_dir: PathBuf,

    /// Socket address for the analyzer HTTP server (e.g. `[::]:8080`).
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
        exporter,
        output_dir,
        analyzer_address,
    } = Args::parse();

    initialize_tracing(&log_level);

    let collector_addr = collector_address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| format!("unable to resolve socket address: {collector_address}"))?;

    let importer_output_dir = output_dir.clone();
    let lister_output_dir = output_dir.clone();

    let format = parse_format(&exporter)?;
    let exporter_kind = ExporterOptions::FileSystem(FileSystemExporterOptions {
        format,
        root: output_dir,
    });

    let collector_options = CollectorServiceOptions {
        exporter: exporter_kind,
    };
    let collector = async {
        collector_service::<SimulatorEvent>(collector_options)?
            .serve(collector_addr)
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
    };

    let analyzer_addr = analyzer_address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| format!("unable to resolve socket address: {analyzer_address}"))?;

    let lister = build_lister(lister_output_dir);
    let importer = build_importer(format, importer_output_dir);

    let analyzer = async {
        axum::serve(
            TcpListener::bind(analyzer_addr).await?,
            analyzer_service_router::<SimulatorUiAnalyzer>(importer, lister, cors_address)?
                .into_make_service(),
        )
        .await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    tracing::info!("listening on {collector_addr} and {analyzer_addr}");

    tokio::try_join!(collector, analyzer)?;

    Ok(())
}
