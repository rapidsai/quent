// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

#[allow(unused)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

mod compiles;

use std::sync::{Arc, Mutex};

use demo::{
    Connection, Context, Demo, DemoEvent, Noop, Query, Server, Thread, ThreadUsage, Uuid,
    query_state,
};
use quent_instrumentation::EventCallback;

/// Runs `emit` against a context recording each event's entity ID and
/// timestamp, returning them once the exporter has drained.
fn recorded_timestamps(emit: impl FnOnce(&Context<Demo>)) -> Vec<(Uuid, u64)> {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&recorded);
    let context = Context::<Demo>::try_new(EventCallback::<DemoEvent>::new(move |event| {
        sink.lock().unwrap().push((event.id, event.timestamp));
    }))
    .unwrap();
    emit(&context);
    drop(context);
    Arc::try_unwrap(recorded).unwrap().into_inner().unwrap()
}

fn timestamps_of(recorded: &[(Uuid, u64)], id: Uuid) -> Vec<u64> {
    recorded
        .iter()
        .filter(|(event_id, _)| *event_id == id)
        .map(|(_, timestamp)| *timestamp)
        .collect()
}

#[test]
fn timestamped_event_methods_emit_the_supplied_timestamp() {
    let mut ids = Vec::new();
    let recorded = recorded_timestamps(|context| {
        let mut server = context.observer::<Server>().handle();
        server.booted_at(1).unwrap();
        assert!(server.booted_emitted());
        assert!(server.booted_at(2).is_err());
        assert!(server.booted().is_err());

        let connection = context.observer::<Connection>().handle();
        connection.transfer_at(3, 10, None).unwrap();
        connection.transfer_at(4, 20, None).unwrap();
        ids.extend([server.id(), connection.id()]);
    });

    assert_eq!(timestamps_of(&recorded, ids[0]), [1]);
    assert_eq!(timestamps_of(&recorded, ids[1]), [3, 4]);
}

#[test]
fn timestamped_transitions_emit_the_supplied_timestamp() {
    let mut ids = Vec::new();
    let recorded = recorded_timestamps(|context| {
        let connection = context.observer::<Connection>().handle();
        let thread = context.observer::<Thread>().handle();
        let typestate = context
            .observer::<Query>()
            .handle()
            .submitted_at(5, "select 1".to_owned(), connection.as_entity_ref())
            .running_at(6, 10, thread.as_entity_ref_with(ThreadUsage))
            .ready_at(7, true);

        let mut dynamic = context.observer::<Query>().handle().into_dynamic();
        let dynamic_id = dynamic.id();
        assert!(
            dynamic
                .running_at(8, 10, thread.as_entity_ref_with(ThreadUsage))
                .is_err()
        );
        dynamic
            .submitted_at(9, "select 2".to_owned(), connection.as_entity_ref())
            .unwrap();
        dynamic
            .running(10, thread.as_entity_ref_with(ThreadUsage))
            .unwrap();
        dynamic
            .running_at(11, 20, thread.as_entity_ref_with(ThreadUsage))
            .unwrap();
        assert!(
            dynamic
                .submitted_at(12, "select 3".to_owned(), connection.as_entity_ref())
                .is_err()
        );
        assert!(dynamic.try_into::<query_state::Running>().is_ok());
        ids.extend([typestate.id(), dynamic_id]);
    });

    assert_eq!(timestamps_of(&recorded, ids[0]), [5, 6, 7]);
    // Rejected transitions emit nothing; the untimestamped one uses the clock.
    let dynamic = timestamps_of(&recorded, ids[1]);
    assert_eq!(dynamic.len(), 3);
    assert_eq!(dynamic[0], 9);
    assert!(dynamic[1] > 12);
    assert_eq!(dynamic[2], 11);
}

#[test]
fn dynamic_to_typestate_retains_handle_on_mismatch() {
    let context = Context::<Demo>::try_new(Noop).unwrap();
    let connection = context.observer::<Connection>().handle();
    let dynamic = context
        .observer::<Query>()
        .handle()
        .submitted("select 1".to_owned(), connection.as_entity_ref())
        .into_dynamic();
    let query_id = dynamic.id();

    let mismatch = match dynamic.try_into::<query_state::Running>() {
        Ok(_) => panic!("submitted unexpectedly converted to running"),
        Err(error) => error,
    };
    let submitted = mismatch
        .into_handle()
        .try_into::<query_state::Submitted>()
        .unwrap();
    assert_eq!(submitted.id(), query_id);
}
