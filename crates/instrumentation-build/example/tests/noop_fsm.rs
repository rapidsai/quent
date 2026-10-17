// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[allow(unused)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

use std::sync::{Arc, Mutex};

use demo::{
    Connection, Context, Demo, DemoEvent, Noop, Query, QueryDynamicState, QueryEvent, Thread,
    ThreadUsage, query_state,
};
use quent_instrumentation::EventCallback;

#[test]
fn noop_skips_initial_state() {
    let context = Context::<Demo>::try_new(Noop).unwrap();
    let mut query = context.observer::<Query>().handle().into_dynamic();
    let query_id = query.id();

    // Disabled instrumentation must not require the initial event or reject repeats.
    query.ready(true).unwrap();
    query.ready(false).unwrap();
    assert_eq!(query.state(), QueryDynamicState::Ready);
    assert_eq!(query.id(), query_id);

    let mismatch = match query.try_into::<query_state::Running>() {
        Ok(_) => panic!("ready unexpectedly converted to running"),
        Err(error) => error,
    };
    let ready = mismatch
        .into_handle()
        .try_into::<query_state::Ready>()
        .unwrap();
    assert_eq!(ready.id(), query_id);
}

#[test]
fn noop_skips_middle_state() {
    let context = Context::<Demo>::try_new(Noop).unwrap();
    let connection = context.observer::<Connection>().handle();
    let mut query = context
        .observer::<Query>()
        .handle()
        .submitted("select 1".to_owned(), connection.as_entity_ref())
        .into_dynamic();

    // Mirrors an inline task that skips scheduler events before execution.
    query.ready(true).unwrap();
    query
        .submitted("select 2".to_owned(), connection.as_entity_ref())
        .unwrap();
    assert_eq!(query.state(), QueryDynamicState::Submitted);
}

#[test]
fn discard_exporter_checks_fsm() {
    let context = Context::<Demo>::try_new(EventCallback::<DemoEvent>::new(|_| {})).unwrap();
    let mut query = context.observer::<Query>().handle().into_dynamic();

    // Discarding output does not make an active pipeline a no-op observer.
    assert!(query.ready(true).is_err());
    assert_eq!(query.state(), QueryDynamicState::New);
}

#[test]
fn active_checks_fsm_steps() {
    let events = Arc::new(Mutex::new(Vec::new()));
    {
        let collected = Arc::clone(&events);
        let context = Context::<Demo>::try_new(EventCallback::<DemoEvent>::new(move |event| {
            if let DemoEvent::Query(query) = event.data {
                collected.lock().unwrap().push(query);
            }
        }))
        .unwrap();
        let connection = context.observer::<Connection>().handle();
        let thread = context.observer::<Thread>().handle();
        let mut query = context.observer::<Query>().handle().into_dynamic();

        // Active capture still rejects missing initial and intermediate events.
        assert!(query.ready(true).is_err());
        assert_eq!(query.state(), QueryDynamicState::New);
        query
            .submitted("select 1".to_owned(), connection.as_entity_ref())
            .unwrap();
        let error = query.ready(true).unwrap_err();
        assert_eq!(
            error.to_string(),
            "cannot transition `Query` from `submitted` to `ready`"
        );
        assert_eq!(query.state(), QueryDynamicState::Submitted);

        query
            .running(1, thread.as_entity_ref_with(ThreadUsage))
            .unwrap();
        query
            .running(2, thread.as_entity_ref_with(ThreadUsage))
            .unwrap();
        query.ready(true).unwrap();
        assert!(query.ready(false).is_err());
        assert_eq!(query.state(), QueryDynamicState::Ready);
    }

    // Dropping the context flushes; rejected calls emit nothing and consume no sequence.
    let events = events.lock().unwrap();
    assert!(matches!(
        events.as_slice(),
        [
            QueryEvent::Submitted { seq: 0, .. },
            QueryEvent::Running {
                seq: 1,
                rows: 1,
                ..
            },
            QueryEvent::Running {
                seq: 2,
                rows: 2,
                ..
            },
            QueryEvent::Ready { seq: 3, ok: true },
        ]
    ));
}
