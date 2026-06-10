// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Git capture shared between this crate's `build.rs` (via `include!`) and the
// `emit_source` build-script helper (via `mod git`). Kept dependency-free and
// free of `//!` inner-doc comments so it is valid in both contexts.

use std::path::Path;
use std::process::Command;

/// Raw git fields captured from a working tree, with best-effort fallbacks.
pub struct RawGit {
    pub commit: String,
    pub branch: String,
    pub dirty: bool,
    pub remote: String,
    pub built_at: String,
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

/// Capture git provenance for the working tree containing `dir`. Missing values
/// degrade to `"unknown"` (or `false` for `dirty`) so a build never fails when
/// the source is not a git checkout (e.g. a `.git`-less cargo git dependency).
pub fn capture(dir: &Path) -> RawGit {
    let unknown = || "unknown".to_string();
    RawGit {
        commit: run(dir, &["rev-parse", "HEAD"]).unwrap_or_else(unknown),
        branch: run(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(unknown),
        dirty: run(dir, &["status", "--porcelain"])
            .map(|status| !status.is_empty())
            .unwrap_or(false),
        remote: run(dir, &["remote", "get-url", "origin"]).unwrap_or_else(unknown),
        built_at: run(dir, &["log", "-1", "--format=%cI"]).unwrap_or_else(unknown),
        git_dir: run(dir, &["rev-parse", "--absolute-git-dir"]),
    }
}

/// Emit `cargo:rustc-env={prefix}_*` for each captured field plus
/// `rerun-if-changed` triggers on the git HEAD/index. Called from build scripts.
pub fn emit(prefix: &str, dir: &Path) {
    let git = capture(dir);
    println!("cargo:rustc-env={prefix}_COMMIT={}", git.commit);
    println!("cargo:rustc-env={prefix}_BRANCH={}", git.branch);
    println!("cargo:rustc-env={prefix}_DIRTY={}", git.dirty);
    println!("cargo:rustc-env={prefix}_REMOTE={}", git.remote);
    println!("cargo:rustc-env={prefix}_BUILT_AT={}", git.built_at);
    if let Some(git_dir) = &git.git_dir {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/index");
    }
}
