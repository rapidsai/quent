// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Git capture shared between this crate's `build.rs` (via `include!`) and the
// `emit_source` build-script helper (via `mod git`). Kept dependency-free and
// free of `//!` inner-doc comments so it is valid in both contexts.

use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

/// Raw git fields captured from a working tree. A field is `None` when it could
/// not be determined (e.g. the source is not a git checkout), so the build never
/// fails and absent provenance stays distinguishable from a real value.
pub struct RawGit {
    pub commit: Option<String>,
    pub branch: Option<String>,
    pub dirty: Option<bool>,
    pub remote: Option<String>,
    pub built_at: Option<String>,
    /// Absolute path to the `.git` directory, if this is a git working tree.
    pub git_dir: Option<String>,
}

fn run(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

// Strip userinfo (a possible embedded token/password) from http(s) remote URLs
// so it is never baked into provenance and leaked via an exported sidecar. ssh
// and scp-style URLs keep their login user, which is not a secret.
fn sanitize_remote(url: String) -> String {
    for scheme in ["https://", "http://"] {
        if let Some(rest) = url.strip_prefix(scheme) {
            let authority_end = rest.find('/').unwrap_or(rest.len());
            if let Some(at) = rest[..authority_end].find('@') {
                return format!("{scheme}{}", &rest[at + 1..]);
            }
            return url;
        }
    }
    url
}

// Prefer the real `origin` so a build's provenance reflects where it was
// actually built from (including a fork). Fall back to the Cargo package
// repository only when `origin` is absent or is a cargo git-cache mirror
// (`file://…/.cargo/git/db/…`), which is useless for locating the source.
fn select_remote(origin: Option<&str>, package_repository: Option<&str>) -> Option<String> {
    let package_repository = package_repository
        .map(str::trim)
        .filter(|repository| !repository.is_empty());

    origin
        .filter(|url| !is_cargo_git_file_remote(url))
        .or(package_repository)
        .map(|url| sanitize_remote(url.to_string()))
}

fn has_component_sequence(path: &Path, sequence: &[&str]) -> bool {
    let components = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect::<Vec<_>>();

    components.windows(sequence.len()).any(|window| {
        window
            .iter()
            .zip(sequence.iter())
            .all(|(component, expected)| *component == OsStr::new(*expected))
    })
}

fn is_cargo_home_git_path(path: &Path, leaf: &str) -> bool {
    if let Some(cargo_home) = std::env::var_os("CARGO_HOME") {
        let cache_path = std::path::PathBuf::from(cargo_home).join("git").join(leaf);
        if path.starts_with(cache_path) {
            return true;
        }
    }

    has_component_sequence(path, &[".cargo", "git", leaf])
}

fn is_cargo_git_checkout_path(path: &Path) -> bool {
    is_cargo_home_git_path(path, "checkouts")
}

fn is_cargo_git_db_path(path: &Path) -> bool {
    is_cargo_home_git_path(path, "db")
}

fn is_cargo_git_file_remote(url: &str) -> bool {
    url.strip_prefix("file://")
        .map(Path::new)
        .is_some_and(is_cargo_git_db_path)
}

fn is_cargo_git_source(dir: &Path, origin: Option<&str>, git_dir: Option<&str>) -> bool {
    is_cargo_git_checkout_path(dir)
        || origin.is_some_and(is_cargo_git_file_remote)
        || git_dir
            .map(Path::new)
            .is_some_and(is_cargo_git_checkout_path)
}

/// Capture git provenance for the working tree containing `dir`. Each field is
/// `None` when unavailable.
pub fn capture(dir: &Path) -> RawGit {
    capture_with_package_repository(dir, None)
}

/// Capture git provenance, preferring a Cargo package repository when provided.
pub fn capture_with_package_repository(dir: &Path, package_repository: Option<&str>) -> RawGit {
    let origin = run(dir, &["remote", "get-url", "origin"]);
    let git_dir = run(dir, &["rev-parse", "--absolute-git-dir"]);
    let dirty = if is_cargo_git_source(dir, origin.as_deref(), git_dir.as_deref()) {
        None
    } else {
        run(dir, &["status", "--porcelain"]).map(|status| !status.is_empty())
    };

    RawGit {
        commit: run(dir, &["rev-parse", "HEAD"]),
        branch: run(dir, &["rev-parse", "--abbrev-ref", "HEAD"]),
        dirty,
        remote: select_remote(origin.as_deref(), package_repository),
        built_at: run(dir, &["log", "-1", "--format=%cI"]),
        git_dir,
    }
}

/// Emit `cargo:rustc-env={prefix}_*` for each captured field plus
/// `rerun-if-changed` triggers. Only known fields are emitted, so `option_env!`
/// resolves to `None` for absent values. Called from build scripts.
pub fn emit(prefix: &str, dir: &Path) {
    let git = capture(dir);
    emit_raw(prefix, &git);
}

/// Emit provenance, preferring a Cargo package repository when provided.
#[allow(dead_code)]
pub fn emit_with_package_repository(prefix: &str, dir: &Path, package_repository: Option<&str>) {
    let git = capture_with_package_repository(dir, package_repository);
    emit_raw(prefix, &git);
}

fn emit_raw(prefix: &str, git: &RawGit) {
    if let Some(commit) = &git.commit {
        println!("cargo:rustc-env={prefix}_COMMIT={commit}");
    }
    if let Some(branch) = &git.branch {
        println!("cargo:rustc-env={prefix}_BRANCH={branch}");
    }
    if let Some(dirty) = git.dirty {
        println!("cargo:rustc-env={prefix}_DIRTY={dirty}");
    }
    if let Some(remote) = &git.remote {
        println!("cargo:rustc-env={prefix}_REMOTE={remote}");
    }
    if let Some(built_at) = &git.built_at {
        println!("cargo:rustc-env={prefix}_BUILT_AT={built_at}");
    }
    if let Some(git_dir) = &git.git_dir {
        // Rerun when the checked-out commit / branch changes. `dirty` tracking is
        // best-effort: an unstaged edit to a tracked file touches none of these,
        // so a stale `dirty=false` is possible until the next ref/index change.
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/index");
        println!("cargo:rerun-if-changed={git_dir}/packed-refs");
        if let Some(branch) = &git.branch
            && branch != "HEAD"
        {
            println!("cargo:rerun-if-changed={git_dir}/refs/heads/{branch}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        is_cargo_git_checkout_path, is_cargo_git_file_remote, is_cargo_git_source, sanitize_remote,
        select_remote,
    };

    #[test]
    fn sanitize_remote_strips_http_userinfo_only() {
        // https/http: userinfo (which may carry a token/password) is removed.
        assert_eq!(
            sanitize_remote("https://ghp_secret@github.com/o/r.git".to_string()),
            "https://github.com/o/r.git"
        );
        assert_eq!(
            sanitize_remote("https://user:pass@host:443/o/r.git".to_string()),
            "https://host:443/o/r.git"
        );
        // No userinfo: unchanged.
        assert_eq!(
            sanitize_remote("https://github.com/o/r.git".to_string()),
            "https://github.com/o/r.git"
        );
        // ssh/scp: login user is not a secret and is required to clone, so kept.
        assert_eq!(
            sanitize_remote("git@github.com:o/r.git".to_string()),
            "git@github.com:o/r.git"
        );
        assert_eq!(
            sanitize_remote("ssh://git@host/o/r.git".to_string()),
            "ssh://git@host/o/r.git"
        );
    }

    #[test]
    fn select_remote_prefers_real_origin_over_package_repository() {
        // A real `origin` (even a fork) wins, so provenance reflects where the
        // build actually came from.
        assert_eq!(
            select_remote(
                Some("git@github.com:me/quent-fork.git"),
                Some("https://github.com/rapidsai/quent")
            ),
            Some("git@github.com:me/quent-fork.git".to_string())
        );
        // A cargo git-cache `file://` mirror is useless, so fall back to the
        // package repository (with any embedded token stripped).
        assert_eq!(
            select_remote(
                Some("file:///home/me/.cargo/git/db/quent-515d44f958e14372"),
                Some("https://token@github.com/rapidsai/quent")
            ),
            Some("https://github.com/rapidsai/quent".to_string())
        );
        // No package repository: keep the real `origin`.
        assert_eq!(
            select_remote(Some("git@github.com:rapidsai/quent.git"), Some("  ")),
            Some("git@github.com:rapidsai/quent.git".to_string())
        );
        // A cargo mirror with no package repository leaves nothing usable.
        assert_eq!(
            select_remote(
                Some("file:///home/me/.cargo/git/db/quent-515d44f958e14372"),
                None
            ),
            None
        );
    }

    #[test]
    fn cargo_git_cache_sources_are_detected() {
        assert!(is_cargo_git_checkout_path(Path::new(
            "/home/me/.cargo/git/checkouts/quent-515d44f958e14372/90e7ae0/crates/build-info"
        )));
        assert!(is_cargo_git_file_remote(
            "file:///home/me/.cargo/git/db/quent-515d44f958e14372"
        ));
        assert!(is_cargo_git_source(
            Path::new("/tmp/quent/crates/build-info"),
            Some("file:///home/me/.cargo/git/db/quent-515d44f958e14372"),
            None
        ));
        assert!(!is_cargo_git_file_remote("file:///tmp/quent.git"));
    }
}
