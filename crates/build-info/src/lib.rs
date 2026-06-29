// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build-time provenance written alongside exported artifacts.
//!
//! An instrumentation `Context` writes an [`ArtifactInfo`] sidecar
//! (`model.qmi`) into each context directory so downstream tools (e.g.
//! `quent-open`) can locate & check out the crate that produced the artifacts,
//! without a hand-maintained config:
//!
//! * [`BuildInfo`] — git provenance. The [`quent`] framework's info is captured
//!   by this crate's `build.rs`; a downstream crate captures its own by calling
//!   [`emit_source`] from its `build.rs`. Each field is optional so absent
//!   provenance stays distinguishable from a real value.
//! * [`ModelInfo`] — the model's identity. The Rust type path and name come from
//!   [`std::any::type_name`]; the cargo package and source git come from a
//!   per-model [`ModelSource`] impl that `model!` generates (so out-of-repo
//!   crates record their own package and git without any provenance threading).
//!
//! Keeping the provenance in a sidecar (rather than embedded in the artifacts)
//! means a single, format-agnostic implementation for all exporters and clean
//! event streams that third-party importers can read as a single object type.

use std::path::Path;

use serde::{Deserialize, Serialize};

mod git;

/// File name of the provenance sidecar written into a context directory.
pub const SIDECAR_FILE_NAME: &str = "model.qmi";

/// Git provenance of a repository, captured at build time. Every field except
/// [`version`](Self::version) is optional and omitted from the serialized
/// sidecar when unknown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    /// Cargo package version.
    pub version: String,
    /// Full commit hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Branch name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Whether the working tree had uncommitted changes at build time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dirty: Option<bool>,
    /// `origin` remote URL, with any embedded userinfo stripped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// Commit timestamp (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_at: Option<String>,
}

/// Identity of the model that produced an artifact. The Rust type to build a
/// viewer from (an analyzer entry point) is supplied by `quent-open`; this only
/// records provenance to locate & check out the producing crate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name (e.g. `"Simulator"`).
    pub name: String,
    /// Cargo package defining the model (e.g. `"quent-simulator-instrumentation"`).
    pub package: String,
    /// Rust type path of the model's event enum
    /// (e.g. `"quent_simulator_instrumentation::SimulatorEvent"`).
    pub type_path: String,
    /// Git provenance of the crate defining the model.
    pub source: BuildInfo,
    /// Cargo package providing this model's `QuentViewer` entry (shares the
    /// model's [`source`](Self::source) git); `None` if the model didn't declare one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analyzer_package: Option<String>,
}

impl BuildInfo {
    /// A [`BuildInfo`] with no provenance, for placeholders where the source is
    /// genuinely unknown.
    pub fn unknown() -> Self {
        Self {
            version: "unknown".to_string(),
            commit: None,
            branch: None,
            dirty: None,
            remote: None,
            built_at: None,
        }
    }
}

impl ModelInfo {
    /// A [`ModelInfo`] with no provenance, for placeholders (e.g. tests) where
    /// the model identity is irrelevant.
    pub fn unknown() -> Self {
        Self {
            name: "unknown".to_string(),
            package: "unknown".to_string(),
            type_path: "unknown".to_string(),
            source: BuildInfo::unknown(),
            analyzer_package: None,
        }
    }
}

/// The provenance written into the `model.qmi` sidecar of each context directory.
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

    /// Write this provenance as the [`SIDECAR_FILE_NAME`] sidecar (pretty JSON)
    /// in `dir`. JSON is used so the sidecar is self-describing and readable
    /// regardless of the artifact serialization format.
    ///
    /// The write is atomic: the JSON goes to a temp file in `dir` that is then
    /// renamed over the final name, so a reader never observes a partial or
    /// torn sidecar.
    pub fn write_sidecar(&self, dir: &Path) -> std::io::Result<()> {
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let tmp = dir.join(format!(".{SIDECAR_FILE_NAME}.tmp"));
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, dir.join(SIDECAR_FILE_NAME))
    }

    /// Read the [`SIDECAR_FILE_NAME`] sidecar from `dir` and parse it. The
    /// inverse of [`write_sidecar`](Self::write_sidecar): a missing sidecar
    /// surfaces as [`std::io::ErrorKind::NotFound`], a malformed one as
    /// [`std::io::ErrorKind::InvalidData`].
    pub fn read_sidecar(dir: &Path) -> std::io::Result<Self> {
        let bytes = std::fs::read(dir.join(SIDECAR_FILE_NAME))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Build info for the quent framework itself, captured by this crate's `build.rs`.
pub fn quent() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: option_env!("QUENT_BUILD_COMMIT").map(str::to_string),
        branch: option_env!("QUENT_BUILD_BRANCH").map(str::to_string),
        dirty: option_env!("QUENT_BUILD_DIRTY").map(|v| v == "true"),
        remote: option_env!("QUENT_BUILD_REMOTE").map(str::to_string),
        built_at: option_env!("QUENT_BUILD_BUILT_AT").map(str::to_string),
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
            commit: Some(commit.to_string()),
            branch: branch.map(str::to_string),
            dirty: dirty.map(|v| v == "true"),
            remote: Some(remote.to_string()),
            built_at: built_at.map(str::to_string),
        },
        _ => quent(),
    }
}

