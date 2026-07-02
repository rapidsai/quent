// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
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
//! use quent_instrumentation_build::{GenerateOptions, generate};
//!
//! let schema = todo!();
//! let opts = GenerateOptions {
//!     event_derives: &["Debug", "Clone"],
//!     record_derives: &["Debug", "Clone"],
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

mod common;
mod data_type;
mod events;
mod records;

use std::path::PathBuf;

use quent_constraints::{BaseConstraintsError, Report, validate};
use quent_schema::Schema;
use quote::quote;

use events::generate_event_types;
use records::generate_record_types;

/// Options controlling instrumentation library generation.
pub struct Options {
    /// Derives applied to every generated event payload enum.
    ///
    /// Use this to apply e.g. `&["Debug", "::serde::Serialize"]`
    // TODO(johanpel): derives are kept as simple as possible for now, but
    // eventually some built-in options for built-in exporters (e.g. serde-based
    // or Narrow) will surface here as simpler type-safe options.
    pub event_derives: &'static [&'static str],

    /// Derives applied to every generated record struct.
    ///
    /// Use this to apply e.g. `&["Debug", "::serde::Serialize"]`
    pub record_derives: &'static [&'static str],

    /// Directory the generated file is written into.
    pub out_dir: PathBuf,

    /// File name to write; defaults to `<schema name>.rs` (lowercased) when
    /// `None`.
    pub file_name: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            event_derives: Default::default(),
            record_derives: Default::default(),
            out_dir: PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()),
            file_name: None,
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
/// Returns [`GenerateError`] if a derive entry is not a parseable Rust path, or
/// if the generated code is not a valid Rust file.
pub fn generate_str(schema: &Schema, opts: &Options) -> Result<String, GenerateError> {
    // record structs first, then event enums
    let records = generate_record_types(schema, opts)?;
    let events = generate_event_types(schema, opts)?;
    let file = syn::parse2::<syn::File>(quote! { #records #events })
        .map_err(GenerateError::InvalidGeneratedCode)?;
    Ok(prettyplease::unparse(&file))
}
