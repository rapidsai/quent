// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Parses a YAML model file into a [`Schema`].
//!
//! [`parse_from_file`] takes a path, [`parse_from_str`] a string; both return
//! the schema plus any warnings, or the diagnostics explaining why it could not
//! be parsed.

use std::path::Path;

use quent_schema::Schema;
use serde_saphyr::{MessageFormatter, UserMessageFormatter};

mod ast;
mod diag;
mod extensions;
mod lower;

pub use diag::{Diagnostic, Diagnostics, Origin};

/// A successfully parsed model.
#[derive(Debug)]
pub struct Parsed {
    /// The parsed schema.
    pub schema: Schema,
    /// Advisory problems that did not prevent parsing.
    pub warnings: Vec<Diagnostic>,
}

/// Failure while parsing a model source.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(Diagnostics),
}

/// Parse and lower model source text into a validated [`Schema`].
///
/// Diagnostics name the source `source`, or leave it unnamed when `None`. See
/// [`parse_from_file`] to read and name a file.
pub fn parse_from_str(src: impl AsRef<str>, source: Option<&str>) -> Result<Parsed, Error> {
    let src = src.as_ref();
    let mut sink = diag::Diagnostics::new(source);

    let model: ast::Model = match serde_saphyr::from_str(src) {
        Ok(model) => model,
        Err(e) => {
            let location = e
                .location()
                .map(|l| (l.line() as usize, l.column() as usize));
            sink.error_at(
                location,
                UserMessageFormatter.format_message(&e).into_owned(),
            );
            return Err(Error::Invalid(sink));
        }
    };

    let schema = match lower::lower(&model, &mut sink) {
        Some(schema) => schema,
        None => {
            if !sink.has_errors() {
                sink.error("", "schema could not be built", None);
            }
            return Err(Error::Invalid(sink));
        }
    };
    if sink.has_errors() {
        return Err(Error::Invalid(sink));
    }

    let warnings = match extensions::validate_schema(&schema, &mut sink) {
        Some(warnings) => warnings,
        None => return Err(Error::Invalid(sink)),
    };
    Ok(Parsed { schema, warnings })
}

/// Read a model file and parse it via [`parse_from_str`], naming it by path.
pub fn parse_from_file(path: impl AsRef<Path>) -> Result<Parsed, Error> {
    let path = path.as_ref();
    let src = std::fs::read_to_string(path)?;
    parse_from_str(&src, Some(&path.display().to_string()))
}
