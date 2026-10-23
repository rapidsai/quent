// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_store::{
    EntityHandle, EntityStore, Timestamp,
    entity::native::NativeStore,
    fsm::{FsmHandle, Sequence, Transition},
};

#[allow(unused_imports, dead_code)]
mod model {
    include!(concat!(env!("OUT_DIR"), "/model.rs"));
}

use model::{
    Event, Uuid,
    nested::r#type::{
        Task, TaskEvent,
        task::{TaskEvents, TaskTransitionHandle},
    },
};

fn event(timestamp: u64, data: TaskEvent) -> Event<TaskEvent> {
    Event::new(Uuid::from_u128(1), timestamp, data)
}

#[test]
fn orders_fsm_events_and_borrows_typed_state_transitions() {
    let label = "running".to_owned();
    let label_pointer = label.as_ptr();
    let store = NativeStore::<Task>::try_from_events([
        event(
            20,
            TaskEvent::Done {
                seq: 3,
                label: "done".into(),
            },
        ),
        event(
            10,
            TaskEvent::Running {
                seq: 2,
                label: "again".into(),
            },
        ),
        event(10, TaskEvent::Running { seq: 1, label }),
        event(
            10,
            TaskEvent::Type {
                seq: 0,
                label: "start".into(),
            },
        ),
    ])
    .unwrap();
    let task = store.entity(Uuid::from_u128(1)).unwrap().unwrap();
    assert_eq!(task.num_events().get(), 4);
    assert_eq!(task.num_states(), 3);
    assert!(!task.is_empty());
    assert_eq!(task.states().len(), 3);
    assert_eq!(
        task.states()
            .map(|state| state.name().to_owned())
            .collect::<Vec<_>>(),
        ["type", "running", "running"]
    );
    assert!(task.transition(4).is_none());
    assert!(task.state(3).is_none());
    assert!(task.state(usize::MAX).is_none());
    let first = task.first().unwrap();
    assert_eq!(first.span().start(), 10);
    assert_eq!(first.span().end(), 10);
    assert_eq!(first.opening_transition().sequence(), 0);
    assert!(!first.opening_transition().is_final());
    let last = task.last().unwrap();
    assert_eq!(last.span().start(), 10);
    assert_eq!(last.span().end(), 20);
    assert!(last.closing_transition().is_final());
    assert_eq!(last.closing_transition().name(), "done");
    assert_eq!(last.closing_transition().timestamp(), 20);
    let state = task.state(1).unwrap();
    let TaskTransitionHandle::Running(running) = state.opening_transition() else {
        panic!("expected running transition");
    };
    assert_eq!(running.label().as_ptr(), label_pointer);
    assert_eq!(Sequence::sequence(running), 1);
    assert_eq!(
        task.running().map(|event| *event.seq()).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(Transition::name(&task.r#type().unwrap()), "type");
    assert!(Transition::is_final(&task.done().unwrap()));
    assert_eq!(task.transition(1).unwrap().sequence(), 1);
    fn through_handle<H: FsmHandle>(handle: H) -> usize {
        handle.num_states()
    }
    assert_eq!(through_handle(task), 3);
}

#[test]
fn orders_across_sequence_rollover_by_timestamp() {
    let store = NativeStore::<Task>::try_from_events([
        event(
            11,
            TaskEvent::Done {
                seq: 0,
                label: String::new(),
            },
        ),
        event(
            10,
            TaskEvent::Running {
                seq: u16::MAX,
                label: String::new(),
            },
        ),
    ])
    .unwrap();
    let task = store.entity(Uuid::from_u128(1)).unwrap().unwrap();
    let state = task.first().unwrap();
    assert_eq!(state.opening_transition().sequence(), u16::MAX);
    assert_eq!(state.closing_transition().sequence(), 0);
    assert_eq!(state.span().start(), 10);
    assert_eq!(state.span().end(), 11);
}

#[test]
fn accepts_partial_recordings_and_all_final_states() {
    for payload in [
        TaskEvent::Type {
            seq: 0,
            label: String::new(),
        },
        TaskEvent::Running {
            seq: 1,
            label: String::new(),
        },
        TaskEvent::Done {
            seq: 2,
            label: String::new(),
        },
        TaskEvent::Failed {
            seq: 2,
            label: String::new(),
        },
    ] {
        let store = NativeStore::<Task>::try_from_events([event(10, payload)]).unwrap();
        let task = store.entity(Uuid::from_u128(1)).unwrap().unwrap();
        assert!(task.is_empty());
        assert_eq!(task.num_states(), 0);
        assert!(task.first().is_none());
        assert!(task.last().is_none());
        assert_eq!(task.states().count(), 0);
        let transition = task.transition(0).unwrap();
        assert_eq!(
            transition.is_final(),
            matches!(transition.name(), "done" | "failed")
        );
    }
}
