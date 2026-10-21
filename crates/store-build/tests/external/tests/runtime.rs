// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_store::{
    EntityStore,
    entity::{BorrowedEventSequenceStore, native::NativeStore},
};
use quent_store_build_external_test::annotation_messages;

#[allow(unused)]
mod application {
    include!(concat!(env!("OUT_DIR"), "/application.rs"));
}

use application::{
    Annotations, AnnotationsEvent, Event, Message, Receiver, ReceiverEvent, Task, TaskEvent, Uuid,
    annotations::{AnnotationsEvents, MarkHandle},
    receiver::ReceiverEvents,
    task::TaskEvents,
};

#[test]
fn analyzes_predefined_payloads_from_application_store() {
    let id = Uuid::from_u128(1);
    let store = NativeStore::<Annotations>::try_from_events([
        Event::new(id, 30, AnnotationsEvent::Closed),
        Event::new(
            id,
            20,
            AnnotationsEvent::Mark {
                message: Message {
                    text: "second".into(),
                },
            },
        ),
        Event::new(
            id,
            10,
            AnnotationsEvent::Mark {
                message: Message {
                    text: "first".into(),
                },
            },
        ),
    ])
    .unwrap();

    let sequence = store.event_sequence(id.into()).unwrap();
    // Analysis accepts predefined payloads without depending on
    // application-specific markers.
    assert_eq!(annotation_messages(sequence.events()), ["first", "second"]);

    let entity = store.entity(id).unwrap().unwrap();
    let marks: Vec<_> = entity.mark().collect();
    assert_eq!(marks[0].message().text, "first");
    assert_eq!(marks[1].timestamp(), 20);
    assert_eq!(entity.closed().unwrap().timestamp(), 30);

    let handle = MarkHandle::try_from(&sequence.events()[0]).unwrap();
    assert!(std::ptr::eq(handle.message(), marks[0].message()));
    assert!(MarkHandle::try_from(&sequence.events()[2]).is_err());
}

#[test]
fn borrows_predefined_records_from_local_events() {
    let id = Uuid::from_u128(2);
    let store = NativeStore::<Receiver>::try_from_events([Event::new(
        id,
        10,
        ReceiverEvent::Received {
            message: Message {
                text: "current".into(),
            },
            history: vec![
                Some(Message {
                    text: "previous".into(),
                }),
                None,
            ],
        },
    )])
    .unwrap();
    let entity = store.entity(id).unwrap().unwrap();
    let received = entity.received().next().unwrap();
    let message: &quent_instrumentation_build_external_test::predefined::Message =
        received.message();
    assert_eq!(message.text, "current");
    assert_eq!(received.history()[0].as_ref().unwrap().text, "previous");
    assert!(received.history()[1].is_none());

    let sequence = store.event_sequence(id.into()).unwrap();
    let ReceiverEvent::Received { message, history } = &sequence.events()[0].data;
    assert!(std::ptr::eq(received.message(), message));
    assert!(std::ptr::eq(received.history(), history));
}

#[test]
fn indexes_predefined_fsm_payloads() {
    let id = Uuid::from_u128(3);
    let store = NativeStore::<Task>::try_from_events([
        Event::new(id, 20, TaskEvent::Finished { seq: 1 }),
        Event::new(
            id,
            10,
            TaskEvent::Started {
                seq: 0,
                message: Message {
                    text: "task".into(),
                },
            },
        ),
    ])
    .unwrap();
    let entity = store.entity(id).unwrap().unwrap();
    assert_eq!(entity.started().unwrap().message().text, "task");
    assert_eq!(entity.finished().unwrap().timestamp(), 20);
}
