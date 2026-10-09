// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_simulator_store::{
    EntityRef, Event, Uuid, Worker, WorkerEvent,
    entity_events::worker::{NativeWorkerEvents, WorkerEvents},
};
use quent_store::entity::{DuplicateOnceEvent, EntityHandle, native::Store};

#[test]
fn reads_worker_events_without_an_accumulator() {
    let id = Uuid::from_u128(1);
    let parent_id = Uuid::from_u128(2);
    let store = Store::<Worker>::new([
        Event::new(id, 20, WorkerEvent::Exit),
        Event::new(
            id,
            10,
            WorkerEvent::Init {
                parent_engine_id: EntityRef::new(parent_id, ()),
                instance_name: "worker".to_owned(),
            },
        ),
    ]);
    let events = NativeWorkerEvents::try_from(store.into_sequences().next().unwrap()).unwrap();
    assert_eq!(events.id(), id);
    assert_eq!(events.init().unwrap().data.instance_name, "worker");
    let parent: &EntityRef<quent_simulator_store::Engine> =
        &events.init().unwrap().data.parent_engine_id;
    assert_eq!(parent.target, parent_id);
    assert_eq!(events.init().unwrap().timestamp, 10);
    assert_eq!(events.exit().unwrap().timestamp, 20);
}

#[test]
fn rejects_duplicate_worker_init_events() {
    let id = Uuid::from_u128(1);
    let store = Store::<Worker>::new([10, 20].map(|timestamp| {
        Event::new(
            id,
            timestamp,
            WorkerEvent::Init {
                parent_engine_id: EntityRef::new(Uuid::from_u128(2), ()),
                instance_name: "worker".to_owned(),
            },
        )
    }));
    assert_eq!(
        NativeWorkerEvents::try_from(store.into_sequences().next().unwrap()).err(),
        Some(DuplicateOnceEvent {
            entity_id: id,
            event_name: "init"
        })
    );
}
