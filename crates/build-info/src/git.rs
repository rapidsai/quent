// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Git capture shared between this crate's `build.rs` (via `include!`) and the
// `emit_source` build-script helper (via `mod git`). Kept dependency-free and
// free of `//!` inner-doc comments so it is valid in both contexts.

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

/// Capture git provenance for the working tree containing `dir`. Each field is
/// `None` when unavailable.
pub fn capture(dir: &Path) -> RawGit {
    RawGit {
        commit: run(dir, &["rev-parse", "HEAD"]),
        branch: run(dir, &["rev-parse", "--abbrev-ref", "HEAD"]),
        dirty: run(dir, &["status", "--porcelain"]).map(|status| !status.is_empty()),
        remote: run(dir, &["remote", "get-url", "origin"]).map(sanitize_remote),
        built_at: run(dir, &["log", "-1", "--format=%cI"]),
        git_dir: run(dir, &["rev-parse", "--absolute-git-dir"]),
    }
}

/// Emit `cargo:rustc-env={prefix}_*` for each captured field plus
/// `rerun-if-changed` triggers. Only known fields are emitted, so `option_env!`
/// resolves to `None` for absent values. Called from build scripts.
pub fn emit(prefix: &str, dir: &Path) {
    let git = capture(dir);
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
    use super::sanitize_remote;

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
}
