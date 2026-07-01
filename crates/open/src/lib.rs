// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Open local Quent benchmark artifacts in an application-specific viewer.
//!
//! Given a context directory, read `model.qmi`, generate a viewer crate pinned
//! to the recorded quent/analyzer commits, build and serve it, and open a browser.
//!
//! The first viewer build fetches git sources and compiles the embedded UI,
//! invoking `pnpm`/`node`; these must be on `PATH`.
//!
//! # Library: custom loaders
//!
//! The `quent-open` binary opens artifacts on the local filesystem. To open
//! artifacts from elsewhere — a remote database, an object store, an HTTP API —
//! implement [`Loader`]: fetch the artifacts (event streams + `model.qmi`) into
//! context directories on disk, then hand them to [`run`], which groups, gates on
//! [trust](Trust), builds, and serves them exactly as the local path does.
//!
//! ```ignore
//! use std::path::PathBuf;
//! use quent_open::{Loader, OpenOptions, Result, Trust, run};
//!
//! /// Fetches a telemetry artifact by database id over some API.
//! struct DbLoader {
//!     id: String,
//!     scratch: tempfile::TempDir, // kept alive for the lifetime of the loader
//! }
//!
//! impl Loader for DbLoader {
//!     async fn load(&self) -> Result<Vec<PathBuf>> {
//!         // GET the artifact for `self.id`, writing each context (its `model.qmi`
//!         // plus per-entity event streams) into a UUID-named directory under
//!         // `self.scratch`, then return those context directories.
//!         todo!()
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let loader = DbLoader { id: "abc123".into(), scratch: tempfile::tempdir()? };
//!     let options = OpenOptions {
//!         no_browser: false,
//!         host: std::net::Ipv4Addr::LOCALHOST.into(),
//!         trust: Trust::new(&[], false),
//!     };
//!     run(loader, options).await
//! }
//! ```

mod error;
mod spec;
mod trust;
mod viewer;
mod wrapper;

use std::collections::BTreeMap;
use std::future::Future;
use std::net::IpAddr;
use std::path::PathBuf;

use quent_build_info::{ArtifactInfo, SIDECAR_FILE_NAME};

pub use crate::error::{OpenError, Result};
pub use crate::spec::{Format, GitPin, ViewerSpec, discover_contexts};
pub use crate::trust::{Trust, canonicalize_remote};
pub use crate::viewer::ViewerGroup;

/// Options controlling how viewers are built, served, and gated on source trust.
pub struct OpenOptions {
    /// Print each viewer's URL but do not open a browser.
    pub no_browser: bool,
    /// Host/interface the viewers bind (`0.0.0.0` to allow other hosts).
    pub host: IpAddr,
    /// Trust policy applied to every source before it is built and run.
    pub trust: Trust,
}

/// A source of Quent artifacts for [`run`] to open.
///
/// A loader fetches/materializes artifacts as context directories (each holding a
/// `model.qmi` sidecar plus per-entity event-stream subdirectories) and returns
/// their paths. Each directory must be named with the context's UUID: the viewer
/// indexes contexts by a `<uuid>` directory name (skipping any whose name is not a
/// UUID) and stages them into one serving root by name, so the names must be unique
/// across the returned set. The exporter already names context directories this
/// way; a custom loader must preserve it.
///
/// [`run`] holds the loader for the whole call, so a loader that owns a scratch
/// directory (e.g. a [`tempfile::TempDir`]) keeps the fetched artifacts alive while
/// the viewer serves them.
///
/// [`tempfile::TempDir`]: https://docs.rs/tempfile/latest/tempfile/struct.TempDir.html
pub trait Loader {
    /// Materialize the artifacts and return their context directories.
    fn load(&self) -> impl Future<Output = Result<Vec<PathBuf>>>;
}

/// The built-in loader: discover context directories under local `paths`.
pub struct LocalLoader {
    pub paths: Vec<PathBuf>,
}

impl Loader for LocalLoader {
    async fn load(&self) -> Result<Vec<PathBuf>> {
        discover_contexts(&self.paths)
    }
}

/// Load artifacts via `loader`, then [`open()`] them. `loader` is held until
/// serving ends, keeping any scratch storage it owns alive.
pub async fn run(loader: impl Loader, options: OpenOptions) -> Result<()> {
    let contexts = loader.load().await?;
    open(contexts, options).await
}

/// Group `contexts` into one viewer per distinct build spec (same analyzer + pinned
/// commits + format), gate each source on [trust](OpenOptions::trust), then build
/// and serve the approved viewers in parallel. Contexts that can't be opened (no
/// analyzer package, unreadable sidecar) are skipped with a warning rather than
/// aborting.
pub async fn open(contexts: Vec<PathBuf>, options: OpenOptions) -> Result<()> {
    let OpenOptions {
        no_browser,
        host,
        mut trust,
    } = options;

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
    let mut decided: BTreeMap<String, bool> = BTreeMap::new();
    for group in &groups {
        for pin in [&group.spec.quent, &group.spec.analyzer] {
            if let std::collections::btree_map::Entry::Vacant(slot) =
                decided.entry(canonicalize_remote(&pin.remote))
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
                .all(|pin| decided[&canonicalize_remote(&pin.remote)]);
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
    viewer::open_all(approved, no_browser, host).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_loader_discovers_contexts() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = tmp.path().join("run");
        std::fs::create_dir_all(ctx.join("engine")).unwrap();
        std::fs::write(ctx.join("engine").join("events.ndjson"), b"").unwrap();
        std::fs::write(ctx.join(SIDECAR_FILE_NAME), b"{}").unwrap();

        let paths = vec![tmp.path().to_path_buf()];
        let found = LocalLoader {
            paths: paths.clone(),
        }
        .load()
        .await
        .unwrap();
        assert_eq!(found, discover_contexts(&paths).unwrap());
        assert_eq!(found.len(), 1);
    }
}
