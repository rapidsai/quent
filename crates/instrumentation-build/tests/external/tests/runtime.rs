// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use application::{AnnotationsEvent, Message, TaskEvent};
use quent_instrumentation::{EventCallback, EventPayload};
use quent_instrumentation_build_external_test::annotation_message;

#[allow(unused)]
mod application {
    include!(concat!(env!("OUT_DIR"), "/application.rs"));
}

#[test]
fn schema_owned_handles_emit_external_payloads() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let output = Arc::clone(&recorded);
    let messages = Arc::new(Mutex::new(Vec::new()));
    let message_output = Arc::clone(&messages);
    let context =
        application::Context::<application::Application>::try_new(EventCallback::<
            application::ApplicationEvent,
        >::new(move |event| {
            // Pass the payload to code that only depends on predefined types.
            if let application::ApplicationEvent::Annotations(payload) = &event.data
                && let Some(message) = annotation_message(payload)
            {
                message_output.lock().unwrap().push(message.to_owned());
            }
            output.lock().unwrap().push(event);
        }))
        .unwrap();
    let observer = context.observer::<application::Annotations>();
    let mut annotations = observer.handle();
    let id = annotations.id();
    let target_id = application::Uuid::now_v7();
    annotations
        .mark(Message {
            text: "first".to_owned(),
        })
        .unwrap();
    annotations
        .mark(Message {
            text: "second".to_owned(),
        })
        .unwrap();
    annotations
        .link(application::EntityRef::new(target_id, ()))
        .unwrap();
    annotations.closed().unwrap();
    assert!(annotations.closed_emitted());
    assert!(annotations.closed().is_err());

    drop(annotations);
    drop(observer);
    drop(context);

    let events = recorded.lock().unwrap();
    assert_eq!(events.len(), 4);
    assert!(events.iter().all(|event| event.id == id));
    assert_eq!(*messages.lock().unwrap(), ["first", "second"]);
    assert!(matches!(
        &events[2].data,
        application::ApplicationEvent::Annotations(AnnotationsEvent::Link { target })
            if target.target == target_id
    ));
    assert!(matches!(
        events[3].data,
        application::ApplicationEvent::Annotations(AnnotationsEvent::Closed)
    ));
    assert_eq!(AnnotationsEvent::NAME, "Annotations");
}

#[test]
fn external_fsm_payload_keeps_owner_provided_sequence_handling() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let output = Arc::clone(&recorded);
    let context =
        application::Context::<application::Application>::try_new(EventCallback::<
            application::ApplicationEvent,
        >::new(move |event| {
            output.lock().unwrap().push(event);
        }))
        .unwrap();
    let started = context
        .observer::<application::Task>()
        .handle()
        .started(Message {
            text: "task".to_owned(),
        });
    let id = started.id();
    let mut dynamic = started.into_dynamic();
    dynamic.finished().unwrap();
    assert!(dynamic.finished().is_err());
    assert_eq!(dynamic.id(), id);
    drop(dynamic);
    drop(context);

    let events = recorded.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|event| event.id == id));
    assert!(matches!(
        events[0].data,
        application::ApplicationEvent::Task(TaskEvent::Started { seq: 0, .. })
    ));
    assert!(matches!(
        events[1].data,
        application::ApplicationEvent::Task(TaskEvent::Finished { seq: 1 })
    ));
    assert_eq!(TaskEvent::NAME, "Task");
}

#[test]
fn local_events_reuse_generated_record_types() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let output = Arc::clone(&recorded);
    let context = application::Context::<application::Application>::try_new(EventCallback::<
        application::ApplicationEvent,
    >::new(
        move |event| output.lock().unwrap().push(event),
    ))
    .unwrap();
    let receiver = context.observer::<application::Receiver>().handle();
    let id = receiver.id();
    receiver
        .received(
            Message {
                text: "current".to_owned(),
            },
            vec![
                Some(Message {
                    text: "previous".to_owned(),
                }),
                None,
            ],
        )
        .unwrap();
    drop(receiver);
    drop(context);

    let events = recorded.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, id);
    let application::ApplicationEvent::Receiver(application::ReceiverEvent::Received {
        message,
        history,
    }) = &events[0].data
    else {
        panic!("expected the receiver event");
    };
    assert_eq!(message.text, "current");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].as_ref().unwrap().text, "previous");
    assert!(history[1].is_none());
}
