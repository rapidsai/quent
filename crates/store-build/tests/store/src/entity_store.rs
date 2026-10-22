// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_store::{
    Error, Timestamped,
    entity::{BorrowedEventSequenceStore, EntityHandle, EntityStore, Id, native::NativeStore},
};

#[allow(unused_imports, dead_code)]
mod model {
    include!(concat!(env!("OUT_DIR"), "/model.rs"));
}

use self::model::{
    EntityRef, Event, Uuid,
    nested::r#type::{Worker, WorkerEvent, worker::WorkerEvents},
};

// Checks lookup by ID and entity reference returns the same stored entity.
#[test]
fn looks_up_entities_by_id_and_reference() {
    let id = Id::<Worker>::new(Uuid::from_u128(8));
    let missing = Id::<Worker>::new(Uuid::from_u128(9));
    let store = NativeStore::<Worker>::try_from_events([
        Event::new(
            id.uuid(),
            20,
            WorkerEvent::Update {
                string: "later".to_owned(),
            },
        ),
        Event::new(
            id.uuid(),
            10,
            WorkerEvent::Renamed {
                string: "name".to_owned(),
            },
        ),
    ])
    .unwrap();
    assert_eq!(store.entities().unwrap().collect::<Vec<_>>(), [id]);
    assert!(store.entity(missing).unwrap().is_none());
    assert!(
        matches!(store.event_sequence(missing), Err(Error::MissingEntity(uuid)) if uuid == missing.uuid())
    );
    let entity = store.entity(id).unwrap().unwrap();
    assert_eq!(entity.renamed().unwrap().string(), "name");
    assert_eq!(entity.update().next().unwrap().timestamp(), 20);
    let reference = EntityRef::<Worker, _>::new(id.uuid(), "reference data".to_owned());
    assert!(std::ptr::eq(
        entity,
        store.entity(&reference).unwrap().unwrap()
    ));
    assert!(std::ptr::eq(
        entity,
        store.entity(reference).unwrap().unwrap()
    ));
    assert!(std::ptr::eq(
        entity,
        store.entity(id.uuid()).unwrap().unwrap()
    ));
    assert_eq!(entity.earliest_timestamp(), 10);
    assert_eq!(entity.latest_timestamp(), 20);
    assert_eq!(entity.num_events().get(), 2);
}

