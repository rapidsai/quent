// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Open local Quent artifacts in an application-specific viewer.
//!
//! Given a context directory, read `model.qmi`, generate a viewer crate pinned
//! to the recorded quent/analyzer commits (see [`wrapper`]), build and serve it,
//! and open a browser.
//!
//! The first viewer build fetches git sources and compiles the embedded UI,
//! invoking `pnpm`/`node`; these must be on `PATH`.

mod error;
mod spec;
mod viewer;
mod wrapper;

use std::net::IpAddr;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use quent_build_info::{ArtifactInfo, SIDECAR_FILE_NAME};

use crate::error::{OpenError, Result};
use crate::spec::ViewerSpec;

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

    /// Host/interface the viewer binds (`0.0.0.0` exposes it to other hosts).
    #[arg(long, global = true, default_value = "127.0.0.1")]
    host: IpAddr,

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
/// Reads each context directory's `model.qmi` sidecar, then resolves the viewer
/// build spec (analyzer package, pinned git sources, artifact format). Each path
/// is treated as a context directory; resolving a sidecar from a nested per-entity
/// subdirectory is not supported.
///
/// For each context directory: generate a viewer crate from the spec, build it,
/// serve the artifacts, and open a browser. Serving blocks until the viewer
/// exits, so multiple paths are opened one after another.
async fn run_local(cli: &Cli, paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        let info = ArtifactInfo::read_sidecar(path).map_err(|source| OpenError::Sidecar {
            path: path.join(SIDECAR_FILE_NAME),
            source,
        })?;
        report_artifact(path, &info);
        let spec = ViewerSpec::from_artifact(path, &info)?;
        report_spec(&spec);
        viewer::open(&spec, cli.no_browser, cli.print_url, cli.host).await?;
    }
    Ok(())
}

/// Print the resolved viewer build spec for `spec`.
fn report_spec(spec: &ViewerSpec) {
    println!(
        "  viewer:   {}::Viewer ({})",
        spec.analyzer_crate(),
        spec.format.extension()
    );
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
