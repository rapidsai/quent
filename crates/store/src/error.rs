// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Errors returned by store operations.

use std::path::PathBuf;

use uuid::Uuid;

/// An error from selecting contexts, storing entities, or loading events.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("entity {entity_id} has multiple occurrences of once-event `{event_name}`")]
    DuplicateOnceEvent {
        entity_id: Uuid,
        event_name: &'static str,
    },
    #[error("at least one context is required")]
    EmptyContextSet,
    #[error("entity {0} is not in this store")]
    MissingEntity(Uuid),
    #[error("context `{0}` was not found")]
    ContextNotFound(Uuid),
    #[error("context path `{0}` is not a directory")]
    ContextNotDirectory(PathBuf),
    #[error("context model `{actual}` does not match expected model `{expected}`")]
    ModelMismatch { expected: String, actual: String },
    #[error("event file `{path}` requires the `{feature}` feature for `{format}` data")]
    DisabledFormat {
        path: PathBuf,
        format: String,
        feature: &'static str,
    },
    #[error("failed to {operation} `{path}`: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to import events from `{path}`: {source}")]
    Importer {
        path: PathBuf,
        #[source]
        source: quent_io::ImporterError,
    },
}
