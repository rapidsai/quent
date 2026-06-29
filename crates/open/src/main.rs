// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Command-line frontend for `quent-open`: opens local Quent artifacts via the
//! built-in [`LocalLoader`]. See the crate docs for the library and custom loaders.

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let options = OpenOptions {
        no_browser: cli.no_browser,
        host: cli.host,
        trust: Trust::new(&cli.trust, cli.trust_all),
    };
    match cli.command {
        OpenCommand::Local { paths } => quent_open::run(LocalLoader { paths }, options).await,
    }
}
