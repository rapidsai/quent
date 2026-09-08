// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Basic traits for exporter / importer implementations

use quent_events::Event;
use std::num::NonZeroUsize;
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

/// A sink for one entity's event stream.
///
/// This is `Send` because generated application contexts first attempt to
/// create all exporters (asynchronously). If any errors occur, these are
/// immediately surfaced through the blocking context creation API. If no errors
/// occur, the exporters are then moved into their respective forwarder tasks.
#[async_trait::async_trait]
pub trait Exporter<T>: Send {
    /// Export one event.
    async fn push(&mut self, event: Event<T>) -> ExporterResult<()>;

    /// Suggested max events per [`drain_events`](Self::drain_events) batch.
    ///
    /// Callers may pass fewer in [`drain_events`], or ignore it completely.
    fn batch_size_hint(&self) -> NonZeroUsize {
        const { NonZeroUsize::new(256).unwrap() }
    }

    /// Exports every event in `events`, in order, leaving it empty.
    ///
    /// The default forwards each event to [`push`](Self::push), logging and
    /// skipping any that fail so one failure does not drop the rest of the
    /// batch. Override this to amortize per-event overhead.
    async fn drain_events(&mut self, events: &mut Vec<Event<T>>) -> ExporterResult<()>
    where
        T: Send + 'async_trait,
    {
        for event in events.drain(..) {
            if let Err(e) = self.push(event).await {
                warn!("unable to export event: {e}");
            }
        }
        Ok(())
    }

    /// Make a best-effort to flush any buffered events.
    async fn shutdown(self: Box<Self>) -> ExporterResult<()>;
}

/// Provides an exporter instance for `T` bound to `context_id` (the id of the
/// context whose events it exports). Backends that do not scope by context, such
/// as a callback, ignore it.
#[async_trait::async_trait]
pub trait ExporterProvider<T>: Send + Sync {
    async fn create_exporter(&self, context_id: Uuid) -> ExporterResult<Box<dyn Exporter<T>>>;
}

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

/// Result of exporters.
pub type ExporterResult<T> = std::result::Result<T, ExporterError>;

#[derive(Debug, Error)]
pub enum ImporterError {
    /// Any failure originating in the importer implementation.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl ImporterError {
    /// Wrap an implementation-specific error as [`ImporterError::Other`].
    pub fn other<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Other(Box::new(error))
    }
}

impl From<std::io::Error> for ImporterError {
    fn from(error: std::io::Error) -> Self {
        Self::other(error)
    }
}

/// Maximum supported payload size for length-prefixed importer frames.
pub const MAX_FRAME_SIZE_BYTES: usize = 64 * 1024 * 1024;

/// Result type for importers.
pub type ImporterResult<T> = std::result::Result<T, ImporterError>;

/// A source of one entity's events.
pub trait Importer<T>: Iterator<Item = ImporterResult<Event<T>>> {}

/// Provides an importer instance for `T`.
pub trait ImporterProvider<T> {
    fn create_importer(&self) -> ImporterResult<Box<dyn Importer<T>>>;
}

/// Resolve the file an importer should read. If `path` is a directory, returns
/// the single file in it whose extension is `ext`; otherwise returns `path`
/// unchanged.
///
/// # Errors
///
/// Returns an error if the directory cannot be read or contains no file with
/// extension `ext`.
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
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("no .{ext} file found in directory {}", path.display()),
    )
    .into())
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
