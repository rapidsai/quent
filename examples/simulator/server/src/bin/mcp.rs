// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Stdio MCP server for the simulator.
//!
//! Exposes the same analysis tools as the HTTP `/mcp` endpoint, but over
//! stdin/stdout for MCP clients that spawn the server as a subprocess. Reads
//! previously collected telemetry from `--output-dir`.
//!
//! Logs go to stderr; stdout is reserved for the MCP protocol.

use std::path::PathBuf;

use clap::Parser;
use quent_query_engine_server::{initialize_tracing, mcp::serve_stdio};
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use quent_simulator_server::{build_importer, build_lister, parse_format};

#[derive(Parser)]
struct Args {
    /// Log level filter (e.g. "debug", "info", "warn", "error").
    /// Overridden by the RUST_LOG environment variable if set.
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Exporter format of the collected event data (ndjson, msgpack, postcard).
    #[arg(long, default_value = "ndjson", env = "QUENT_COLLECTOR_EXPORTER")]
    exporter: String,

    /// Directory holding the collected event data.
    #[arg(long, default_value = "data", env = "QUENT_COLLECTOR_OUTPUT_DIR")]
    output_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Args {
        log_level,
        exporter,
        output_dir,
    } = Args::parse();

    initialize_tracing(&log_level);

    let format = parse_format(&exporter)?;
    let importer = build_importer(format, output_dir.clone());
    let lister = build_lister(output_dir);

    tracing::info!("serving MCP over stdio");
    serve_stdio::<SimulatorUiAnalyzer>(importer, lister).await
}
