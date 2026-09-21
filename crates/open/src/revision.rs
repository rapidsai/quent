// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Resolve compatibility capabilities from the ancestry of pinned revisions.

use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

use crate::error::{OpenError, Result};
use crate::spec::GitPin;

const CANDIDATE_REF: &str = "refs/quent-open/candidate";

/// A pinned revision fetched into a local bare repository for ancestry checks.
pub struct PinnedRevision<'a> {
    repository: &'a Path,
}

impl<'a> PinnedRevision<'a> {
    /// Fetch `pin` into `repository` without checking out its file contents.
    pub async fn fetch(repository: &'a Path, pin: &GitPin) -> Result<Self> {
        tokio::fs::create_dir_all(repository).await?;
        if !repository.join("HEAD").is_file() {
            run_git(
                Command::new("git")
                    .arg("init")
                    .arg("--bare")
                    .arg(repository),
                "initialize revision cache",
            )
            .await?;
        }

        let remote = pin.cargo_url();
        run_git(
            Command::new("git")
                .arg("--git-dir")
                .arg(repository)
                .args(["fetch", "--force", "--no-tags", "--filter=tree:0"])
                .arg(&remote)
                .arg(format!("+{}:{CANDIDATE_REF}", pin.commit)),
            "fetch pinned revision",
        )
        .await?;

        Ok(Self { repository })
    }

    /// Return whether `boundary` is an ancestor of this pinned revision.
    pub async fn contains(&self, boundary: &str) -> Result<bool> {
        let boundary_exists = Command::new("git")
            .arg("--git-dir")
            .arg(self.repository)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(format!("{boundary}^{{commit}}"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|source| OpenError::Spawn {
                what: "git rev-parse".into(),
                source,
            })?;
        if !boundary_exists.success() {
            return Ok(false);
        }

        let output = Command::new("git")
            .arg("--git-dir")
            .arg(self.repository)
            .args(["merge-base", "--is-ancestor", boundary, CANDIDATE_REF])
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|source| OpenError::Spawn {
                what: "git merge-base".into(),
                source,
            })?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(OpenError::Revision {
                operation: "check revision ancestry".into(),
                status: command_status(&output),
            }),
        }
    }

    /// Return whether `boundary` is a proper ancestor of the pinned revision.
    pub async fn is_strict_descendant_of(&self, boundary: &str) -> Result<bool> {
        if !self.contains(boundary).await? {
            return Ok(false);
        }
        let output = Command::new("git")
            .arg("--git-dir")
            .arg(self.repository)
            .args(["merge-base", "--is-ancestor", CANDIDATE_REF, boundary])
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|source| OpenError::Spawn {
                what: "git merge-base".into(),
                source,
            })?;
        match output.status.code() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(OpenError::Revision {
                operation: "check strict revision ancestry".into(),
                status: command_status(&output),
            }),
        }
    }
}

async fn run_git(command: &mut Command, operation: &str) -> Result<()> {
    let output = command
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|source| OpenError::Spawn {
            what: operation.into(),
            source,
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(OpenError::Revision {
            operation: operation.into(),
            status: command_status(&output),
        })
    }
}

fn command_status(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if stderr.is_empty() {
        output.status.to_string()
    } else {
        format!("{}: {stderr}", output.status)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;

    use super::*;

    fn git(repository: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn commit(repository: &Path, name: &str) -> String {
        std::fs::write(repository.join(name), name).unwrap();
        git(repository, &["add", name]);
        git(repository, &["commit", "-m", name]);
        git(repository, &["rev-parse", "HEAD"])
    }

    #[tokio::test]
    async fn checks_ancestor_and_divergent_revisions() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir(&source).unwrap();
        git(&source, &["init"]);
        git(&source, &["config", "user.name", "Quent Test"]);
        git(&source, &["config", "user.email", "quent@example.com"]);

        let boundary = commit(&source, "boundary");
        let descendant = commit(&source, "descendant");
        git(&source, &["checkout", "--orphan", "divergent"]);
        git(&source, &["rm", "-rf", "."]);
        let divergent = commit(&source, "divergent");

        let cache = temp.path().join("revision.git");
        let descendant_pin = GitPin {
            remote: source.display().to_string(),
            commit: descendant,
        };
        let revision = PinnedRevision::fetch(&cache, &descendant_pin)
            .await
            .unwrap();
        assert!(revision.contains(&boundary).await.unwrap());
        assert!(revision.is_strict_descendant_of(&boundary).await.unwrap());

        let boundary_pin = GitPin {
            remote: source.display().to_string(),
            commit: boundary.clone(),
        };
        let revision = PinnedRevision::fetch(&cache, &boundary_pin).await.unwrap();
        assert!(!revision.is_strict_descendant_of(&boundary).await.unwrap());

        let divergent_pin = GitPin {
            remote: source.display().to_string(),
            commit: divergent,
        };
        let revision = PinnedRevision::fetch(&cache, &divergent_pin).await.unwrap();
        assert!(!revision.contains(&boundary).await.unwrap());
        assert!(!revision.is_strict_descendant_of(&boundary).await.unwrap());
        assert!(
            !revision
                .is_strict_descendant_of("0000000000000000000000000000000000000000")
                .await
                .unwrap()
        );
    }
}
