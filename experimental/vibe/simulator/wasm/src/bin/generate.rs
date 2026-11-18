// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates the browser demo's Postcard event recording.

use std::path::PathBuf;

use quent_browser_bundle::Capabilities;
use quent_browser_bundle_build::{BuildOptions, generate_bundle};
use quent_instrumentation::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
use quent_simulator_instrumentation as instrumentation;
use quent_simulator_store::{Simulator, SimulatorEvent};

type SimulatorContext = instrumentation::Context<instrumentation::Simulator>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(args.next().ok_or("usage: generate-demo <output>")?);
    if args.next().is_some() {
        return Err("usage: generate-demo <output>".into());
    }

    let event_dir = tempfile::tempdir()?;
    let context = SimulatorContext::try_new(ExporterOptions::FileSystem(
        FileSystemExporterOptions::new(FileSystemFormat::Ndjson, event_dir.path().to_path_buf()),
    ))?;
    quent_simulator::simulate(context, Default::default());
    let summary = generate_bundle::<Simulator>(
        event_dir.path(),
        &output,
        &BuildOptions {
            model_name: "quent-simulator",
            schema_digest: "quent-simulator-v1",
            analyzer_revision: env!("CARGO_PKG_VERSION"),
            capabilities: Capabilities {
                data_flow: true,
                nvtx: false,
            },
        },
        |event| matches!(event.data, SimulatorEvent::Engine(_)).then_some(event.id),
    )?;
    println!(
        "wrote {} events for engine {} to {}",
        summary.event_count,
        summary.engine_id,
        output.display()
    );
    Ok(())
}
