// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Basic traits for exporter / importer implementations

use quent_events::{EntityEvent, Event};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExporterError {
    /// Push was called after [`Exporter::shutdown`].
    #[error("exporter has been shut down")]
    Shutdown,
    /// Any failure originating in the exporter implementation.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl ExporterError {
    /// Wrap an implementation-specific error as [`ExporterError::Other`].
    pub fn other<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Other(Box::new(error))
    }
}

impl From<std::io::Error> for ExporterError {
    fn from(error: std::io::Error) -> Self {
        Self::other(error)
    }
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

/// A sink for one entity's event stream.
#[async_trait::async_trait]
pub trait Exporter<T>: Send
where
    T: Serialize + Send + EntityEvent,
{
    /// Export one event.
    async fn push(&mut self, event: Event<T>) -> ExporterResult<()>;

    /// Make a best-effort to flush any buffered events, then release any internal resources.
    ///
    /// Calling [`Self::push`] will result in an error after calling this.
    async fn shutdown(&mut self) -> ExporterResult<()>;
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
