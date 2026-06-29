// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build a [`ViewerSpec`] from a context's `model.qmi`: pinned git sources,
//! analyzer package, and artifact format for generating/building a viewer.

use std::path::{Path, PathBuf};

use quent_build_info::{ArtifactInfo, BuildInfo};

use crate::error::{OpenError, Result};

/// Serialization format of an artifact's event streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Ndjson,
    Msgpack,
    Postcard,
}

impl Format {
    /// File extension of an event stream in this format.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Ndjson => "ndjson",
            Format::Msgpack => "msgpack",
            Format::Postcard => "postcard",
        }
    }

    /// The `quent_exporter::FileSystemFormat` variant name, for generated code.
    pub fn variant(self) -> &'static str {
        match self {
            Format::Ndjson => "Ndjson",
            Format::Msgpack => "Msgpack",
            Format::Postcard => "Postcard",
        }
    }

    fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "ndjson" => Some(Format::Ndjson),
            "msgpack" => Some(Format::Msgpack),
            "postcard" => Some(Format::Postcard),
            _ => None,
        }
    }
}

/// A git source pinned to an exact commit, as recorded in the sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitPin {
    pub remote: String,
    pub commit: String,
}

impl GitPin {
    /// Remote as a Cargo `git = "..."` URL.
    ///
    /// Rewrite git's scp-style `git@host:path` to `ssh://git@host/path`, which
    /// Cargo accepts. Leave URLs with a scheme (`https://`, `ssh://`, ...) and
    /// local paths unchanged; like git, treat a remote as scp-style only when the
    /// first colon has no earlier slash, so `/tmp/foo:bar` stays a path.
    pub fn cargo_url(&self) -> String {
        if self.remote.contains("://") {
            return self.remote.clone();
        }
        match self.remote.split_once(':') {
            Some((host, path)) if !host.contains('/') => format!("ssh://{host}/{path}"),
            _ => self.remote.clone(),
        }
    }

    /// Extract a pin from a [`BuildInfo`], or report which provenance is missing.
    fn from_build_info(info: &BuildInfo, what: &str) -> Result<Self> {
        match (&info.remote, &info.commit) {
            (Some(remote), Some(commit)) => Ok(GitPin {
                remote: remote.clone(),
                commit: commit.clone(),
            }),
            _ => Err(OpenError::MissingProvenance { what: what.into() }),
        }
    }
}

/// Everything needed to generate and build a viewer for one context directory.
#[derive(Debug, Clone)]
pub struct ViewerSpec {
    /// The context directory holding the per-entity event streams.
    pub root: PathBuf,
    /// Event serialization format, detected from the on-disk streams.
    pub format: Format,
    /// Cargo package of the analyzer crate providing `Viewer` (`QuentViewer`).
    pub analyzer_package: String,
    /// Quent framework source, pinned to the build commit.
    pub quent: GitPin,
    /// Analyzer crate source, pinned to the build commit (the model's source).
    pub analyzer: GitPin,
}

impl ViewerSpec {
    /// Derive a spec from a sidecar and its context directory.
    pub fn from_artifact(root: &Path, info: &ArtifactInfo) -> Result<Self> {
        let analyzer_package =
            info.model
                .analyzer_package
                .clone()
                .ok_or_else(|| OpenError::NoAnalyzer {
                    model: info.model.name.clone(),
                })?;
        Ok(Self {
            root: root.to_path_buf(),
            format: detect_format(root)?,
            analyzer_package,
            quent: GitPin::from_build_info(&info.quent, "quent")?,
            analyzer: GitPin::from_build_info(&info.model.source, "analyzer source")?,
        })
    }

    /// Analyzer crate identifier (hyphens to underscores) for `<crate>::Viewer`
    /// in generated code.
    pub fn analyzer_crate(&self) -> String {
        self.analyzer_package.replace('-', "_")
    }

    /// Stable directory name for caching this viewer's generated crate and build,
    /// keyed on everything that affects the build output.
    pub fn cache_key(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            self.analyzer_package,
            short_commit(&self.analyzer.commit),
            short_commit(&self.quent.commit),
            self.format.extension(),
        )
    }
}

