// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_store::{
    EntityHandle, EntityStore, Timestamp, Timestamped,
    entity::native::NativeStore,
    fsm::{FsmHandle, FsmStateHandle, Sequence, TransitionHandle},
};

#[allow(unused_imports, dead_code)]
mod model {
    include!(concat!(env!("OUT_DIR"), "/model.rs"));
}

use model::{
    Event, Uuid,
    nested::r#type::{
        Task, TaskEvent,
        task::{RunningHandle, TaskEvents, TaskTransitionHandle},
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
    assert!(task.is_valid());
    assert_eq!(task.states().len(), 3);
    assert_eq!(
        task.states().map(|state| state.name()).collect::<Vec<_>>(),
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
    let reconstructed_first =
        FsmStateHandle::try_new(*first.opening_transition(), *first.closing_transition()).unwrap();
    assert_eq!(reconstructed_first.span(), first.span());
    let last = task.last().unwrap();
    assert_eq!(last.span().start(), 10);
    assert_eq!(last.span().end(), 20);
    assert!(last.closing_transition().is_final());
    assert_eq!(last.closing_transition().name(), "done");
    assert_eq!(last.closing_transition().timestamp(), 20);
    let reconstructed_last =
        FsmStateHandle::try_new(*last.opening_transition(), *last.closing_transition()).unwrap();
    assert_eq!(reconstructed_last.span(), last.span());
    assert_eq!(reconstructed_last.name(), last.name());
    assert!(reconstructed_last.closing_transition().is_final());
    assert!(
        FsmStateHandle::try_new(*last.closing_transition(), *last.opening_transition()).is_err()
    );
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
    assert_eq!(TransitionHandle::name(&task.r#type().unwrap()), "type");
    assert!(TransitionHandle::is_final(&task.done().unwrap()));
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
        assert!(!task.is_valid());
        assert_eq!(task.num_states(), 0);
        assert!(task.first().is_none());
        assert!(task.last().is_none());
        assert_eq!(task.states().count(), 0);
        let transition = task.transition(0).unwrap();
        assert_eq!(transition.is_initial(), transition.name() == "type");
        assert_eq!(
            transition.is_final(),
            matches!(transition.name(), "done" | "failed")
        );
    }
}

#[test]
fn attributes_do_not_shadow_transition_properties() {
    let store = NativeStore::<Task>::try_from_events([event(
        10,
        TaskEvent::Key {
            seq: 7,
            label: String::new(),
            name: "attribute name".into(),
            sequence: 99,
            is_final: false,
            is_initial: true,
            is_valid_next: true,
            timestamp: 123,
        },
    )])
    .unwrap();
    let task = store.entity(Uuid::from_u128(1)).unwrap().unwrap();
    let key = task.key().unwrap();
    assert_eq!(key.name(), "key");
    assert_eq!(key.sequence(), 7);
    assert!(key.is_final());
    assert_eq!(key.timestamp(), 10);
    assert_eq!(key.field_name(), "attribute name");
    assert_eq!(*key.field_sequence(), 99);
    assert!(!key.field_is_final());
    assert!(key.field_is_initial());
    assert!(!key.is_initial());
    assert!(key.field_is_valid_next());
    assert!(!key.is_valid_next(&key));
    assert_eq!(*key.field_timestamp(), 123);
}

#[test]
fn checks_transition_pairs_against_the_topology() {
    let payloads = [
        TaskEvent::Type {
            seq: 0,
            label: String::new(),
        },
        TaskEvent::Running {
            seq: 1,
            label: String::new(),
        },
        TaskEvent::Running {
            seq: 2,
            label: String::new(),
        },
        TaskEvent::Done {
            seq: 3,
            label: String::new(),
        },
        TaskEvent::Failed {
            seq: 4,
            label: String::new(),
        },
        TaskEvent::Key {
            seq: 5,
            label: String::new(),
            name: String::new(),
            sequence: 0,
            is_final: false,
            is_initial: false,
            is_valid_next: false,
            timestamp: 0,
        },
    ];
    let store = NativeStore::<Task>::try_from_events(
        payloads.into_iter().map(|payload| event(10, payload)),
    )
    .unwrap();
    let task = store.entity(Uuid::from_u128(1)).unwrap().unwrap();
    let allowed = [
        [false, true, true, false, false, false],
        [false, false, true, true, true, true],
        [false, false, false, true, true, true],
        [false; 6],
        [false; 6],
        [false; 6],
    ];
    for (source, targets) in allowed.iter().enumerate() {
        let transition = task.transition(source).unwrap();
        assert_eq!(transition.is_initial(), source == 0);
        for (target, expected) in targets.iter().enumerate() {
            let next = task.transition(target).unwrap();
            assert_eq!(
                transition.is_valid_next(&next),
                *expected,
                "{} -> {}",
                transition.name(),
                next.name()
            );
        }
    }
    let running = task.running().collect::<Vec<_>>();
    assert!(!running[0].is_initial());
    assert!(running[0].is_valid_next(&running[1]));
    assert!(!running[1].is_valid_next(&running[0]));
    assert!(!running[0].is_valid_next(&running[0]));
    let initial = task.r#type().unwrap();
    assert!(initial.is_initial());
    assert!(!initial.is_valid_next(&initial));
    let done = task.done().unwrap();
    assert!(!done.is_initial());
    assert!(!done.is_valid_next(&done));
}

#[test]
fn checks_transition_pair_ordering() {
    for (first, next, expected) in [
        ((10, 1), (10, 2), true),
        ((10, 1), (10, 1), false),
        ((10, 2), (10, 1), false),
        ((11, 0), (10, 1), false),
        ((10, u16::MAX), (11, 0), true),
        ((10, 1), (11, 1), true),
    ] {
        let payload = |(timestamp, seq)| {
            Timestamped::new(
                timestamp,
                TaskEvent::Running {
                    seq,
                    label: String::new(),
                },
            )
        };
        let first_event = payload(first);
        let next_event = payload(next);
        let first_handle = RunningHandle::try_from(&first_event).unwrap();
        let next_handle = RunningHandle::try_from(&next_event).unwrap();
        assert_eq!(
            first_handle.is_valid_next(&next_handle),
            expected,
            "{first:?} -> {next:?}"
        );
        let first_handle = TaskTransitionHandle::Running(first_handle);
        let next_handle = TaskTransitionHandle::Running(next_handle);
        assert_eq!(
            first_handle.is_valid_next(&next_handle),
            expected,
            "{first:?} -> {next:?}"
        );
    }
}

#[test]
fn validates_recordings_without_rejecting_them_from_the_store() {
    for (name, expected, events) in [
        (
            "complete recording with equal timestamps",
            true,
            vec![(10, "type", 0), (10, "running", 1), (10, "done", 2)],
        ),
        (
            "missing initial transition",
            false,
            vec![(10, "running", 1), (20, "done", 2)],
        ),
        (
            "missing final transition",
            false,
            vec![(10, "type", 0), (20, "running", 1)],
        ),
        (
            "missing both boundaries",
            false,
            vec![(10, "running", 1), (20, "running", 2)],
        ),
        (
            "illegal topology edge",
            false,
            vec![(10, "type", 0), (20, "done", 1)],
        ),
        (
            "duplicate ordering key on a legal self-loop",
            false,
            vec![
                (10, "type", 0),
                (10, "running", 1),
                (10, "running", 1),
                (20, "done", 2),
            ],
        ),
        (
            "duplicate ordering key across event types",
            false,
            vec![(10, "type", 0), (10, "running", 0), (20, "done", 1)],
        ),
        (
            "sequence rollover across timestamps",
            true,
            vec![(10, "type", u16::MAX), (11, "running", 0), (12, "done", 1)],
        ),
        (
            "sequence reuse across timestamps",
            true,
            vec![(10, "type", 0), (11, "running", 0), (12, "done", 0)],
        ),
        (
            "alternative final state with sequence gaps",
            true,
            vec![(10, "type", 1), (11, "running", 3), (12, "failed", 5)],
        ),
    ] {
        let events = events.into_iter().rev().map(|(timestamp, state, seq)| {
            let label = String::new();
            let payload = match state {
                "type" => TaskEvent::Type { seq, label },
                "running" => TaskEvent::Running { seq, label },
                "done" => TaskEvent::Done { seq, label },
                "failed" => TaskEvent::Failed { seq, label },
                _ => unreachable!(),
            };
            event(timestamp, payload)
        });
        let store = NativeStore::<Task>::try_from_events(events).unwrap();
        let task = store.entity(Uuid::from_u128(1)).unwrap().unwrap();
        assert_eq!(task.is_valid(), expected, "{name}");
    }
}