/// Provenance of the crate that defines a model: the cargo package it belongs
/// to and the git origin it was built from.
///
/// Each model carries its own implementation, so the recorded package and
/// source git describe the defining crate — which may live outside the quent
/// repository.
pub trait ModelSource {
    /// Cargo package of the crate defining the model (`env!("CARGO_PKG_NAME")`).
    fn package() -> &'static str;
    /// Git provenance of the crate defining the model.
    fn source() -> BuildInfo;
    /// Cargo package providing this model's `QuentViewer` entry, if declared via
    /// `analyzer_package` in `model!` (shares the model's [`source`](Self::source) git).
    fn analyzer_package() -> Option<&'static str> {
        None
    }

    /// Assemble the [`ModelInfo`] for this model: the Rust type path and name
    /// from [`std::any::type_name`], the cargo package and source git from the
    /// [`package`](Self::package) / [`source`](Self::source) impls above.
    fn model_info() -> ModelInfo {
        let type_path = std::any::type_name::<Self>();
        let event_name = type_path.rsplit("::").next().unwrap_or(type_path);
        let name = event_name.strip_suffix("Event").unwrap_or(event_name);
        ModelInfo {
            name: name.to_string(),
            package: Self::package().to_string(),
            type_path: type_path.to_string(),
            source: Self::source(),
            analyzer_package: Self::analyzer_package().map(str::to_string),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestModel;

    impl ModelSource for TestModel {
        fn package() -> &'static str {
            "quent-build-info"
        }
        fn source() -> BuildInfo {
            BuildInfo::unknown()
        }
    }

    #[test]
    fn unknown_build_info_omits_absent_fields() {
        // Only `version` is non-optional, so absent provenance serializes to a
        // single key rather than a string of `"unknown"` sentinels.
        let json = serde_json::to_string(&BuildInfo::unknown()).unwrap();
        assert_eq!(json, r#"{"version":"unknown"}"#);
    }

    #[test]
    fn artifact_info_roundtrips() {
        let info = ArtifactInfo::new(TestModel::model_info());
        let bytes = serde_json::to_vec(&info).unwrap();
        let back: ArtifactInfo = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(info, back);
    }

    #[test]
    fn analyzer_package_threads_and_roundtrips() {
        struct WithAnalyzer;
        impl ModelSource for WithAnalyzer {
            fn package() -> &'static str {
                "quent-build-info"
            }
            fn source() -> BuildInfo {
                BuildInfo::unknown()
            }
            fn analyzer_package() -> Option<&'static str> {
                Some("quent-simulator-analyzer")
            }
        }

        // `model_info()` carries the declared analyzer package...
        let info = WithAnalyzer::model_info();
        assert_eq!(
            info.analyzer_package.as_deref(),
            Some("quent-simulator-analyzer")
        );
        let back: ModelInfo = serde_json::from_slice(&serde_json::to_vec(&info).unwrap()).unwrap();
        assert_eq!(info, back);

        // ...and absent by default, omitted from the serialized form.
        let none = TestModel::model_info();
        assert_eq!(none.analyzer_package, None);
        assert!(
            !serde_json::to_string(&none)
                .unwrap()
                .contains("analyzer_package")
        );
    }

    #[test]
    fn write_sidecar_is_atomic_and_named() {
        let dir = std::env::temp_dir().join("quent_build_info_sidecar_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let info = ArtifactInfo::new(TestModel::model_info());
        info.write_sidecar(&dir).unwrap();

        assert!(dir.join(SIDECAR_FILE_NAME).is_file());
        // The temp file used for the atomic rename must not linger.
        assert!(!dir.join(format!(".{SIDECAR_FILE_NAME}.tmp")).exists());

        // The sidecar reads back to an equal value.
        assert_eq!(ArtifactInfo::read_sidecar(&dir).unwrap(), info);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
