// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_instrumentation::EventCallback;

use crate::demo::{Connection, Context, Demo, DemoEvent, Handle, Observer, Query, Server, Uuid};

#[allow(unused)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The context builds one exporter pipeline per entity event type and
    // exposes the corresponding typed observers.
    let context: Context<Demo> = Context::try_new(println_exporter())?;

    // `observer.handle()` creates a fresh entity instance to events emit for.
    let mut server = context.observer::<Server>().handle();
    server.booted()?;

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
    query.running(10)?;
    query.ready(true)?;

    conn.closed()?;

    // A once-event returns an error if emitted again.
    assert!(conn.closed_emitted());
    assert!(conn.closed().is_err());

    Ok(())
}

/// Return a callback that debug-prints each emitted event.
fn println_exporter() -> EventCallback<DemoEvent> {
    EventCallback::new(|event| println!("{event:?}"))
}
