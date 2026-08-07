// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Diagnostics for model sources.
//!
//! `serde` reports parse and shape errors with a source line and column;
//! lowering reports its own problems with a dotted semantic path instead
//! (the deserializer has consumed the structure by then).

/// Where in the source a diagnostic points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A source line and column (counted from 1), from parsing.
    Location { line: usize, column: usize },
    /// A dotted semantic path, e.g. `entities.Engine.events.started`, from
    /// lowering.
    Path(String),
    /// The model as a whole, with no finer location.
    Whole,
}

/// A single problem in a model source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The source name (e.g. a file path), or `None` when the input was
    /// unnamed.
    pub source: Option<String>,
    /// Where in the source the problem is.
    pub origin: Origin,
    /// What is wrong.
    pub message: String,
    /// Optional hint on how to fix it.
    pub help: Option<String>,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.source, &self.origin) {
            (Some(source), Origin::Location { line, column }) => {
                write!(f, "{source}:{line}:{column}: ")?
            }
            (Some(source), Origin::Path(path)) => write!(f, "{source} ({path}): ")?,
            (Some(source), Origin::Whole) => write!(f, "{source}: ")?,
            (None, Origin::Location { line, column }) => write!(f, "{line}:{column}: ")?,
            (None, Origin::Path(path)) => write!(f, "({path}): ")?,
            (None, Origin::Whole) => {}
        }
        write!(f, "{}", self.message)?;
        if let Some(help) = &self.help {
            write!(f, "\n  help: {help}")?;
        }
        Ok(())
    }
}

/// The problems from one parse, in the order they were detected.
///
/// The crate pushes into it as it parses and lowers (so one run reports every
/// problem instead of stopping at the first), stamping each with the shared
/// source name; it is then returned as [`crate::Error::Invalid`]. Callers read
/// it via [`Self::iter`] or `Display`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics {
    source: Option<String>,
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub(crate) fn new(source: Option<&str>) -> Self {
        Self {
            source: source.map(str::to_string),
            items: Vec::new(),
        }
    }

    /// The diagnostics, in order of detection.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> + '_ {
        self.items.iter()
    }

    /// Record a lowering problem at semantic `path`.
    pub(crate) fn error(&mut self, path: &str, message: impl Into<String>, help: Option<String>) {
        let diagnostic = self.make(path, message.into(), help);
        self.items.push(diagnostic);
    }

    /// Record a parse problem at a source `line` and `column`.
    pub(crate) fn error_at(&mut self, location: Option<(usize, usize)>, message: String) {
        let origin = match location {
            Some((line, column)) => Origin::Location { line, column },
            None => Origin::Whole,
        };
        self.items.push(Diagnostic {
            source: self.source.clone(),
            origin,
            message,
            help: None,
        });
    }

    /// Build a diagnostic stamped with the source, without recording it.
    pub(crate) fn make(&self, path: &str, message: String, help: Option<String>) -> Diagnostic {
        let origin = if path.is_empty() {
            Origin::Whole
        } else {
            Origin::Path(path.to_string())
        };
        Diagnostic {
            source: self.source.clone(),
            origin,
            message,
            help,
        }
    }

    pub(crate) fn has_errors(&self) -> bool {
        !self.items.is_empty()
    }
}

impl std::fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, d) in self.items.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{d}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostics {}

impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}
