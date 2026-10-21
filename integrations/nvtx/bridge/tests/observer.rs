// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_events::{EntityRef, Event};
use quent_nvtx_bridge::Capture;
use quent_nvtx_sink::quent::nvtx::SinkEvent;
use std::sync::{Arc, Mutex};
mod fixtures;

#[test]
fn capture_creates_an_instance_linked_to_the_process_and_preserves_callback_timestamp() {
    let events = Arc::new(Mutex::new(Vec::<Event<SinkEvent>>::new()));
    let context = fixtures::context({
        let events = Arc::clone(&events);
        move |event| events.lock().unwrap().push(event)
    });
    let observer = context.observer::<fixtures::Sink>();
    let mut process_handle = context
        .observer::<fixtures::application::Process>()
        .handle();
    process_handle
        .started(fixtures::application::quent::os::Process {
            native_id: std::process::id(),
        })
        .unwrap();
    let process = process_handle.id();
    let capture = Capture::install(observer.clone(), EntityRef::new(process, ())).unwrap();
    assert!(Capture::install(observer.clone(), EntityRef::new(process, ())).is_err());
    let before = quent_time::timestamp();
    nvtx::mark(c"observer-instance");
    let after = quent_time::timestamp();
    drop(process_handle);
    drop(observer);
    drop(context);
    drop(capture);
    let events = events.lock().unwrap();
    assert_eq!(events.len(), 2);
    let id = events[0].id;
    assert_ne!(id, process);
    assert!(events.iter().all(|event| event.id == id));
    assert!(
        matches!(&events[0].data, SinkEvent::Initialized { process: reference } if reference.target == process)
    );
    let SinkEvent::Mark { data } = &events[1].data else {
        panic!()
    };
    assert_eq!(
        data.attributes.message.as_ref().unwrap().string.as_deref(),
        Some("observer-instance")
    );
    assert!((before..=after).contains(&events[1].timestamp));
}
