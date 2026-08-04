// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_instrumentation::{EventCallback, ExporterOptions};

use crate::demo::{
    Connection, Context, Demo, Handle, Observer, Query, Server, Thread, ThreadPool, ThreadUsage,
    Uuid,
};

#[allow(unused)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The context owns the exporter and exposes one observer per entity type.
    let context: Context<Demo> = Context::try_new(Some(debug_printing_exporter()))?;

    // `observer.handle()` creates a fresh entity instance to events emit for.
    let mut server = context.observer::<Server>().handle();
    server.booted(demo::quent::os::Process {
        native_id: std::process::id(),
    })?;

    let mut pool = context.observer::<ThreadPool>().handle();
    pool.created(server.as_entity_ref())?;

    let mut thread = context.observer::<Thread>().handle();
    thread.started(
        demo::quent::os::Thread {
            // Obtaining the native thread ID is left as an exercise to the reader.
            native_id: 42,
        },
        pool.as_entity_ref(),
    )?;

    let observer: Observer<Connection> = context.observer::<Connection>();
    // Once-cardinality events take `&mut self` and may fire only once, tracked
    // by the handle, hence it is mut:
    let mut conn: Handle<Connection> = observer.handle();

    // One method per entity event:
    conn.opened(
        demo::Endpoint {
            host: "localhost".to_owned(),
            port: 8080,
        },
        Uuid::nil(),
        // A handle can deal out a reference to the entity it represents:
        server.as_entity_ref(),
    )?;
    conn.data(1234, None)?;

    // A `dynamic` schema field maps to `DynamicAttributes`, which are
    // dynamically-typed key-value pairs:
    let mut extra = demo::DynamicAttributes::new();
    extra.add_string("peer_agent", "curl/8.4");
    extra.add_u64("chunk_index", 3);
    extra.add_bool("compressed", true);
    conn.data(
        5678,
        Some(demo::Meta {
            tags: vec!["tls".to_string(), "keepalive".to_string()],
            extra,
        }),
    )?;

    // `as_entity_ref_with` produces an entity ref that also carries data:
    conn.routed(server.as_entity_ref_with(demo::Route { hops: 3 }))?;

    // An FSM entity's events are transitions into its states.
    // Their cardinality is derived from the topology at build time.
    //
    // FSMs will get typestate pattern handles in the future, also see
    // https://github.com/rapidsai/quent/issues/416
    let mut query = context.observer::<Query>().handle();
    query.submitted("select 1".to_owned(), conn.as_entity_ref())?;
    query.running(10, thread.as_entity_ref_with(ThreadUsage))?;
    query.ready(true)?;

    conn.closed()?;

    // A once-event returns an error if emitted again.
    assert!(conn.closed_emitted());
    assert!(conn.closed().is_err());

    Ok(())
}

/// Return an exporter that debug-prints each emitted event's payload.
fn debug_printing_exporter() -> ExporterOptions {
    ExporterOptions::Callback(EventCallback::new(|recorded| {
        if let Some(event) = demo::AnyEvent::from_any(recorded.event.as_ref()) {
            println!("{event:?}");
        }
    }))
}
