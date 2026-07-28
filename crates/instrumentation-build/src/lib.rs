// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates a Rust instrumentation library source from a
//! [`quent_schema::Schema`].
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
//! use quent_instrumentation_build::{Options, generate};
//!
//! let schema = todo!();
//! let opts = Options {
//!     // Exporters serialize events, so a `Serialize` derive is required.
//!     event_derives: &["Debug", "::serde::Serialize"],
//!     record_derives: &["Debug", "::serde::Serialize"],
//!     out_dir: std::env::var("OUT_DIR")?.into(),
//!     file_name: None, // defaults to `<schema name>.rs`
//! };
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
//! Qualified record and entity paths are not supported; generation fails with
//! [`GenerateError::UnsupportedTypePath`].
//!
//! The schema does not limit how many events an entity declares, but this
//! generator caps once-cardinality
//! ([`Cardinality::Once`](quent_schema::Cardinality::Once)) events at 64 per
//! entity; beyond that, generation fails with
//! [`GenerateError::TooManyOnceEvents`].
//!
//! Building an exporter requires the event type to be `Serialize`, so
//! [`Options::event_derives`] (and [`Options::record_derives`], for events
//! carrying records or entity refs) must include a `Serialize`-providing
//! derive; otherwise the generated code will not compile.

mod any_event;
mod common;
mod data_type;
mod events;
mod records;
mod runtime;

use std::path::PathBuf;

use quent_constraints::{BaseConstraintsError, Report, validate};
use quent_schema::{Path, Schema};
use quote::quote;

use events::generate_event_types;
use records::generate_record_types;
use runtime::generate_runtime_types;

/// Options controlling instrumentation library generation.
pub struct Options {
    /// Derives applied to every generated event payload enum.
    ///
    /// Must include a `Serialize`-providing derive (e.g. `"::serde::Serialize"`):
    /// the generated context builds exporters, which require it.
    // TODO(johanpel): derives are kept as simple as possible for now, but
    // eventually some built-in options for built-in exporters (e.g. serde-based
    // or Narrow) will surface here as simpler type-safe options.
    pub event_derives: &'static [&'static str],

    /// Derives applied to every generated record struct.
    ///
    /// Records embedded in events must also be `Serialize`, so include a
    /// `Serialize`-providing derive (e.g. `"::serde::Serialize"`).
    pub record_derives: &'static [&'static str],

    /// Directory the generated file is written into.
    pub out_dir: PathBuf,

    /// File name to write; defaults to `<schema name>.rs` (lowercased) when
    /// `None`.
    pub file_name: Option<String>,

    /// Emit `AnyEvent` and `AnyEvent::from_any`, a decoder from a type-erased
    /// `&dyn Any` back to the concrete `Event<T>`. Carries [`Self::event_derives`].
    ///
    /// No aggregate is emitted when the schema declares no events.
    pub any_event: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            event_derives: Default::default(),
            record_derives: Default::default(),
            out_dir: PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()),
            file_name: None,
            any_event: false,
        }
    }
}

/// An error from generating instrumentation source.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("base schema validation failed: {0}")]
    InvalidSchema(#[from] BaseConstraintsError),
    #[error("invalid derive path {derive:?}")]
    InvalidDerive {
        /// The offending derive entry.
        derive: String,
        /// The underlying parse error.
        source: syn::Error,
    },
    #[error("generated code did not form a valid Rust file")]
    InvalidGeneratedCode(#[source] syn::Error),
    #[error("qualified type path `{path}` is not supported")]
    UnsupportedTypePath {
        /// The unsupported record or entity path.
        path: Path,
    },
    #[error(
        "entity `{entity}` declares {count} once-events, exceeding the maximum of {max}",
        max = crate::runtime::MAX_ONCE_EVENTS
    )]
    TooManyOnceEvents {
        /// The offending entity.
        entity: Path,
        /// The number of once-cardinality events the entity declares.
        count: usize,
    },
    #[error("failed to write generated file")]
    Io(#[from] std::io::Error),
}

pub struct GenerateInfo {
    pub path: PathBuf,
    pub warnings: Vec<String>,
}

/// Generate the full instrumentation source for `schema` with `opts`.
pub fn generate(schema: &Schema, opts: &Options) -> Result<GenerateInfo, GenerateError> {
    let Report {
        base_constraints,
        unregistered_constraints,
        results: _, // unused for now, but built-in constraints go here later
                    // and will add to either errors or warnings.
    } = validate::<()>(schema);

    let warnings = unregistered_constraints;

    // Fail if base constraints aren't met.
    base_constraints?;

    let file_name = opts
        .file_name
        .clone()
        .unwrap_or_else(|| format!("{}.rs", schema.name().to_string().to_lowercase()));
    let path = opts.out_dir.join(file_name);
    std::fs::write(&path, generate_str(schema, opts)?)?;
    Ok(GenerateInfo { path, warnings })
}

/// Return the full instrumentation source for `schema`.
///
/// # Errors
///
/// Returns [`GenerateError`] if the schema contains a qualified type path, a
/// derive entry is not a parseable Rust path, or the generated code is not a
/// valid Rust file.
pub fn generate_str(schema: &Schema, opts: &Options) -> Result<String, GenerateError> {
    ensure_unqualified_type_paths(schema)?;

    // record structs, event enums, then the live instrumentation surface
    let reexports = runtime::reexports();
    let records = generate_record_types(schema, opts)?;
    let events = generate_event_types(schema, opts)?;
    let runtime = generate_runtime_types(schema)?;
    let any_event = if opts.any_event {
        any_event::generate_any_event(schema, opts)?
    } else {
        quote! {}
    };
    let file = syn::parse2::<syn::File>(quote! { #reexports #records #events #runtime #any_event })
        .map_err(GenerateError::InvalidGeneratedCode)?;
    Ok(prettyplease::unparse(&file))
}

fn ensure_unqualified_type_paths(schema: &Schema) -> Result<(), GenerateError> {
    let qualified = schema
        .records()
        .map(|record| record.path())
        .chain(schema.entities().map(|entity| entity.path()))
        .find(|path| !path.namespace().is_empty());

    match qualified {
        Some(path) => Err(GenerateError::UnsupportedTypePath { path: path.clone() }),
        None => Ok(()),
    }
}
