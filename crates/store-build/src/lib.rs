// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates event-loader support and entity-store access from Quent schemas.
//!
//! - **Event loaders** read entity-scoped events as Rust types. This might be
//!   required to construct some store implenmentations (see below).
//! - **Entity stores** provide entity listing, lookup, and schema-defined
//!   accessors (e.g. `my_entity.my_event().my_attribute).
//!
//! # Usage
//!
//! Add `quent-store-build` to `[build-dependencies]`, call [`generate`] from
//! `build.rs`, and include the generated file from Cargo's `OUT_DIR`.
//!
//! The crate including that source needs to depend on `quent-store`. Depending
//! on the type of generated loaders and stores, certain config features must be
//! enabled for `quent-store`. Event types generated with Serde require its `serde` feature.
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
//! quent-store = { path = "../quent/crates/store", features = ["serde"] }
//! ```

use std::path::PathBuf;

pub use quent_events_build::Options as EventOptions;
use quent_schema::Schema;
use quote::quote;

mod entities;
mod event_loaders;

/// Generates event types and their typed stored-event retrieval API.
///
/// # Errors
///
/// Returns an error when the options are inconsistent, the schema cannot be generated, or the
/// output cannot be written.
pub fn generate(schema: &Schema, opts: &Options) -> Result<GenerateInfo, GenerateError> {
    let warnings = quent_events_build::validate_schema(schema)?;
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
    if opts.filesystem && !opts.events.combined_event {
        return Err(GenerateError::FilesystemRequiresCombinedEvent);
    }

    if opts.filesystem && !opts.events.serde {
        return Err(GenerateError::FilesystemRequiresSerde);
    }
    let event_opts = &opts.events;
    let events = quent_events_build::generate_str(schema, event_opts, &event_runtime())?;
    let mut events =
        syn::parse_str::<syn::File>(&events).map_err(GenerateError::InvalidGeneratedCode)?;
    if opts.entity_store.native {
        entities::generate(schema, event_opts, &mut events, &opts.entity_store)?;
    }

    let loaders = event_loaders::generate(schema, opts.filesystem);
    let file = syn::parse2::<syn::File>(quote! {
        #events

        #loaders
    })
    .map_err(GenerateError::InvalidGeneratedCode)?;

    Ok(prettyplease::unparse(&file))
}

/// Options controlling stored-event retrieval source generation.
pub struct Options {
    /// Options for generated event types, records, and model types.
    pub events: EventOptions,

    /// Generate model-wide filesystem loading support.
    ///
    /// Requires [`EventOptions::combined_event`] and [`EventOptions::serde`]. The consuming crate
    /// must enable at least one `quent-store` `io-*` feature.
    pub filesystem: bool,

    /// Generate the selected entity stores and typed access to their events.
    ///
    /// Defaults to a native in-memory store. Disabling all stores also disables
    /// typed entity event access.
    ///
    /// Access traits and event handles are available in each
    /// entity's module, such as `model::connection`.
    pub entity_store: EntityStoreImplementations,

    /// Directory the generated file is written into.
    pub out_dir: PathBuf,

    /// File name to write; defaults to the lowercase schema name with a `.rs` extension.
    pub file_name: Option<String>,
}

/// Store backends to generate alongside entity-specific event access.
#[derive(Clone, Copy, Debug, Default)]
pub struct EntityStoreImplementations {
    /// Generate a Rust-native in-memory store.
    pub native: bool,
    // TODO(johanpel): Add options for Arrow and possibly other formats to
    // directly query from here.
}

impl Default for Options {
    fn default() -> Self {
        Self {
            events: EventOptions {
                serde: true,
                combined_event: true,
                ..EventOptions::default()
            },
            filesystem: true,
            entity_store: EntityStoreImplementations { native: true },
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
    #[error("filesystem loading requires serde generation")]
    FilesystemRequiresSerde,
    #[error("generated entity-event name `{name}` conflicts between {first} and {second}")]
    EntityEventsNameConflict {
        name: String,
        first: String,
        second: String,
    },
    #[error(transparent)]
    EventModel(#[from] quent_events_build::Error),
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

fn event_runtime() -> quent_events_build::Runtime {
    quent_events_build::Runtime {
        events: syn::parse_quote!(::quent_store::events),
        entity_marker: syn::parse_quote!(::quent_store::events::EntityMarker),
        serde: Some(syn::parse_quote!(::quent_store::serde)),
    }
}

#[cfg(test)]
mod tests;
