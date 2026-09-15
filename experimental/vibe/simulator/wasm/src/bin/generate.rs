// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates the browser demo's Postcard event recording.

use std::path::PathBuf;

use quent_events::Event;
use quent_instrumentation::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
use quent_simulator_instrumentation as instrumentation;
use quent_simulator_store::{Simulator, SimulatorEvent};
use quent_store::event::{ModelEventStore, filesystem::Store};

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
    let events = Store::<Simulator>::new(event_dir.path())
        .events(context_id)?
        .collect::<Result<Vec<Event<SimulatorEvent>>, _>>()?;
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output, postcard::to_allocvec(&events)?)?;
    println!("wrote {} events to {}", events.len(), output.display());
    Ok(())
}
