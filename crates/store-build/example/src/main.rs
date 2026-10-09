// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Runs instrumentation and loads its filesystem-exported events.

use demo::entity_events::connection::{ConnectionEvents, NativeConnectionEvents};
use demo::{Connection, Demo, Server, ServerEvent};
use quent_events::EventPayload;
use quent_store::{
    context::ContextSet,
    entity::{BorrowedEventSequenceStore, EntityHandle, EntityStore, native},
    event::{
        EventLoader,
        filesystem::{Loader, Result as LoaderResult},
    },
};

#[allow(unused_imports, dead_code)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = tempfile::tempdir()?;
    let context_id = quent_instrumentation_build_example::run_with_ndjson(output.path())?;

    let store = Loader::<Demo>::new(output.path(), ContextSet::one(context_id));

    print_raw_connection_events(&store)?;
    let connection_events =
        EventLoader::<Connection>::events(&store)?.collect::<LoaderResult<Vec<_>>>()?;
    let connections = native::Store::<Connection>::new(connection_events);
    let server_events = EventLoader::<Server>::events(&store)?.collect::<LoaderResult<Vec<_>>>()?;
    let servers = native::Store::<Server>::new(server_events);
    print_connection_events(connections, &servers)?;

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

// Entity stores supply ordered events and allow references to be followed by UUID.
fn print_connection_events(
    connections: native::Store<Connection>,
    servers: &native::Store<Server>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(sequence) = connections.into_sequences().next() {
        // Group the owned sequence by event type without cloning its payloads.
        // Conversion fails if a once-event occurs more than once.
        let events = NativeConnectionEvents::try_from(sequence)?;

        println!("\nConnection {} events:", events.id());

        // `opened()` returns an optional borrowed event; `data()` returns an iterator.
        // Each event retains its UUID, timestamp, and typed payload in `data`.
        println!("  opened: {:?}", events.opened());
        println!("  data: {:?}", events.data().collect::<Vec<_>>());

        // The opened event's host reference identifies a server by UUID.
        let host_id = events.opened().map(|event| event.data.host.target);

        if let Some(host_id) = host_id
            && let Some(server) = servers.entity(host_id)?
        {
            println!("\n{} {} events:", ServerEvent::NAME, server.id());
            for event in servers.event_sequence(&server)?.events() {
                println!("  {event:?}");
            }
        }
    }
    Ok(())
}
