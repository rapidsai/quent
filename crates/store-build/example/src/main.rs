// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Runs instrumentation and loads its filesystem-exported events.

use quent_store::{
    context::ContextSet,
    entity::EntityHandle,
    event::{EventLoader, filesystem},
};

use crate::demo::{
    Connection, Demo, DemoStore, Query, connection::ConnectionEvents, server::ServerEvents,
};

#[allow(unused_imports, dead_code)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Run the instrumentation example to produce some telemetry we can inspect.
    let output = tempfile::tempdir()?;
    let context_id = quent_instrumentation_build_example::run_with_ndjson(output.path())?;

    // Prepare the filesystem event loader.
    let loader = filesystem::Loader::<Demo>::new(output.path(), ContextSet::one(context_id));

    // A loader loads raw events through the [`EventLoader`] trait. This is a
    // trait such that loading can be backed by various implementations without
    // having to change this code. The events here are not necessarily grouped
    // by entity or guaranteed to be ordered by timestamp.
    for event in EventLoader::<Query>::events(&loader)?.take(2) {
        println!("{event:?}");
    }
    println!("  ...");

    // Load all entity types into the generated schema-wide store.
    //
    // An entity store can list entities and look up their events by id through
    // the [`EntityStore`] trait.
    //
    // Note that [`EntityStore`] implementations may choose to initialize
    // without using an explicit event iterator such as is provided in this
    // example by the [`EventLoader`] above.
    let store = DemoStore::try_from_loader(&loader)?;

    // Take the ID of the first entity, then look it up and turn it into a
    // handle through which we can access the event data using schema-defined
    // accessors.
    if let Some(id) = store.entities::<Connection>()?.next() {
        let entity = store.entity(id)?.unwrap();
        // Note we unwrap here because we just got the id from the store, so we
        // know it exists there. If this ID is taken from data, it would be
        // prudent to properly handle a missing entity (also see below).

        // We can access its underlying event sequence. This is now scoped to
        // this particular entity only and it is ordered by timestamp.
        for event in entity.event_sequence().events() {
            println!("{event:?}");
        }
        println!("  ...");

        // Access some properties common to all entities.
        println!(
            "Observed entity {} between {} and {}",
            entity.id().uuid(),
            entity.earliest_timestamp(),
            entity.latest_timestamp()
        );

        // Access some entity-specific properties:
        if let Some(opened) = entity.opened() {
            println!(
                "\tOpened at {} to {}:{}",
                opened.timestamp(),
                opened.peer().host,
                opened.peer().port
            );
            // Since the Connection refers to its parent Server, we can try to
            // look the parent up:
            if let Some(server) = store.entity(opened.host())? {
                println!(
                    "\tOwned by server booted at {} with pid {}",
                    server.booted().timestamp(),
                    server.booted().process().native_id
                );
            }
        }

        // The "transfer" event on the "Connection" entity can occur multiple
        // times per entity as described in the schema.
        for transfer in entity.transfer() {
            println!(
                "\tTransfer: @={} size={}",
                transfer.timestamp(),
                transfer.bytes()
            )
        }
    }

    Ok(())
}
