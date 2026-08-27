// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Runs instrumentation and loads its filesystem-exported events.

use demo::{Demo, Query};
use quent_store::event::filesystem::Store;
use quent_store::event::{EntityEventStore, ModelEventStore};

#[allow(unused_imports)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = tempfile::tempdir()?;
    let context_id = quent_instrumentation_build_example::run_with_ndjson(output.path())?;

    let store = Store::<Demo>::new(output.path());

    println!("--- Query events ---");

    // Load events for one entity type.
    for event in store.entity_events::<Query>(context_id)? {
        println!("{:?}", event?);
    }

    println!("\n--- All model events ---");

    // Load all model events as `DemoEvent`.
    for event in store.events(context_id)? {
        println!("{:?}", event?);
    }

    Ok(())
}