/// First 12 chars of a commit hash, for compact cache keys.
fn short_commit(commit: &str) -> &str {
    let end = commit.len().min(12);
    &commit[..end]
}

/// Detect the artifact format from an `events.<ext>` stream in any per-entity
/// subdirectory.
fn detect_format(root: &Path) -> Result<Format> {
    let entries = std::fs::read_dir(root).map_err(|source| OpenError::Sidecar {
        path: root.to_path_buf(),
        source,
    })?;
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        if let Ok(files) = std::fs::read_dir(entry.path()) {
            for file in files.flatten() {
                if let Some(ext) = Path::new(&file.file_name()).extension()
                    && let Some(format) = ext.to_str().and_then(Format::from_extension)
                {
                    return Ok(format);
                }
            }
        }
    }
    Err(OpenError::UnknownFormat {
        root: root.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quent_build_info::ModelInfo;

    fn artifact_with(analyzer_package: Option<&str>, commit: &str) -> ArtifactInfo {
        let mut model = ModelInfo::unknown();
        model.name = "Simulator".into();
        model.analyzer_package = analyzer_package.map(str::to_string);
        model.source = BuildInfo {
            remote: Some("https://example.com/analyzer".into()),
            commit: Some(commit.into()),
            ..BuildInfo::unknown()
        };
        let mut info = ArtifactInfo::new(model);
        info.quent = BuildInfo {
            remote: Some("https://example.com/quent".into()),
            commit: Some("0123456789abcdef".into()),
            ..BuildInfo::unknown()
        };
        info
    }

    fn ctx_with_stream(name: &str, file: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let entity = dir.path().join(name);
        std::fs::create_dir_all(&entity).unwrap();
        std::fs::write(entity.join(file), b"").unwrap();
        dir
    }

    #[test]
    fn detects_format_from_entity_subdir() {
        let ctx = ctx_with_stream("engine", "events.msgpack");
        assert_eq!(detect_format(ctx.path()).unwrap(), Format::Msgpack);
    }

    #[test]
    fn unknown_format_when_no_streams() {
        let ctx = ctx_with_stream("engine", "notes.txt");
        assert!(matches!(
            detect_format(ctx.path()),
            Err(OpenError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn spec_requires_analyzer_package() {
        let ctx = ctx_with_stream("engine", "events.ndjson");
        let info = artifact_with(None, "abc");
        assert!(matches!(
            ViewerSpec::from_artifact(ctx.path(), &info),
            Err(OpenError::NoAnalyzer { .. })
        ));
    }

    #[test]
    fn cargo_url_normalizes_scp_style_but_leaves_real_urls() {
        let scp = GitPin {
            remote: "git@github.com:org/repo.git".into(),
            commit: "c".into(),
        };
        assert_eq!(scp.cargo_url(), "ssh://git@github.com/org/repo.git");
        let https = GitPin {
            remote: "https://github.com/rapidsai/quent".into(),
            commit: "c".into(),
        };
        assert_eq!(https.cargo_url(), "https://github.com/rapidsai/quent");
        // A local path with a colon after a slash is not scp-style: leave it.
        let local = GitPin {
            remote: "/tmp/foo:bar.git".into(),
            commit: "c".into(),
        };
        assert_eq!(local.cargo_url(), "/tmp/foo:bar.git");
    }

    #[test]
    fn spec_derives_crate_ident_and_cache_key() {
        let ctx = ctx_with_stream("engine", "events.ndjson");
        let info = artifact_with(Some("quent-simulator-analyzer"), "feedface99887766");
        let spec = ViewerSpec::from_artifact(ctx.path(), &info).unwrap();
        assert_eq!(spec.analyzer_crate(), "quent_simulator_analyzer");
        assert_eq!(spec.format, Format::Ndjson);
        // commit is truncated to 12 chars in the key.
        assert_eq!(
            spec.cache_key(),
            "quent-simulator-analyzer-feedface9988-0123456789ab-ndjson"
        );
    }
}