// Checks field accessors preserve values and borrow the original string
// allocation.
#[test]
fn borrows_nested_fields() {
    let id = Uuid::from_u128(1);
    let parent_id = Uuid::from_u128(2);
    let name = "worker".to_owned();
    let name_pointer = name.as_ptr();
    let store = NativeStore::<Worker>::try_from_events([Event::new(
        id,
        10,
        WorkerEvent::Type {
            string: name,
            option: None,
            vec: vec!["label".to_owned()],
            some: true,
            r#type: "kind".to_owned(),
            http2_code: 200,
            uuid: parent_id,
            attributes: model::DynamicAttributes::default(),
            shared: model::Shared {
                label: "shared".to_owned(),
            },
            local: model::nested::Record { count: 3 },
            parent: EntityRef::new(parent_id, ()),
            peers: vec![EntityRef::new(
                parent_id,
                model::Shared {
                    label: "peer".to_owned(),
                },
            )],
        },
    )])
    .unwrap();

    let events = store.entity(id).unwrap().unwrap();
    let event = events.r#type().unwrap();
    assert_eq!(event.timestamp(), 10);
    assert_eq!(event.string().as_str(), "worker");
    assert_eq!(event.string().as_ptr(), name_pointer);
    assert_eq!(event.option(), &None);
    assert_eq!(event.vec().as_slice(), ["label"]);
    assert!(*event.some());
    assert_eq!(event.r#type().as_str(), "kind");
    assert_eq!(*event.http2_code(), 200);
    assert_eq!(*event.uuid(), parent_id);
    assert!(event.attributes().0.is_empty());
    assert_eq!(event.shared().label, "shared");
    assert_eq!(event.local().count, 3);
    assert_eq!(event.parent().target, parent_id);
    assert_eq!(event.peers()[0].data.label, "peer");
}

// Checks interleaved event types retain timestamp order and equal-timestamp
// order.
#[test]
fn groups_multi_events_stably_and_leaves_once_events_absent() {
    let id = Uuid::from_u128(1);
    let store = NativeStore::<Worker>::try_from_events([
        Event::new(
            id,
            20,
            WorkerEvent::Update {
                string: "later".into(),
            },
        ),
        Event::new(
            id,
            10,
            WorkerEvent::Update {
                string: "first".into(),
            },
        ),
        Event::new(
            id,
            10,
            WorkerEvent::Update {
                string: "second".into(),
            },
        ),
        Event::new(
            id,
            15,
            WorkerEvent::Renamed {
                string: "renamed".into(),
            },
        ),
    ])
    .unwrap();
    let events = store.entity(id).unwrap().unwrap();
    assert!(events.r#type().is_none());
    assert_eq!(events.renamed().unwrap().timestamp(), 15);
    assert_eq!(
        events
            .update()
            .map(|event| (event.timestamp(), event.string().as_str()))
            .collect::<Vec<_>>(),
        [(10, "first"), (10, "second"), (20, "later")]
    );
}

// Checks sole once-events provide direct access and reject duplicates at any
// timestamp.
#[test]
fn rejects_duplicates_including_equal_timestamps_and_empty_payloads() {
    use self::model::{Parent, ParentEvent, parent::ParentEvents};
    let id = Uuid::from_u128(1);
    let single =
        NativeStore::<Parent>::try_from_events([Event::new(id, 10, ParentEvent::Created)]).unwrap();
    let grouped = single.entity(id).unwrap().unwrap();
    assert_eq!(grouped.created().timestamp(), 10);
    for timestamp in [10, 20] {
        let store = NativeStore::<Parent>::try_from_events([
            Event::new(id, 10, ParentEvent::Created),
            Event::new(id, timestamp, ParentEvent::Created),
        ]);
        assert!(matches!(
            store,
            Err(Error::EntityIndex {
                entity_id,
                source,
            }) if entity_id == id
                && matches!(*source, Error::DuplicateOnceEvent { event_name: "created" })
        ));
    }
}

// Checks a sole multi-event type returns every occurrence in timestamp order.
#[test]
fn a_sole_multi_event_still_returns_all_occurrences() {
    use self::model::{Stream, StreamEvent, stream::StreamEvents};
    let id = Uuid::from_u128(4);
    let store = NativeStore::<Stream>::try_from_events(
        [20, 10, 10].map(|timestamp| Event::new(id, timestamp, StreamEvent::Tick)),
    )
    .unwrap();
    let events = store.entity(id).unwrap().unwrap();
    assert_eq!(
        events
            .tick()
            .map(|event| event.timestamp())
            .collect::<Vec<_>>(),
        [10, 10, 20]
    );
}

// Checks timestamp and other payload fields remain accessible under distinct
// names.
#[test]
fn payload_fields_do_not_shadow_the_recorded_timestamp() {
    let id = Uuid::from_u128(3);
    let store = NativeStore::<Worker>::try_from_events([Event::new(
        id,
        42,
        WorkerEvent::Metadata {
            id: 7,
            timestamp: 8,
            data: "data".to_owned(),
            event_id: "event_id".to_owned(),
            event_timestamp: "event_timestamp".to_owned(),
            field_0: "field_0".to_owned(),
        },
    )])
    .unwrap();
    let grouped = store.entity(id).unwrap().unwrap();
    let event = grouped.metadata().unwrap();
    assert_eq!(event.timestamp(), 42);
    assert_eq!(*event.id(), 7);
    assert_eq!(*event.field_timestamp(), 8);
    assert_eq!(event.data().as_str(), "data");
    assert_eq!(event.event_id().as_str(), "event_id");
    assert_eq!(event.event_timestamp().as_str(), "event_timestamp");
    assert_eq!(event.field_0().as_str(), "field_0");
}

// Checks handle conversion rejects the wrong variant and borrows matching
// payloads.
#[test]
fn checked_handles_reject_other_variants_and_borrow_valid_events() {
    use self::model::nested::r#type::worker::RenamedHandle;

    let event = Timestamped::new(
        30,
        WorkerEvent::Update {
            string: "update".to_owned(),
        },
    );
    let rejected = RenamedHandle::try_from(&event).err().unwrap();
    assert!(std::ptr::eq(rejected, &event));

    let event = Timestamped::new(
        40,
        WorkerEvent::Renamed {
            string: "renamed".to_owned(),
        },
    );
    let handle = RenamedHandle::try_from(&event).unwrap();
    assert_eq!(handle.timestamp(), 40);
    let WorkerEvent::Renamed { string } = &event.data else {
        unreachable!()
    };
    assert!(std::ptr::eq(handle.string(), string));
}
