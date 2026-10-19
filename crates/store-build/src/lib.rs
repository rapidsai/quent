// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates Rust event types, event loaders, entity stores, and model stores
//! from a [`quent_schema::Schema`].
//!
//! - **Event loaders** read entity-scoped events as Rust types. Some store
//!   implementations use them to load events.
//! - **Entity stores** provide entity listing, lookup, and schema-defined
//!   accessors, such as `my_entity.my_event().my_attribute()`.
//! - **Model stores** combine the entity stores for an entire application event
//!   model captured by a schema.
//!
//! The crate including the generated code must depend on `quent-store`. Add
//! `quent-store-build` as a build dependency of that crate.
//!
//! The usual workflow is build-time generation:
//!
//! 1. From your crate's build script, call [`generate`] with `out_dir` set to
//!    the directory Cargo provides via the `OUT_DIR` environment variable; it
//!    writes the generated source there.
//! 2. Pull that file into your crate's source at compile time with the
//!    `include!` macro.
//!
//! # Example
//!
//! In your crate's `build.rs`:
//!
//! ```ignore
//! use quent_store_build::{Options, generate};
//!
//! let schema = todo!();
//! let opts = Options::default();
//! generate(&schema, &opts)?;
//! ```
//!
//! Then, anywhere in your crate's source:
//!
//! ```ignore
//! pub mod demo {
//!     include!(concat!(env!("OUT_DIR"), "/demo.rs"));
//! }
//! ```
//!
//! # Restrictions
//!
//! ## Filesystem loading
//!
//! To generate filesystem loaders, set [`Options::filesystem`] to `true`,
//! enable [`EventOptions::combined_event`] and [`EventOptions::serde`], and
//! enable at least one `io-*` feature on `quent-store`.
//!
//! ## Serde requirements
//!
//! To generate event types that support Serde, set [`EventOptions::serde`] to
//! `true` and enable the `serde` feature on `quent-store`.

use std::path::PathBuf;

pub use quent_events_build::Options as EventOptions;
use quent_schema::Schema;
use quote::quote;

mod entities;
mod event_loaders;
mod model;

/// Generates event types and their typed stored-event retrieval API.
///
/// # Errors
///
/// Returns an error when the options are inconsistent, the schema cannot be
/// generated, or the output cannot be written.
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
/// Returns an error when the options are inconsistent, generated names
/// conflict, event generation fails, or the combined output is not valid Rust.
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
    let model_store = opts
        .entity_store
        .native
        .then(|| model::generate(schema, &events))
        .transpose()?;

    let file = syn::parse2::<syn::File>(quote! {
        #events

        #loaders
        #model_store
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
    /// Requires [`EventOptions::combined_event`] and [`EventOptions::serde`].
    /// The consuming crate must enable at least one `quent-store` `io-*`
    /// feature.
    pub filesystem: bool,

    /// Generate the selected entity stores and typed access to their events.
    ///
    /// Generates a model-wide `<Model>Store` with typed entity access.
    /// Defaults to a native in-memory store. Disabling all stores also disables
    /// typed entity event access.
    ///
    /// Access traits and event handles are available in each entity's module,
    /// such as `model::connection`.
    pub entity_store: EntityStoreImplementations,

    /// Directory the generated file is written into.
    pub out_dir: PathBuf,

    /// File name to write; defaults to the lowercase schema name with a `.rs`
    /// extension.
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
