// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates the browser demo's Postcard event recording.

use std::path::PathBuf;

use quent_browser_bundle::{Capabilities, FORMAT_VERSION, Metadata};
use quent_events::Event;
use quent_instrumentation::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
use quent_simulator_instrumentation as instrumentation;
use quent_simulator_store::{Simulator, SimulatorEvent};
use quent_store::event::{CombinedEventLoader, filesystem::Loader};

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
    let context_id = context.id();
    quent_simulator::simulate(context, Default::default());
    let events = Loader::<Simulator>::new(
        event_dir.path(),
        quent_store::context::ContextSet::one(context_id),
    )
    .combined_events()?
    .collect::<Result<Vec<Event<SimulatorEvent>>, _>>()?;
    let engine_id = events
        .iter()
        .find_map(|event| matches!(event.data, SimulatorEvent::Engine(_)).then_some(event.id))
        .ok_or("simulator demo contains no engine events")?;
    let payload = postcard::to_allocvec(&events)?;
    let bytes = quent_browser_bundle::encode(
        &Metadata {
            format_version: FORMAT_VERSION,
            model_name: "quent-simulator".into(),
            schema_digest: "quent-simulator-v1".into(),
            model_provenance: None,
            analyzer_revision: env!("CARGO_PKG_VERSION").into(),
            quent_revision: env!("CARGO_PKG_VERSION").into(),
            codec: "postcard".into(),
            engine_id,
            context_ids: vec![context_id],
            capabilities: Capabilities {
                data_flow: true,
                nvtx: false,
            },
        },
        &payload,
    )?;
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output, bytes)?;
    println!("wrote {} events to {}", events.len(), output.display());
    Ok(())
}
