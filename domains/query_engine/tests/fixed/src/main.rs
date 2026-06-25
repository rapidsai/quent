// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Binary wrapper around the fixed query-engine event emitter. See the crate
//! library for the scenario.

use clap::Parser;
use quent_exporter::{
    CollectorExporterOptions, ExporterOptions, FileSystemExporterOptions, FileSystemFormat,
};
use quent_query_engine_fixed::emit;
use quent_simulator_instrumentation::SimulatorContext;

#[derive(Parser, Debug)]
#[command(name = "quent-query-engine-fixed")]
#[command(about = "Emits a fixed query-engine telemetry stream", long_about = None)]
struct Args {
    #[arg(long, default_value = "collector")]
    exporter: String,

    #[arg(
        long,
        default_value = "http://localhost:7836",
        env = "QUENT_COLLECTOR_ADDRESS"
    )]
    collector_address: String,

    #[arg(long, default_value = "events")]
    output_dir: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let exporter = match args.exporter.as_str() {
        "postcard" => Some(ExporterOptions::FileSystem(FileSystemExporterOptions {
            format: FileSystemFormat::Postcard,
            root: args.output_dir.clone().into(),
        })),
        "messagepack" => Some(ExporterOptions::FileSystem(FileSystemExporterOptions {
            format: FileSystemFormat::Msgpack,
            root: args.output_dir.clone().into(),
        })),
        "ndjson" => Some(ExporterOptions::FileSystem(FileSystemExporterOptions {
            format: FileSystemFormat::Ndjson,
            root: args.output_dir.clone().into(),
        })),
        "collector" => Some(ExporterOptions::Collector(CollectorExporterOptions {
            address: args.collector_address,
        })),
        "none" => None,
        _ => {
            return Err(format!(
                "invalid exporter '{}': must be postcard, messagepack, ndjson, collector, or none",
                args.exporter
            )
            .into());
        }
    };

    let ctx = SimulatorContext::try_new(exporter)?;
    emit(&ctx);
    Ok(())
}
