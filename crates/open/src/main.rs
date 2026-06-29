// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `quent-open` opens local Quent benchmark artifacts in an application-specific
//! viewer. See <https://github.com/rapidsai/quent/issues/234>.
//!
//! This is a scaffold: the CLI surface is in place and the provenance sidecar
//! (`model.qmi`) is read to identify each artifact's model; selecting, building,
//! serving and launching a viewer is not yet implemented.

mod error;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use quent_build_info::{ArtifactInfo, SIDECAR_FILE_NAME};

use crate::error::{OpenError, Result};

#[derive(Debug, Parser)]
#[command(name = "quent-open")]
#[command(about = "Open local Quent benchmark artifacts in an application-specific viewer")]
struct Cli {
    /// Config file path. Defaults to ./quent-open.toml, then ~/.config/quent/open.toml.
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Do not open a browser.
    #[arg(long, global = true)]
    no_browser: bool,

    /// Print the opened viewer URL.
    #[arg(long, global = true)]
    print_url: bool,

    /// Force a specific viewer by name from the config (skips automatic matching).
    #[arg(long, global = true)]
    viewer: Option<String>,

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
    match &cli.command {
        OpenCommand::Local { paths } => run_local(&cli, paths).await,
    }
}

/// Open local artifacts in a viewer.
///
/// Reads the `model.qmi` provenance sidecar in each context directory to identify
/// the model that produced the artifacts. Each path is treated as a context
/// directory; resolving a sidecar from a nested per-entity subdirectory is not
/// supported.
///
/// TODO(#234): load config, then select and build a viewer matching the model's
/// provenance, serve the artifacts, and launch the viewer / open a browser.
async fn run_local(_cli: &Cli, paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        let info = ArtifactInfo::read_sidecar(path).map_err(|source| OpenError::Sidecar {
            path: path.join(SIDECAR_FILE_NAME),
            source,
        })?;
        report_artifact(path, &info);
    }
    Ok(())
}

/// Print the provenance discovered for `path`. The model `source` is what later
/// drives checking out and building a viewer for the producing crate.
fn report_artifact(path: &Path, info: &ArtifactInfo) {
    let model = &info.model;
    println!("{}", path.display());
    println!("  model:    {} ({})", model.name, model.type_path);
    println!("  package:  {}", model.package);
    if let Some(analyzer) = &model.analyzer_package {
        println!("  analyzer: {analyzer}");
    }
    println!("  quent:    {}", info.quent);
    println!("  source:   {}", model.source);
}
