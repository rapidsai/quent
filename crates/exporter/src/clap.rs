// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Clap argument group for selecting an exporter from the command line.

use std::path::PathBuf;

#[cfg(feature = "collector")]
use crate::CollectorExporterOptions;
use crate::ExporterOptions;
#[cfg(filesystem)]
use crate::{FileSystemExporterOptions, FileSystemFormat};

/// Exporter selected on the command line. `None` is the no-op exporter.
#[derive(::clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExporterKind {
    #[cfg(feature = "postcard")]
    Postcard,
    #[cfg(feature = "msgpack")]
    Messagepack,
    #[cfg(feature = "ndjson")]
    Ndjson,
    #[cfg(feature = "collector")]
    Collector,
    None,
}

/// Clap argument group selecting an exporter. Flatten into a binary's args with
/// `#[command(flatten)]`:
///
/// ```
/// use clap::Parser;
/// use quent_exporter::clap::ExporterArgs;
///
/// #[derive(Parser)]
/// struct Args {
///     #[arg(long, default_value_t = 4)]
///     num_workers: usize,
///
///     #[command(flatten)]
///     exporter: ExporterArgs,
/// }
///
/// let args = Args::parse_from(["app", "--num-workers", "8"]);
/// let options = args.exporter.into_options();
/// ```
#[derive(::clap::Args, Debug)]
pub struct ExporterArgs {
    /// Quent event exporter to use.
    #[arg(long, value_enum, default_value_t = ExporterKind::None, env = "QUENT_EXPORTER")]
    pub exporter: ExporterKind,

    /// Quent collector address when `--exporter collector`.
    #[arg(
        long,
        default_value = "http://localhost:7836",
        env = "QUENT_COLLECTOR_ADDRESS"
    )]
    pub collector_address: http::Uri,

    /// Quent output directory for filesystem exporters.
    #[arg(long, default_value = "events", env = "QUENT_OUTPUT_DIR")]
    pub output_dir: PathBuf,
}

impl ExporterArgs {
    /// Resolve to exporter options; `ExporterKind::None` selects the no-op
    /// exporter.
    pub fn into_options(self) -> Option<ExporterOptions> {
        #[cfg(filesystem)]
        let filesystem = |format| {
            ExporterOptions::FileSystem(FileSystemExporterOptions {
                format,
                root: self.output_dir.clone(),
            })
        };
        match self.exporter {
            #[cfg(feature = "postcard")]
            ExporterKind::Postcard => Some(filesystem(FileSystemFormat::Postcard)),
            #[cfg(feature = "msgpack")]
            ExporterKind::Messagepack => Some(filesystem(FileSystemFormat::Msgpack)),
            #[cfg(feature = "ndjson")]
            ExporterKind::Ndjson => Some(filesystem(FileSystemFormat::Ndjson)),
            #[cfg(feature = "collector")]
            ExporterKind::Collector => Some(ExporterOptions::Collector(CollectorExporterOptions {
                address: self.collector_address,
            })),
            ExporterKind::None => None,
        }
    }
}
