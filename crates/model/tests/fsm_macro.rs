// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Integration test for the `fsm!` proc macro.

use quent_model::{EventSender, FsmEvent, ModelBuilder, ModelComponent, StateMetadata};
use uuid::Uuid;

// States via state! macro

quent_model::state! {
    Queued {
        attributes: {
            priority: u32,
        },
    }
}

quent_model::state! {
    Running {
        attributes: {
            worker_id: Uuid,
        },
    }
}

// FSM via the fsm! macro

quent_model::fsm! {
    Task {
        states: {
            queued: Queued,
            running: Running,
        },
        entry: queued,
        exit_from: { running },
        transitions: {
            queued => running,
        },
    }
}

// Tests

#[test]
fn transition_enum_exists() {
    let _q = TaskTransition::Queued(Queued {
        instance_name: "test".to_string(),
        priority: 1,
    });
    let _r = TaskTransition::Running(Running {
        instance_name: "test".to_string(),
        worker_id: Uuid::nil(),
    });
    let _e = TaskTransition::Exit;
}

#[test]
fn from_impls() {
    let q = Queued {
        instance_name: "test".to_string(),
        priority: 1,
    };
    let _t: TaskTransition = q.into();
}

#[test]
fn event_type_alias() {
    let _event: TaskEvent = FsmEvent {
        seq: 0,
        state: TaskTransition::Exit,
    };
}

#[test]
fn model_component_collects() {
    let mut builder = ModelBuilder::new("test");
    Task::collect(&mut builder);

    assert_eq!(builder.fsms.len(), 1);
    let fsm = &builder.fsms[0];
    assert_eq!(fsm.name, "task");
    assert_eq!(fsm.states.len(), 2);
    // entry->queued, queued->running, running->exit = 3 transitions
    assert_eq!(fsm.transitions.len(), 3);
}

#[test]
fn state_metadata() {
    assert_eq!(Queued::state_name(), "queued");
    assert_eq!(Running::state_name(), "running");
}

#[test]
fn observer_entry_method() {
    let tx: EventSender<TaskEvent> = EventSender::default();
    let observer = TaskObserver::new(&tx);
    let id = Uuid::nil();
    let mut handle = observer.queued(id, "my_task", 5);
    assert_eq!(handle.uuid(), id);
    handle.exit();
}

#[test]
fn handle_transition_method() {
    let tx: EventSender<TaskEvent> = EventSender::default();
    let observer = TaskObserver::new(&tx);
    let id = Uuid::nil();
    let mut handle = observer.queued(id, "my_task", 5);
    handle.running("my_task", Uuid::nil());
    handle.exit();
}
