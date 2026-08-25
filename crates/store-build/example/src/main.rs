// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Runs instrumentation and loads its filesystem-exported events.

use demo::{Connection, Demo, Uuid};
use quent_store::entity::{ContextSet, EntityStore, ModelEntityStore};
use quent_store::event::EntityEventStore;
use quent_store::event::filesystem::{Result as StoreResult, Store};

#[allow(unused)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = tempfile::tempdir()?;
    let context_id = quent_instrumentation_build_example::run_with_ndjson(output.path())?;

    let store = Store::<Demo>::new(output.path());
    let contexts = ContextSet::try_new([context_id])?;

    print_raw_connection_events(&store, context_id)?;
    print_connection_entities(&store, &contexts)?;
    print_all_entities(&store, &contexts)?;

    Ok(())
}

// The event store is the lowest layer. It returns individual recorded events.
fn print_raw_connection_events(store: &Store<Demo>, context_id: Uuid) -> StoreResult<()> {
    println!("Raw Connection events:");
    for event in store.entity_events::<Connection>(context_id)? {
        let event = event?;
        println!("  {event:?}");
    }
    Ok(())
}

// The entity store groups the raw events and returns one handle per entity UUID.
fn print_connection_entities(store: &Store<Demo>, contexts: &ContextSet) -> StoreResult<()> {
    println!("Connection entities:");
    for connection in store.entities::<Connection>(contexts)? {
        println!(
            "  Connection {} from {:?}:",
            connection.id(),
            connection.contexts()
        );
        for event in connection.load_events(store)?.into_inner() {
            println!("    {event:?}");
        }
    }
    Ok(())
}

// The model entity store uses DemoEvent to provide handles for every entity type.
fn print_all_entities(store: &Store<Demo>, contexts: &ContextSet) -> StoreResult<()> {
    println!("All entities:");
    for entity in store.any_entities(contexts)? {
        println!("  Entity {} from {:?}:", entity.id(), entity.contexts());
        for event in entity.load_events(store)?.into_inner() {
            println!("    {event:?}");
        }
    }
    Ok(())
}
