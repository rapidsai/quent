// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Basic traits for exporter / importer implementations

use quent_events::Event;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExporterError {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("flush error: {0}")]
    Flush(String),
    #[error("serde error: {0}")]
    Serde(String),
    #[error("collector error: {0}")]
    Collector(String),
}

#[derive(Error, Debug)]
pub enum ImporterError {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type ExporterResult<T> = std::result::Result<T, ExporterError>;
pub type ImporterResult<T> = std::result::Result<T, ImporterError>;

/// Resolve the file an importer should read. If `path` is a directory, returns
/// the single file in it whose extension is `ext`; otherwise returns `path`
/// unchanged.
///
/// # Errors
/// Returns [`ImporterError::IoError`] if the directory cannot be read or
/// contains no file with extension `ext`.
pub fn resolve_import_path(
    path: &std::path::Path,
    ext: &str,
) -> ImporterResult<std::path::PathBuf> {
    if !path.is_dir() {
        return Ok(path.to_path_buf());
    }
    for entry in std::fs::read_dir(path)? {
        let candidate = entry?.path();
        if candidate.is_file() && candidate.extension().and_then(|e| e.to_str()) == Some(ext) {
            return Ok(candidate);
        }
    }
    Err(ImporterError::IoError(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("no .{ext} file found in directory {}", path.display()),
    )))
}

#[async_trait::async_trait]
pub trait Exporter<T>: Send + Sync
where
    T: Serialize + Send,
{
    async fn push(&self, event: Event<T>) -> ExporterResult<()>;
    async fn force_flush(&self) -> ExporterResult<()>;
}

pub trait Importer<T>: Iterator<Item = Event<T>>
where
    T: for<'de> Deserialize<'de>,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_import_path_handles_dir_and_file() {
        let dir = std::env::temp_dir().join("quent_resolve_import_path_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("019abcdef.ndjson");
        fs::write(&file, b"{}\n").unwrap();

        // A directory resolves to the contained file with the matching extension.
        assert_eq!(resolve_import_path(&dir, "ndjson").unwrap(), file);
        // A direct file path is returned unchanged.
        assert_eq!(resolve_import_path(&file, "ndjson").unwrap(), file);
        // No file with the requested extension is an error.
        assert!(resolve_import_path(&dir, "msgpack").is_err());

        fs::remove_dir_all(&dir).unwrap();
    }
}
