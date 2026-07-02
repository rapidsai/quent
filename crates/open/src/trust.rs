// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Trust policy for git sources `quent-open` clones, builds, and runs.
//!
//! A `model.qmi` from another artifact is attacker-controlled: opening it would
//! `cargo build` (build scripts, proc-macros, `pnpm`) and run code from the
//! named remote. Build only trusted sources: built-in defaults (the quent repo
//! and this tool's build remote), the allowlist file, `--trust`, or interactive
//! confirmation.

use std::collections::BTreeSet;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;

/// Resolves whether git remotes are trusted to build.
pub struct Trust {
    /// Canonicalized trust entries: exact repos (`github.com/rapidsai/quent`) or
    /// explicit org/prefix wildcards (`github.com/org/*`).
    allow: BTreeSet<String>,
    /// Bypass the gate entirely (`--trust-all`).
    trust_all: bool,
    /// Persistent allowlist path for appending "always" answers.
    allowlist_file: Option<PathBuf>,
}

impl Trust {
    /// Build policy from built-in defaults, persistent allowlist, and per-run
    /// `--trust` / `--trust-all`.
    pub fn new(cli_trust: &[String], trust_all: bool) -> Self {
        let mut allow = BTreeSet::new();
        // Built-in: canonical quent repo plus this tool's build remote, so
        // artifacts from your fork work.
        allow.insert("github.com/rapidsai/quent".to_string());
        if let Some(remote) = quent_build_info::quent().remote {
            allow.insert(canonicalize_remote(&remote));
        }
        let allowlist_file = allowlist_path();
        if let Some(path) = &allowlist_file
            && let Ok(contents) = std::fs::read_to_string(path)
        {
            for line in contents.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    allow.insert(canonicalize_remote(line));
                }
            }
        }
        for remote in cli_trust {
            allow.insert(canonicalize_remote(remote));
        }
        Self {
            allow,
            trust_all,
            allowlist_file,
        }
    }

    /// Whether `remote` is trusted without prompting. Plain entries match one repo
    /// exactly; only explicit `.../*` entries trust an org/prefix, avoiding
    /// accidental trust of nested repos.
    fn is_trusted(&self, remote: &str) -> bool {
        if self.trust_all {
            return true;
        }
        let canonical = canonicalize_remote(remote);
        self.allow
            .iter()
            .any(|entry| match entry.strip_suffix("/*") {
                Some(prefix) => canonical == prefix || canonical.starts_with(&format!("{prefix}/")),
                None => canonical == *entry,
            })
    }

    /// Decide whether `remote` at `commit` may be built. Trusted remotes pass;
    /// otherwise prompt interactively (`a` persists) or refuse non-interactively.
    pub fn authorize(&mut self, remote: &str, commit: &str) -> bool {
        if self.is_trusted(remote) {
            return true;
        }
        if !std::io::stdin().is_terminal() {
            return false;
        }
        match prompt(remote, commit) {
            Answer::No => false,
            Answer::Once => {
                self.allow.insert(canonicalize_remote(remote));
                true
            }
            Answer::Always => {
                self.allow.insert(canonicalize_remote(remote));
                self.persist(remote);
                true
            }
        }
    }

    /// Append a canonical remote to the persistent allowlist (best effort).
    fn persist(&self, remote: &str) {
        let Some(path) = &self.allowlist_file else {
            return;
        };
        let canonical = canonicalize_remote(remote);
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(path));
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{canonical}");
        }
    }
}

enum Answer {
    Once,
    Always,
    No,
}

/// Prompt on the terminal whether to trust an untrusted git remote. Trust is
/// per-remote (the commit is shown only as context): `[y]es` trusts it for this
/// run, `[a]lways` persists it to the allowlist.
fn prompt(remote: &str, commit: &str) -> Answer {
    eprint!(
        "Build and run code from an untrusted git remote:\n  {remote}\n  at commit {commit}\nTrust this remote? [y]es (this run) / [a]lways / [N]o: "
    );
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        return Answer::No;
    }
    match line.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Answer::Once,
        "a" | "always" => Answer::Always,
        _ => Answer::No,
    }
}

