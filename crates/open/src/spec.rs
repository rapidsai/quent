// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build a [`ViewerSpec`] from a context's `model.qmi`: pinned git sources,
//! analyzer package, and artifact format for generating/building a viewer.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use quent_build_info::{ArtifactInfo, BuildInfo, SIDECAR_FILE_NAME};
use walkdir::WalkDir;

use crate::error::{OpenError, Result};

/// Recursively discover context directories (those containing a `model.qmi`
/// sidecar) under the given `paths`. Hidden directories (dotfiles, e.g. `.git`)
/// are skipped and symlinks are not followed (so the walk stays cycle-safe).
/// Results are canonicalized and deduplicated, preserving discovery order.
pub fn discover_contexts(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for path in paths {
        // `WalkDir` does not descend into symlinked directories (cycle-safe);
        // `filter_entry` prunes hidden directories while keeping an explicitly-passed
        // root.
        let mut walk = WalkDir::new(path)
            .into_iter()
            .filter_entry(|entry| entry.depth() == 0 || !is_hidden(entry));
        while let Some(entry) = walk.next() {
            // Report (don't silently drop) traversal errors so a permission-denied
            // subtree can't quietly shrink the discovered set; keep walking the rest.
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    eprintln!("warning: skipping unreadable path during discovery: {error}");
                    continue;
                }
            };
            // `Path::is_dir` follows symlinks, so a context reached via a symlinked
            // argument (or directory) is still recognized, even though the walk
            // itself never descends through the link.
            if entry.path().is_dir() && entry.path().join(SIDECAR_FILE_NAME).is_file() {
                let canonical = entry.path().canonicalize()?;
                if seen.insert(canonical.clone()) {
                    found.push(canonical);
                }
                // A context is a leaf: skip its entity dirs and event streams so
                // discovery scales with the directory tree, not the payload size.
                // Skip only for directories `WalkDir` actually descends into: a
                // real directory, or the root (followed even when it is a symlink).
                // A non-root symlinked context isn't descended, and skipping it
                // would instead drop the symlink's (un-walked) siblings.
                if entry.depth() == 0 || entry.file_type().is_dir() {
                    walk.skip_current_dir();
                }
            }
        }
    }
    Ok(found)
}

/// Whether a walked entry is hidden (its file name starts with `.`).
fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| name.starts_with('.'))
}

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

/// Viewer build inputs; contexts are tracked separately because one viewer can
/// serve multiple same-spec contexts.
#[derive(Debug, Clone)]
pub struct ViewerSpec {
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

    /// Unambiguous build identity: analyzer package, format, and both git
    /// remotes + full commits. Used to group/dedup contexts into viewers.
    /// Short label distinguishing this build from other groups (package, format,
    /// and short pins) so concurrent viewers with equal context counts are
    /// still tellable apart.
    pub fn describe(&self) -> String {
        format!(
            "{} ({}, quent@{} analyzer@{})",
            self.analyzer_package,
            self.format.extension(),
            short_commit(&self.quent.commit),
            short_commit(&self.analyzer.commit),
        )
    }

    pub fn group_key(&self) -> String {
        // Key on the Cargo-normalized remotes so equivalent spellings (e.g.
        // scp-style vs `ssh://`) — which produce one dependency — share a build
        // instead of splitting into separate viewers. Unit separator between
        // fields so values can't run together.
        [
            self.analyzer_package.as_str(),
            self.format.extension(),
            self.quent.cargo_url().as_str(),
            &self.quent.commit,
            self.analyzer.cargo_url().as_str(),
            &self.analyzer.commit,
        ]
        .join("\u{1f}")
    }

    /// Filesystem-safe cache dir for this generated crate/build: readable prefix
    /// plus [`group_key`](Self::group_key) hash, so distinct builds never share a
    /// directory even when short commits or package names match.
    pub fn cache_key(&self) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.group_key().hash(&mut hasher);
        format!(
            "{}-{}-{}-{:016x}",
            self.analyzer_package,
            short_commit(&self.analyzer.commit),
            self.format.extension(),
            hasher.finish(),
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

    fn make_context(dir: &Path) {
        std::fs::create_dir_all(dir.join("engine")).unwrap();
        std::fs::write(dir.join("engine").join("events.ndjson"), b"").unwrap();
        std::fs::write(dir.join(SIDECAR_FILE_NAME), b"{}").unwrap();
    }

    #[test]
    fn discover_finds_nested_contexts_and_skips_hidden() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        make_context(&root.join("a"));
        make_context(&root.join("nested/b"));
        make_context(&root.join(".hidden/c")); // under a dotdir: must be skipped

        let found = discover_contexts(&[root.to_path_buf()]).unwrap();
        let mut names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);

