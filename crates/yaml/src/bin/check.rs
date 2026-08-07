// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `quent-yaml-check`: parse model files and report diagnostics.
//!
//! Usage: `quent-yaml-check [--warnings] <file>...`. Exits non-zero if any
//! file fails to parse. With `--warnings`, also reports unregistered-constraint
//! warnings. Diagnostics are emitted as `tracing` events on stderr.

use std::process::ExitCode;

use quent_yaml::{Diagnostic, Error, Origin};
use tracing::{error, info, warn};

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut warnings = false;
    let mut files = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--warnings" => warnings = true,
            "--help" | "-h" => {
                println!("usage: quent-yaml-check [--warnings] <file>...");
                return ExitCode::SUCCESS;
            }
            _ => files.push(arg),
        }
    }
    if files.is_empty() {
        error!("usage: quent-yaml-check [--warnings] <file>...");
        return ExitCode::FAILURE;
    }

    let mut failed = false;
    for file in &files {
        if let Err(e) = check(file, warnings) {
            if let Error::Io(e) = e {
                error!("{file}: {e}");
            }
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn check(file: &str, warnings: bool) -> Result<(), Error> {
    let src = std::fs::read_to_string(file)?;
    match quent_yaml::parse_from_str(&src, Some(file)) {
        Ok(parsed) => {
            if warnings {
                for warning in &parsed.warnings {
                    warn!("{}", render(&src, warning));
                }
            }
            let events: usize = parsed.schema.entities().map(|e| e.events().count()).sum();
            info!(
                "{file}: ok — model `{}`: {} records, {} entities, {events} events",
                parsed.schema.name(),
                parsed.schema.records().count(),
                parsed.schema.entities().count(),
            );
            Ok(())
        }
        Err(Error::Invalid(diagnostics)) => {
            for diagnostic in diagnostics.iter() {
                error!("{}", render(&src, diagnostic));
            }
            Err(Error::Invalid(diagnostics))
        }
        Err(e) => Err(e),
    }
}

/// A diagnostic rendered with source context when available.
struct RenderedDiagnostic<'a> {
    src: &'a str,
    diagnostic: &'a Diagnostic,
}

impl std::fmt::Display for RenderedDiagnostic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let full = self.diagnostic.to_string();
        let Origin::Location { line, column } = self.diagnostic.origin else {
            return f.write_str(&full);
        };
        let Some(text) = self.src.lines().nth(line.saturating_sub(1)) else {
            return f.write_str(&full);
        };
        let gutter = format!("{line:>4} | ");
        let padding = " ".repeat(gutter.len() + column.saturating_sub(1));
        match full.split_once('\n') {
            Some((header, rest)) => {
                write!(f, "{header}\n{gutter}{text}\n{padding}^\n{rest}")
            }
            None => write!(f, "{full}\n{gutter}{text}\n{padding}^"),
        }
    }
}

fn render<'a>(src: &'a str, diagnostic: &'a Diagnostic) -> RenderedDiagnostic<'a> {
    RenderedDiagnostic { src, diagnostic }
}
