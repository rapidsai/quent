// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Trust policy for the git sources `quent-open` clones, builds, and runs.
//!
//! A `model.qmi` from someone else's artifact is attacker-controlled, and opening
//! it would `cargo build` (build scripts, proc-macros, `pnpm`) and run code from
//! the remote it names. So a source is built only when it is trusted: a built-in
//! default (the quent repo, and the remote this tool was built from), an entry in
//! the allowlist file, a `--trust` flag, or an interactive confirmation.

use std::collections::BTreeSet;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;

/// Resolves whether git remotes are trusted to build.
pub struct Trust {
    /// Canonicalized trusted entries: an exact repo (`github.com/rapidsai/quent`)
    /// or an explicit prefix (`github.com/org/*`) to trust a whole org.
    allow: BTreeSet<String>,
    /// Bypass the gate entirely (`--trust-all`).
    trust_all: bool,
    /// Path of the persistent allowlist, for the "always" answer to append to.
    allowlist_file: Option<PathBuf>,
}

impl Trust {
    /// Build the policy from the built-in defaults, the persistent allowlist file,
    /// and the per-run `--trust` remotes / `--trust-all` flag.
    pub fn new(cli_trust: &[String], trust_all: bool) -> Self {
        let mut allow = BTreeSet::new();
        // Built-in: the canonical quent repo and the remote this tool was built
        // from (so opening your own artifacts built from your fork just works).
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

    /// Whether `remote` is already trusted (without prompting). A plain entry
    /// matches one repository exactly; only an explicit `…/*` entry trusts a whole
    /// org/prefix (so a repo entry can't accidentally trust a different repo under
    /// a nested group).
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

    /// Decide whether `remote` (recorded at `commit`) may be built. Trusted remotes
    /// pass silently; otherwise prompt on an interactive terminal (`a` persists to
    /// the allowlist), or refuse when non-interactive.
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

/// Canonicalize a git remote to a scheme-agnostic `host/path` so the https, ssh,
/// and scp-style forms of the same repo compare equal. Strips a leading scheme or
/// `user@`, a trailing `.git`, and lowercases the host.
pub fn canonicalize_remote(remote: &str) -> String {
    // Drop the scheme, if any.
    let rest = remote.split_once("://").map(|(_, r)| r).unwrap_or(remote);
    // For scp-style `user@host:path`, turn the first `:` into `/`.
    let rest = if !remote.contains("://") {
        match rest.split_once(':') {
            Some((host, path)) if !host.contains('/') => {
                return canonicalize_host_path(&format!("{host}/{path}"));
            }
            _ => rest,
        }
    } else {
        rest
    };
    canonicalize_host_path(rest)
}

fn canonicalize_host_path(host_path: &str) -> String {
    // Strip any `user@` in the host segment, a trailing `.git`, and a leading `/`.
    let host_path = host_path.trim_start_matches('/');
    let (host, path) = match host_path.split_once('/') {
        Some((h, p)) => (h, p),
        None => (host_path, ""),
    };
    let host = host.rsplit('@').next().unwrap_or(host).to_ascii_lowercase();
    let path = path
        .trim_end_matches('/')
        .strip_suffix(".git")
        .unwrap_or(path.trim_end_matches('/'));
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