/// The persistent allowlist path, `<config_dir>/quent/open/trusted`.
fn allowlist_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("quent").join("open").join("trusted"))
}

/// Canonicalize a git remote to scheme-agnostic `host/path` so https, ssh, and
/// scp-style forms of the same repo match. Lowercase the host and strip a
/// trailing `.git`.
pub fn canonicalize_remote(remote: &str) -> String {
    // `gix-url` parses real git URLs and scp-style forms into host + path (with
    // the scheme and `user@` already split off). Bare `host/path` defaults and
    // `host/org/*` allowlist wildcards aren't URLs — gix reads them as local file
    // paths (no host) — so fall back to splitting the input on the first `/`.
    let (host, path) = match gix_url::Url::try_from(remote) {
        Ok(url) if url.host().is_some() => {
            // Keep a non-default port in the key: a different port can be a
            // different endpoint, so it must not inherit the default's trust.
            let host = match url.port {
                Some(port) => format!("{}:{port}", url.host().unwrap_or_default()),
                None => url.host().unwrap_or_default().to_string(),
            };
            (host, url.path.to_string())
        }
        _ => match remote.split_once('/') {
            Some((host, path)) => (host.to_string(), path.to_string()),
            None => (remote.to_string(), String::new()),
        },
    };
    let host = host.to_ascii_lowercase();
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    if path.is_empty() {
        host
    } else {
        format!("{host}/{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_collapses_url_forms() {
        let want = "github.com/rapidsai/quent";
        assert_eq!(
            canonicalize_remote("https://github.com/rapidsai/quent"),
            want
        );
        assert_eq!(
            canonicalize_remote("https://github.com/rapidsai/quent.git"),
            want
        );
        assert_eq!(
            canonicalize_remote("git@github.com:rapidsai/quent.git"),
            want
        );
        assert_eq!(
            canonicalize_remote("ssh://git@github.com/rapidsai/quent.git"),
            want
        );
        assert_eq!(canonicalize_remote("git@GitHub.com:rapidsai/quent"), want);
    }

    #[test]
    fn canonicalize_keeps_non_default_port() {
        // A non-default port is a distinct endpoint and must not collapse onto the
        // default-port repo's trust key.
        assert_eq!(
            canonicalize_remote("ssh://git@git.example:2222/team/repo.git"),
            "git.example:2222/team/repo"
        );
        assert_ne!(
            canonicalize_remote("ssh://git@git.example:2222/team/repo"),
            canonicalize_remote("ssh://git@git.example/team/repo")
        );
    }

    #[test]
    fn trust_matches_exact_and_explicit_wildcard() {
        let trust = Trust {
            allow: ["github.com/rapidsai/quent".into(), "github.com/me/*".into()]
                .into_iter()
                .collect(),
            trust_all: false,
            allowlist_file: None,
        };
        // Exact repo entry: matches that repo (any URL form), nothing under it.
        assert!(trust.is_trusted("https://github.com/rapidsai/quent.git"));
        assert!(!trust.is_trusted("https://github.com/rapidsai/quent-evil"));
        assert!(!trust.is_trusted("https://github.com/rapidsai/quent/sub"));
        // Explicit `/*` entry: trusts the whole org/prefix.
        assert!(trust.is_trusted("git@github.com:me/anything.git"));
        assert!(!trust.is_trusted("https://github.com/someone/else"));
    }

    #[test]
    fn trust_all_bypasses() {
        let trust = Trust {
            allow: BTreeSet::new(),
            trust_all: true,
            allowlist_file: None,
        };
        assert!(trust.is_trusted("https://anywhere.example/x/y"));
    }
}
