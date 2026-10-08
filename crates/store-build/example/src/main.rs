// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Runs instrumentation and loads its filesystem-exported events.

use demo::{Connection, ConnectionEvent, Demo, Server, ServerEvent};
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
    print_connection_events(&connections, &servers)?;

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
    connections: &native::Store<Connection>,
    servers: &native::Store<Server>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(connection) = connections.entities()?.next() {
        println!("\n{} {} events:", ConnectionEvent::NAME, connection.id());
        let events = connections.event_sequence(&connection)?;
        for event in events.events() {
            println!("  {event:?}");
        }
        let host_id = events.events().iter().find_map(|event| match &event.data {
            ConnectionEvent::Opened { host, .. } => Some(host.target),
            _ => None,
        });
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
