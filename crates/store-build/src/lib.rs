// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates schema-based typed APIs for retrieving stored events.
//!
//! Add `quent-store-build` to `[build-dependencies]`, call [`generate`] from
//! `build.rs`, and include the generated file from Cargo's `OUT_DIR`.
//! The crate including that source needs normal dependencies on `quent-store`,
//! serde-enabled `quent-events`, and derive-enabled `serde`.
//!
//! ```ignore
//! // build.rs
//! use quent_store_build::{Options, generate};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let schema = todo!("load a quent_schema::Schema");
//!     generate(&schema, &Options::default())?;
//!     Ok(())
//! }
//! ```
//!
//! ```ignore
//! // src/lib.rs
//! mod model {
//!     include!(concat!(env!("OUT_DIR"), "/demo.rs"));
//! }
//! ```
//!
//! ```toml
//! [build-dependencies]
//! quent-store-build = { path = "../quent/crates/store-build" }
//!
//! [dependencies]
//! quent-events = { path = "../quent/crates/events", features = ["serde"] }
//! quent-store = { path = "../quent/crates/store" }
//! serde = { version = "1", features = ["derive"] }
//! ```

use std::path::PathBuf;

use quent_schema::Schema;
use quote::quote;

mod entities;
mod event_loaders;

/// Options controlling stored-event retrieval source generation.
///
/// Generated event and record types always derive `serde::Serialize` and
/// `serde::Deserialize`.
pub struct Options {
    /// Derive [`Debug`](std::fmt::Debug) on generated event and record types.
    pub debug: bool,

    /// Additional derives applied to every generated event payload enum.
    pub event_derives: &'static [&'static str],

    /// Additional derives applied to every generated record struct.
    pub record_derives: &'static [&'static str],

    /// Generate a model-wide combined event.
    pub combined_event: bool,

    /// Generate model-wide filesystem loading support.
    ///
    /// [`Self::combined_event`] must also be enabled. The consuming crate must enable at least one
    /// `quent-store` `io-*` feature.
    pub filesystem: bool,

    /// Generate event-specific payloads, access traits, and consuming native storage.
    ///
    /// Types are emitted under `entity_events::<namespace>::<entity>`.
    /// An entity's sole `Once` event is borrowed directly; other `Once` events
    /// return `Option`, and `Multi` events return an iterator.
    /// Event names `id`, `type_name`, `properties`, and `event_storage` are reserved.
    pub entity_events: bool,

    /// Directory the generated file is written into.
    pub out_dir: PathBuf,

    /// File name to write; defaults to the lowercase schema name with a `.rs` extension.
    pub file_name: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            debug: true,
            event_derives: Default::default(),
            record_derives: Default::default(),
            combined_event: true,
            filesystem: true,
            entity_events: false,
            out_dir: PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()),
            file_name: None,
        }
    }
}

/// An error from generating stored-event retrieval source.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("filesystem loading requires combined-event generation")]
    FilesystemRequiresCombinedEvent,
    #[error("generated entity-event name `{name}` conflicts between {first} and {second}")]
    EntityEventsNameConflict {
        name: String,
        first: String,
        second: String,
    },
    #[error(transparent)]
    EventModel(#[from] quent_instrumentation_build::GenerateError),
    #[error("generated stored-event retrieval code did not form a valid Rust file")]
    InvalidGeneratedCode(#[source] syn::Error),
    #[error("failed to write generated stored-event retrieval source")]
    Io(#[from] std::io::Error),
}

/// Information about generated stored-event retrieval source.
pub struct GenerateInfo {
    /// Path of the generated Rust source file.
    pub path: PathBuf,
    /// Constraint names without registered validators.
    pub warnings: Vec<String>,
}

/// Generates event types and their typed stored-event retrieval API.
///
/// # Errors
///
/// Returns an error when the options are inconsistent, the schema cannot be generated, or the
/// output cannot be written.
pub fn generate(schema: &Schema, opts: &Options) -> Result<GenerateInfo, GenerateError> {
    let warnings = quent_instrumentation_build::validate_schema(schema)?;
    let file_name = opts
        .file_name
        .clone()
        .unwrap_or_else(|| format!("{}.rs", schema.name().to_string().to_lowercase()));
    let path = opts.out_dir.join(file_name);
    std::fs::write(&path, generate_str(schema, opts)?)?;
    Ok(GenerateInfo { path, warnings })
}

/// Returns stored-event retrieval source for `schema`.
///
/// # Errors
///
/// Returns an error when the options are inconsistent, generated names conflict, event generation
/// fails, or the combined output is not valid Rust.
pub fn generate_str(schema: &Schema, opts: &Options) -> Result<String, GenerateError> {
    if opts.filesystem && !opts.combined_event {
        return Err(GenerateError::FilesystemRequiresCombinedEvent);
    }

    let event_opts = quent_instrumentation_build::Options {
        instrumentation: false,
        debug: opts.debug,
        serde: true,
        event_derives: opts.event_derives,
        record_derives: opts.record_derives,
        combined_event: opts.combined_event,
        ..quent_instrumentation_build::Options::default()
    };
    let events = quent_instrumentation_build::generate_str(schema, &event_opts)?;
    let events =
        syn::parse_str::<syn::File>(&events).map_err(GenerateError::InvalidGeneratedCode)?;
    let entity_events = if opts.entity_events {
        Some(entities::generate(schema, &event_opts, &events)?)
    } else {
        None
    };

    let loaders = event_loaders::generate(schema, opts.filesystem);

    let file = syn::parse2::<syn::File>(quote! {
        #events

        #loaders

        #entity_events
    })
    .map_err(GenerateError::InvalidGeneratedCode)?;

    Ok(prettyplease::unparse(&file))
}

#[cfg(test)]
mod tests;
