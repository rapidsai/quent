// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Errors returned while building browser bundles.

use std::path::PathBuf;

use uuid::Uuid;

/// An error from converting collector output or a ZIP archive into a browser bundle.
///
/// Messages include the underlying error where there is one, so `to_string()` alone is
/// informative; the same error is also available through [`std::error::Error::source`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The input is not a readable ZIP archive.
    #[error("unable to read ZIP archive: {0}")]
    Archive(#[from] zip::result::ZipError),
    /// The archive has more entries than extraction allows.
    #[error("ZIP contains too many entries: {entries} exceeds {max_entries}")]
    ArchiveTooManyEntries { entries: usize, max_entries: usize },
    /// The archive expands to more bytes than extraction allows.
    #[error("ZIP expands beyond the {max_bytes}-byte safety limit")]
    ArchiveTooLarge { max_bytes: u64 },
    /// An archive entry would be extracted outside the destination directory.
    #[error("ZIP contains unsafe path: {name}")]
    ArchiveUnsafePath { name: String },
    /// An archive entry is neither a regular file nor a directory (for example a symlink).
    #[error("ZIP contains non-regular entry: {}", path.display())]
    ArchiveNonRegularEntry { path: PathBuf },

    /// The input contains no context directories (UUID-named subdirectories).
    #[error("no UUID context directories found under {}", root.display())]
    NoContexts { root: PathBuf },
    /// Two context directories have names that parse to the same UUID (for example the
    /// hyphenated and simple forms, or differing only in case).
    #[error("context directories `{first_name}` and `{second_name}` are both UUID {context_id}")]
    DuplicateContext {
        context_id: Uuid,
        first_name: String,
        second_name: String,
    },
    /// The recording's events could not be loaded.
    #[error("unable to load recorded events: {0}")]
    LoadEvents(#[from] quent_store::Error),
    /// The recording does not contain exactly one engine.
    #[error("expected exactly one engine in browser bundle, found {found}")]
    UnexpectedEngineCount { found: usize },
    /// A context's `model.qmi` is not valid JSON.
    #[error("invalid model provenance in context {context_id}: {source}")]
    InvalidProvenance {
        context_id: Uuid,
        #[source]
        source: serde_json::Error,
    },
    /// Two contexts record different `model.qmi` provenance.
    #[error(
        "browser bundle contexts have inconsistent model provenance: \
         contexts {first_context_id} and {conflicting_context_id} differ"
    )]
    ProvenanceConflict {
        first_context_id: Uuid,
        conflicting_context_id: Uuid,
    },

    /// The events could not be serialized into the bundle payload.
    #[error("unable to encode browser bundle payload: {0}")]
    EncodePayload(#[from] postcard::Error),
    /// The bundle envelope could not be encoded.
    #[error("unable to encode browser bundle: {0}")]
    EncodeBundle(#[from] quent_browser_bundle::Error),
    /// A filesystem operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// A specialized [`Result`](std::result::Result) for browser bundle building.
pub type Result<T> = std::result::Result<T, Error>;
