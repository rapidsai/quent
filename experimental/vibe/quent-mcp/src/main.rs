// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use quent_mcp::serve_stdio;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Parser)]
#[command(about = "Experimental stdio MCP bridge for the Quent REST API")]
struct Args {
    /// Quent REST API base URL.
    #[arg(
        long,
        env = "QUENT_API_BASE_URL",
        default_value = "http://localhost:8080/api"
    )]
    api_base: String,

    /// Tracing filter used when RUST_LOG is unset.
    #[arg(long, default_value = "info")]
    log_level: String,
}

fn initialize_tracing(log_level: &str) {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    initialize_tracing(&args.log_level);

    tracing::info!("serving MCP over stdio");
    serve_stdio(&args.api_base).await
}
