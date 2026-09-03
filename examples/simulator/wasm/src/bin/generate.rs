// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates the browser demo's Postcard event recording.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use quent_events::Event;
use quent_model::EventCallback;
use quent_simulator_instrumentation::{SimulatorContext, SimulatorEvent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(args.next().ok_or("usage: generate-demo <output>")?);
    if args.next().is_some() {
        return Err("usage: generate-demo <output>".into());
    }

    let events = Arc::new(Mutex::new(Vec::<Event<SimulatorEvent>>::new()));
    let callback = {
        let events = Arc::clone(&events);
        EventCallback::new(move |event| events.lock().unwrap().push(event))
    };
    let context = SimulatorContext::try_new(callback)?;
    quent_simulator::simulate(context, Default::default());

    let events = Arc::try_unwrap(events)
        .map_err(|_| "simulator event callback is still in use")?
        .into_inner()?;
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
