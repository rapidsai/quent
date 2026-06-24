// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use thiserror::Error;

/// Result alias for `quent-open` operations.
pub type Result<T> = std::result::Result<T, OpenError>;

/// Errors that can occur while opening Quent artifacts in a viewer.
#[derive(Debug, Error)]
pub enum OpenError {
    /// The provenance sidecar (`model.qmi`) is missing, malformed, or unreadable.
    #[error("failed to read provenance sidecar '{path}': {source}")]
    Sidecar {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The model did not declare an `analyzer_package`, so no viewer can be built
    /// for it.
    #[error(
        "model '{model}' declares no analyzer package (set `analyzer_package` in its `model!`)"
    )]
    NoAnalyzer { model: String },

    /// No event stream with a recognized extension was found, so the artifact
    /// serialization format could not be determined.
    #[error(
        "could not determine the artifact format under '{root}': no ndjson, msgpack, or postcard event streams found"
    )]
    UnknownFormat { root: PathBuf },

    /// The sidecar lacks the git provenance (remote and commit) needed to fetch a
    /// crate for the viewer build.
    #[error(
        "{what} provenance is incomplete: a git remote and commit are required to build a viewer"
    )]
    MissingProvenance { what: String },

    /// No cache directory could be resolved for the generated viewer builds.
    #[error("could not resolve a cache directory for viewer builds")]
    NoCacheDir,

    /// Spawning a child process (cargo, the viewer binary) failed.
    #[error("failed to spawn {what}: {source}")]
    Spawn {
        what: String,
        #[source]
        source: std::io::Error,
    },

    /// Building the generated viewer crate failed (non-zero `cargo build`).
    #[error("building the viewer failed (cargo exited with {status})")]
    Build { status: String },

    /// The viewer process exited or never reported a URL before serving.
    #[error("the viewer exited unexpectedly (status {status})")]
    ViewerExited { status: String },

    /// An I/O error occurred (reading artifacts, spawning the viewer, etc.).
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
