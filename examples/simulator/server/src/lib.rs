// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared setup for the simulator server binaries.
//!
//! Both the HTTP analyzer server (`main.rs`) and the stdio MCP server
//! (`bin/mcp.rs`) read previously collected telemetry from a filesystem
//! directory. The importer/lister callbacks and exporter-format parsing live
//! here so both binaries stay in sync.

use std::path::PathBuf;

use quent_exporter::{
    FileSystemFormat, FileSystemImporterOptions, ImporterOptions, create_importer,
};
use quent_query_engine_server::analyzer_cache::{ImporterFn, ListerFn};
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use quent_simulator_instrumentation::SimulatorEvent;
use uuid::Uuid;

/// Parse an exporter-format CLI string into a [`FileSystemFormat`].
pub fn parse_format(exporter: &str) -> Result<FileSystemFormat, Box<dyn std::error::Error>> {
    match exporter {
        "ndjson" => Ok(FileSystemFormat::Ndjson),
        "msgpack" => Ok(FileSystemFormat::Msgpack),
        "postcard" => Ok(FileSystemFormat::Postcard),
        other => Err(format!("unknown exporter: {other}").into()),
    }
}

/// Build a lister that enumerates engine ids from `output_dir`.
///
/// Each context exports to its own `output_dir/<context-id>/` subdirectory;
/// this lists those subdirectories whose name parses as a UUID.
pub fn build_lister(output_dir: PathBuf) -> Box<ListerFn> {
    Box::new(move || {
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&output_dir)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            if let Some(id) = path
                .file_name()
                .and_then(|s| s.to_str())
                .and_then(|s| Uuid::parse_str(s).ok())
            {
                ids.push(id);
            }
        }
        Ok(ids)
    })
}

/// Build an importer that reads an engine's events from its per-context
/// `output_dir/<context-id>/` directory using the configured `format`.
pub fn build_importer(
    format: FileSystemFormat,
    output_dir: PathBuf,
) -> Box<ImporterFn<SimulatorUiAnalyzer>> {
    Box::new(move |context_id| {
        let dir = output_dir.join(format!("{context_id}"));
        let kind = ImporterOptions::FileSystem(FileSystemImporterOptions { format, path: dir });
        Ok(Box::new(create_importer::<SimulatorEvent>(&kind)?) as Box<dyn Iterator<Item = _>>)
    })
}
