// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Runs instrumentation and loads its filesystem-exported events.

use demo::{Connection, Demo, Uuid};
use quent_store::context::ContextSet;
use quent_store::entity::memory;
use quent_store::entity::{BorrowedEventSequenceStore, EntityHandle, EntityStore};
use quent_store::event::EventLoader;
use quent_store::event::filesystem::{Loader, Result as LoaderResult};

#[allow(unused_imports, dead_code)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = tempfile::tempdir()?;
    let context_id = quent_instrumentation_build_example::run_with_ndjson(output.path())?;

    let store = Loader::<Demo>::new(output.path(), ContextSet::one(context_id));

    print_raw_connection_events(&store)?;
    let events = EventLoader::<Connection>::events(&store)?.collect::<LoaderResult<Vec<_>>>()?;
    let connection_id = events
        .first()
        .expect("instrumentation emitted a Connection event")
        .id;
    let entities = memory::Store::<Connection>::new(events);
    print_connection_events(&entities, connection_id)?;

    Ok(())
}

// The event store is the lowest layer. It returns individual recorded events.
fn print_raw_connection_events(store: &Loader<Demo>) -> LoaderResult<()> {
    println!("Raw Connection events:");
    for event in EventLoader::<Connection>::events(store)?.take(2) {
        let event = event?;
        println!("  {event:?}");
    }
    println!("  ...");
    Ok(())
}

// The entity store looks up one entity and supplies its ordered events.
fn print_connection_events(
    entities: &memory::Store<Connection>,
    id: Uuid,
) -> Result<(), memory::MissingEntity> {
    let connection = entities.entity(id)?.ok_or(memory::MissingEntity(id))?;
    println!("\n{} {} events:", connection.type_name(), connection.id());
    for event in entities.event_sequence(&connection)?.events() {
        println!("  {event:?}");
    }
    Ok(())
}
