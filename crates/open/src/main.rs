// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Command-line frontend for `quent-open`: opens local Quent artifacts via the
//! built-in [`LocalLoader`]. See the crate docs for the library and custom loaders.

use std::future::Future;
use std::net::IpAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use quent_open::{LocalLoader, OpenOptions, Result, Trust};

#[derive(Debug, Parser)]
#[command(name = "quent-open")]
#[command(about = "Open local Quent artifacts in an application-specific viewer")]
struct Cli {
    /// Do not open a browser; print each viewer URL when ready.
    #[arg(long, global = true)]
    no_browser: bool,

    /// Host/interface the viewer binds (`0.0.0.0` exposes it to other hosts).
    #[arg(long, global = true, default_value = "127.0.0.1")]
    host: IpAddr,

    /// Trust a git remote without prompting (repeatable): full repo URL, or
    /// `github.com/org/*` for an org/prefix.
    #[arg(long = "trust", global = true, value_name = "REMOTE")]
    trust: Vec<String>,

    /// Trust every source, skipping the trust gate; only use for trusted sources,
    /// because building runs their code.
    #[arg(long, global = true)]
    trust_all: bool,

    #[command(subcommand)]
    command: OpenCommand,
}

#[derive(Debug, Subcommand)]
enum OpenCommand {
    /// Analyze local Quent artifacts directly.
    Local {
        /// Context directories to analyze; each has a root `model.qmi` sidecar and
        /// per-entity subdirectories containing event streams.
        #[arg(required = true, num_args = 1..)]
        paths: Vec<PathBuf>,
    },

    /// Analyze a benchmark run's telemetry fetched from the Benchmarking API
    /// (an internal RAPIDS/NVIDIA service; not useful to non-NVIDIA users).
    #[cfg(feature = "db")]
    Db {
        /// Benchmark run to open: its integer run id, or its `run_id` UUID.
        run: String,

        /// Base URL of the Benchmarking API (e.g. `https://host`). Also read from
        /// `QUENT_OPEN_API_BASE_URL` (a `.env` file is loaded first).
        #[arg(long, env = "QUENT_OPEN_API_BASE_URL")]
        api_base_url: String,

        /// Bearer token for the Benchmarking API. Also read from `QUENT_OPEN_TOKEN`
        /// (a `.env` file is loaded first).
        #[arg(long, env = "QUENT_OPEN_TOKEN", hide_env_values = true)]
        token: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load `.env` (e.g. `QUENT_OPEN_API_BASE_URL` / `QUENT_OPEN_TOKEN`) before clap
    // reads the environment; real environment variables still take precedence.
    #[cfg(feature = "db")]
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();
    let options = OpenOptions {
        no_browser: cli.no_browser,
        host: cli.host,
        trust: Trust::new(&cli.trust, cli.trust_all),
    };
    match cli.command {
        OpenCommand::Local { paths } => {
            serve_until_interrupt(quent_open::run(LocalLoader::new(paths)?, options)).await
        }
        #[cfg(feature = "db")]
        OpenCommand::Db {
            run,
            api_base_url,
            token,
        } => {
            let loader = quent_open::DbLoader::new(api_base_url, token, run)?;
            serve_until_interrupt(quent_open::run(loader, options)).await
        }
    }
}

/// Serve `viewers` until it finishes or the user presses Ctrl-C. On interrupt we
/// drop the future, which drops the loader — removing any scratch `TempDir` it
/// owns — and kills the spawned viewers (`kill_on_drop`) before we exit, instead
/// of the default SIGINT termination that would leak them.
async fn serve_until_interrupt(viewers: impl Future<Output = Result<()>>) -> Result<()> {
    tokio::select! {
        result = viewers => result,
        _ = tokio::signal::ctrl_c() => Ok(()),
    }
}
