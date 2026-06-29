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
mod trust;
mod viewer;
mod wrapper;

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use quent_build_info::{ArtifactInfo, SIDECAR_FILE_NAME};

use crate::error::{OpenError, Result};
use crate::spec::ViewerSpec;
use crate::viewer::ViewerGroup;

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
    match &cli.command {
        OpenCommand::Local { paths } => run_local(&cli, paths).await,
    }
}

/// Recursively discover contexts under `paths`, group them by build spec (same
/// analyzer + pinned commits + format), then build and serve viewers in parallel.
/// Contexts that can't be opened (no analyzer package, unreadable sidecar) are
/// warned and skipped.
async fn run_local(cli: &Cli, paths: &[PathBuf]) -> Result<()> {
    let contexts = spec::discover_contexts(paths)?;

    // One group per build spec; contexts sharing a spec share a viewer.
    let mut groups: BTreeMap<String, ViewerGroup> = BTreeMap::new();
    for context in contexts {
        let spec = match ArtifactInfo::read_sidecar(&context)
            .map_err(|source| OpenError::Sidecar {
                path: context.join(SIDECAR_FILE_NAME),
                source,
            })
            .and_then(|info| ViewerSpec::from_artifact(&context, &info))
        {
            Ok(spec) => spec,
            Err(e) => {
                eprintln!("skipping {}: {e}", context.display());
                continue;
            }
        };
        groups
            .entry(spec.group_key())
            .or_insert_with(|| ViewerGroup {
                spec: spec.clone(),
                contexts: Vec::new(),
            })
            .contexts
            .push(context);
    }

    let groups: Vec<ViewerGroup> = groups.into_values().collect();
    if groups.is_empty() {
        return Err(OpenError::NoContexts);
    }

    // Each viewer builds and runs code from its quent/analyzer remotes; require
    // trust before building. Authorize each distinct remote once, with prompts
    // before parallel builds.
    let mut trust = trust::Trust::new(&cli.trust, cli.trust_all);
    let mut decided: BTreeMap<String, bool> = BTreeMap::new();
    for group in &groups {
        for pin in [&group.spec.quent, &group.spec.analyzer] {
            if let std::collections::btree_map::Entry::Vacant(slot) =
                decided.entry(trust::canonicalize_remote(&pin.remote))
            {
                slot.insert(trust.authorize(&pin.remote, &pin.commit));
            }
        }
    }
    let approved: Vec<ViewerGroup> = groups
        .into_iter()
        .filter(|group| {
            let trusted = [&group.spec.quent, &group.spec.analyzer]
                .iter()
                .all(|pin| decided[&trust::canonicalize_remote(&pin.remote)]);
            if !trusted {
                eprintln!(
                    "skipping {}: source not trusted",
                    group.spec.analyzer_package
                );
            }
            trusted
        })
        .collect();
    if approved.is_empty() {
        return Err(OpenError::NothingTrusted);
    }
    viewer::open_all(approved, cli.no_browser, cli.host).await
}
