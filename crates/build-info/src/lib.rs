// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build-time provenance embedded in exported artifacts.
//!
//! Exporters write an [`ArtifactInfo`] header into each artifact so downstream
//! tools (e.g. `quent-open`) can locate & check out the crate that produced it
//! and pick the Rust type to build a viewer, without a hand-maintained config:
//!
//! * [`BuildInfo`] — git provenance. The [`quent`] framework's info is captured
//!   by this crate's `build.rs`; a downstream crate captures its own by calling
//!   [`emit_source`] from its `build.rs`.
//! * [`ModelInfo`] — the model's identity. The Rust type path and name come from
//!   [`std::any::type_name`]; the cargo package and source git come from a
//!   per-model [`ModelSource`] impl that `model!` generates (so out-of-repo
//!   crates record their own package and git without any provenance threading).

use serde::{Deserialize, Serialize};

mod git;

/// Git provenance of a repository, captured at build time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    /// Cargo package version.
    pub version: String,
    /// Full commit hash, or `"unknown"`.
    pub commit: String,
    /// Branch name, or `"unknown"`.
    pub branch: String,
    /// Whether the working tree had uncommitted changes at build time.
    pub dirty: bool,
    /// `origin` remote URL, or `"unknown"`.
    pub remote: String,
    /// Commit timestamp (RFC 3339), or `"unknown"`.
    pub built_at: String,
}

/// How `quent-open` should turn the model's [`type_path`](ModelInfo::type_path)
/// into a viewer entry point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelKind {
    /// `type_path` is an application event enum embedding the query-engine model.
    #[default]
    QueryEngineEvent,
    /// `type_path` is a `UiAnalyzer` implementation.
    Analyzer,
    /// `type_path` is a `QuentViewer` implementation.
    Viewer,
}

/// Identity of the model that produced an artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name (e.g. `"Simulator"`).
    pub name: String,
    /// Cargo package defining the model (e.g. `"quent-simulator-instrumentation"`).
    pub package: String,
    /// Rust type path of the model's event enum / analyzer / viewer
    /// (e.g. `"quent_simulator_instrumentation::SimulatorEvent"`).
    pub type_path: String,
    /// How to interpret [`type_path`](Self::type_path).
    #[serde(default)]
    pub kind: ModelKind,
    /// Git provenance of the crate defining the model.
    pub source: BuildInfo,
}

impl BuildInfo {
    /// A [`BuildInfo`] with no provenance, for placeholders where the source is
    /// genuinely unknown.
    pub fn unknown() -> Self {
        Self {
            version: "unknown".to_string(),
            commit: "unknown".to_string(),
            branch: "unknown".to_string(),
            dirty: false,
            remote: "unknown".to_string(),
            built_at: "unknown".to_string(),
        }
    }
}

impl ModelInfo {
    /// A placeholder [`ModelInfo`] with no identity, for tests and callers
    /// without a real model.
    pub fn unknown() -> Self {
        Self {
            name: "unknown".to_string(),
            package: "unknown".to_string(),
            type_path: "unknown".to_string(),
            kind: ModelKind::default(),
            source: BuildInfo::unknown(),
        }
    }
}

/// The full provenance header written at the start of each exported artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactInfo {
    /// Build info of the quent framework.
    pub quent: BuildInfo,
    /// Identity and source of the model that produced the artifact.
    pub model: ModelInfo,
}

impl ArtifactInfo {
    /// Construct an [`ArtifactInfo`] pairing the [`quent`] framework build info
    /// with a [`ModelInfo`].
    pub fn new(model: ModelInfo) -> Self {
        Self {
            quent: quent(),
            model,
        }
    }
}

/// Build info for the quent framework itself, captured by this crate's `build.rs`.
pub fn quent() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: option_env!("QUENT_BUILD_COMMIT")
            .unwrap_or("unknown")
            .to_string(),
        branch: option_env!("QUENT_BUILD_BRANCH")
            .unwrap_or("unknown")
            .to_string(),
        dirty: matches!(option_env!("QUENT_BUILD_DIRTY"), Some("true")),
        remote: option_env!("QUENT_BUILD_REMOTE")
            .unwrap_or("unknown")
            .to_string(),
        built_at: option_env!("QUENT_BUILD_BUILT_AT")
            .unwrap_or("unknown")
            .to_string(),
    }
}

/// Build a model's source [`BuildInfo`] from `QUENT_SOURCE_*` values captured by
/// [`emit_source`], falling back to [`quent`] when they are absent.
///
/// `version` is the model crate's own version (`env!("CARGO_PKG_VERSION")` at the
/// call site) — not this crate's. Pass the results of `option_env!("QUENT_SOURCE_*")`.
/// When a downstream crate has not opted in via [`emit_source`] the values are
/// `None` and the model is assumed to live in the quent repository (the in-repo
/// case), so the [`quent`] build info is used.
pub fn source_or_quent(
    version: &str,
    remote: Option<&str>,
    commit: Option<&str>,
    branch: Option<&str>,
    dirty: Option<&str>,
    built_at: Option<&str>,
) -> BuildInfo {
    match (remote, commit) {
        (Some(remote), Some(commit)) => BuildInfo {
            version: version.to_string(),
            commit: commit.to_string(),
            branch: branch.unwrap_or("unknown").to_string(),
            dirty: matches!(dirty, Some("true")),
            remote: remote.to_string(),
            built_at: built_at.unwrap_or("unknown").to_string(),
        },
        _ => quent(),
    }
}

/// Per-model provenance hook, generated by `model!` for each event enum.
///
/// Lets exporters record the model's cargo package and the git of the crate that
/// defines it — including out-of-repo crates — because the impl's `env!` /
/// `option_env!` are evaluated in that crate. The Rust type path and model name
/// are derived separately from [`std::any::type_name`], so no provenance has to
/// be threaded through `Context`/`create_exporter` call sites.
pub trait ModelSource {
    /// Cargo package of the crate defining the model (`env!("CARGO_PKG_NAME")`).
    fn package() -> &'static str;
    /// Git provenance of the crate defining the model.
    fn source() -> BuildInfo;
}

/// Assemble the [`ModelInfo`] for event type `T`: the Rust type path and name
/// from [`std::any::type_name`], the cargo package and source git from `T`'s
/// [`ModelSource`] impl.
pub fn model_info<T: ModelSource + ?Sized>() -> ModelInfo {
    let type_path = std::any::type_name::<T>();
    let event_name = type_path.rsplit("::").next().unwrap_or(type_path);
    let name = event_name.strip_suffix("Event").unwrap_or(event_name);
    ModelInfo {
        name: name.to_string(),
        package: T::package().to_string(),
        type_path: type_path.to_string(),
        kind: ModelKind::QueryEngineEvent,
        source: T::source(),
    }
}

/// Call from a downstream crate's `build.rs` to capture that crate's repository
/// git into `QUENT_SOURCE_*` env vars. The `model!`/`instrumentation!` macros
/// read them via `option_env!`, so the values bake into the downstream crate.
///
/// Requires a `build-dependencies` entry on `quent-build-info` and that the
/// `build.rs` lives in the same package that invokes the macros.
pub fn emit_source() {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    git::emit("QUENT_SOURCE", &manifest_dir);
}