        // Passing a context directory directly yields just it.
        let direct = discover_contexts(&[root.join("a")]).unwrap();
        assert_eq!(direct.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn discovers_context_through_symlinked_root() {
        let tmp = tempfile::tempdir().unwrap();
        make_context(&tmp.path().join("real"));
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(tmp.path().join("real"), &link).unwrap();

        // A symlink pointing straight at a context must still be recognized.
        let found = discover_contexts(&[link]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file_name().unwrap(), "real");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_context_does_not_hide_siblings() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        make_context(&root.join("target"));
        make_context(&root.join("sibling"));
        // A non-root symlink straight to a context must not let `skip_current_dir`
        // prune the symlink's (un-walked) siblings.
        std::os::unix::fs::symlink(root.join("target"), root.join("link")).unwrap();

        let found = discover_contexts(&[root.to_path_buf()]).unwrap();
        let mut names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        // `link` canonicalizes to `target`, so the set is {sibling, target}.
        assert_eq!(names, vec!["sibling", "target"]);
    }

    #[test]
    fn group_key_normalizes_equivalent_remotes() {
        let scp = ViewerSpec {
            format: Format::Ndjson,
            analyzer_package: "p".into(),
            quent: GitPin {
                remote: "git@github.com:org/quent.git".into(),
                commit: "c".into(),
            },
            analyzer: GitPin {
                remote: "git@github.com:org/a.git".into(),
                commit: "d".into(),
            },
        };
        let ssh = ViewerSpec {
            quent: GitPin {
                remote: "ssh://git@github.com/org/quent.git".into(),
                commit: "c".into(),
            },
            analyzer: GitPin {
                remote: "ssh://git@github.com/org/a.git".into(),
                commit: "d".into(),
            },
            ..scp.clone()
        };
        assert_eq!(scp.group_key(), ssh.group_key());
        assert_eq!(scp.cache_key(), ssh.cache_key());
    }

    #[cfg(unix)]
    #[test]
    fn discovery_does_not_follow_symlink_cycles() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        make_context(&root.join("a"));
        // A symlink back to the root would loop a naive recursive walk.
        std::os::unix::fs::symlink(root, root.join("loop")).unwrap();

        let found = discover_contexts(&[root.to_path_buf()]).unwrap(); // must terminate
        assert_eq!(found.len(), 1);
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
    fn spec_derives_crate_ident_and_keys() {
        let ctx = ctx_with_stream("engine", "events.ndjson");
        let info = artifact_with(Some("quent-simulator-analyzer"), "feedface99887766");
        let spec = ViewerSpec::from_artifact(ctx.path(), &info).unwrap();
        assert_eq!(spec.analyzer_crate(), "quent_simulator_analyzer");
        assert_eq!(spec.format, Format::Ndjson);
        assert!(
            spec.cache_key()
                .starts_with("quent-simulator-analyzer-feedface9988-ndjson-")
        );
    }

    #[test]
    fn keys_distinguish_full_pins_not_just_short_commit() {
        let ctx = ctx_with_stream("engine", "events.ndjson");
        // Same package, format, and 12-char commit prefix, but different full
        // analyzer commits — must NOT collide.
        let a =
            ViewerSpec::from_artifact(ctx.path(), &artifact_with(Some("p"), "abcabcabcabc1111"))
                .unwrap();
        let b =
            ViewerSpec::from_artifact(ctx.path(), &artifact_with(Some("p"), "abcabcabcabc2222"))
                .unwrap();
        assert_ne!(a.group_key(), b.group_key());
        assert_ne!(a.cache_key(), b.cache_key());
        // Identical inputs group together and are deterministic.
        let a2 =
            ViewerSpec::from_artifact(ctx.path(), &artifact_with(Some("p"), "abcabcabcabc1111"))
                .unwrap();
        assert_eq!(a.group_key(), a2.group_key());
        assert_eq!(a.cache_key(), a2.cache_key());
    }
}
