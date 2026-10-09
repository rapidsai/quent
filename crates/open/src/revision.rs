// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Resolve compatibility capabilities from pinned revision ancestry and trees.

use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

use crate::error::{OpenError, Result};
use crate::spec::GitPin;

const CANDIDATE_REF: &str = "refs/quent-open/candidate";
const SOURCE_REMOTE: &str = "quent-open-source";

/// A pinned revision fetched into a local bare repository for compatibility checks.
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
        let remote_exists = Command::new("git")
            .arg("--git-dir")
            .arg(repository)
            .args(["remote", "get-url", SOURCE_REMOTE])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|source| OpenError::Spawn {
                what: "inspect revision cache remote".into(),
                source,
            })?
            .success();
        let operation = if remote_exists { "set-url" } else { "add" };
        run_git(
            Command::new("git")
                .arg("--git-dir")
                .arg(repository)
                .args(["remote", operation, SOURCE_REMOTE])
                .arg(&remote),
            "configure revision cache remote",
        )
        .await?;
        run_git(
            Command::new("git").arg("--git-dir").arg(repository).args([
                "config",
                &format!("remote.{SOURCE_REMOTE}.promisor"),
                "true",
            ]),
            "configure revision cache promisor",
        )
        .await?;
        run_git(
            Command::new("git").arg("--git-dir").arg(repository).args([
                "config",
                &format!("remote.{SOURCE_REMOTE}.partialclonefilter"),
                "blob:none",
            ]),
            "configure revision cache filter",
        )
        .await?;
        run_git(
            Command::new("git")
                .arg("--git-dir")
                .arg(repository)
                // Package detection needs the pinned tree but not source blobs.
                .args(["fetch", "--force", "--no-tags", "--filter=blob:none"])
                .arg(SOURCE_REMOTE)
                .arg(format!("+{}:{CANDIDATE_REF}", pin.commit)),
            "fetch pinned revision",
        )
        .await?;

        Ok(Self { repository })
    }

    /// Return whether `path` exists in the exact pinned revision tree.
    pub async fn contains_path(&self, path: &str) -> Result<bool> {
        let output = Command::new("git")
            .arg("--git-dir")
            .arg(self.repository)
            .args(["ls-tree", "--name-only", CANDIDATE_REF, "--"])
            .arg(path)
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|source| OpenError::Spawn {
                what: "git ls-tree".into(),
                source,
            })?;
        if !output.status.success() {
            return Err(OpenError::Revision {
                operation: "inspect pinned revision tree".into(),
                status: command_status(&output),
            });
        }
        Ok(!output.stdout.is_empty())
    }

    /// Read a UTF-8 file from the exact pinned revision.
    pub async fn read_file(&self, path: &str) -> Result<String> {
        let output = Command::new("git")
            .arg("--git-dir")
            .arg(self.repository)
            .args(["show", &format!("{CANDIDATE_REF}:{path}")])
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|source| OpenError::Spawn {
                what: "git show".into(),
                source,
            })?;
        if !output.status.success() {
            return Err(OpenError::Revision {
                operation: format!("read `{path}` from pinned revision"),
                status: command_status(&output),
            });
        }
        String::from_utf8(output.stdout).map_err(|error| OpenError::Revision {
            operation: format!("decode `{path}` from pinned revision"),
            status: error.to_string(),
        })
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
        assert!(revision.contains_path("boundary").await.unwrap());
        assert!(revision.contains_path("descendant").await.unwrap());
        assert!(!revision.contains_path("missing").await.unwrap());
        assert_eq!(revision.read_file("boundary").await.unwrap(), "boundary");

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
