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
    /// Do not open a browser (a viewer's URL is always printed when it is ready).
    #[arg(long, global = true)]
    no_browser: bool,

    /// Host/interface the viewer binds (`0.0.0.0` exposes it to other hosts).
    #[arg(long, global = true, default_value = "127.0.0.1")]
    host: IpAddr,

    /// Trust a git remote (repeatable) without prompting: a full repo URL for an
    /// exact repo, or a `github.com/org/*` form to trust a whole org/prefix.
    #[arg(long = "trust", global = true, value_name = "REMOTE")]
    trust: Vec<String>,

    /// Trust every source (skips the trust gate entirely — only for sources you
    /// already trust, since building runs their code).
    #[arg(long, global = true)]
    trust_all: bool,

    #[command(subcommand)]
    command: OpenCommand,
}

#[derive(Debug, Subcommand)]
enum OpenCommand {
    /// Analyze local Quent artifacts directly.
    Local {
        /// Context directories to analyze. A context directory holds a `model.qmi`
        /// provenance sidecar at its root, plus one per-entity subdirectory per
        /// entity containing that entity's event stream.
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
