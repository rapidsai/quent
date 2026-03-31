// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Basic integration test for FSM and state macro code generation.

use quent_model::prelude::*;

// --- Define states ---

#[quent_model::state]
pub struct Queueing {
    pub operator_id: Uuid,
    pub instance_name: String,
}

#[quent_model::state]
pub struct Computing {
    pub value: u64,
    #[quent_model::deferred]
    pub rows_processed: Option<u64>,
}

#[quent_model::state]
pub struct Sending {
    pub channel_id: Uuid,
}

// --- Define FSM ---

#[quent_model::fsm(
    entry -> Queueing,
    Queueing -> Computing,
    Computing -> Sending,
    Sending -> Queueing,
    Computing -> exit,
)]
pub struct Task;

// --- Tests ---

#[test]
fn transition_enum_variants_exist() {
    let _q = TaskTransition::Queueing(Queueing {
        operator_id: Uuid::nil(),
        instance_name: "test".to_string(),
    });
    let _c = TaskTransition::Computing(Computing {
        value: 42,
        rows_processed: None,
    });
    let _s = TaskTransition::Sending(Sending {
        channel_id: Uuid::nil(),
    });
    let _e = TaskTransition::Exit;
}

#[test]
fn from_impl_works() {
    let q = Queueing {
        operator_id: Uuid::nil(),
        instance_name: "test".to_string(),
    };
    let _t: TaskTransition = q.into();
}

#[test]
fn deferred_enum_exists() {
    // Computing has a deferred field, so ComputingDeferred should have a variant
    let _d = ComputingDeferred::RowsProcessed(42);

    // TaskDeferred wraps per-state deferred types
    let _td = TaskDeferred::Computing(ComputingDeferred::RowsProcessed(42));
}

#[test]
fn event_type_alias_works() {
    let _event: TaskEvent = FsmEvent::Transition {
        seq: 0,
        state: TaskTransition::Exit,
    };

    let _deferred: TaskEvent = FsmEvent::Deferred {
        seq: 1,
        deferred: TaskDeferred::Computing(ComputingDeferred::RowsProcessed(100)),
    };
}

#[test]
fn state_metadata() {
    assert_eq!(Queueing::state_name(), "queueing");
    assert_eq!(Computing::state_name(), "computing");
    assert_eq!(Sending::state_name(), "sending");

    let def = Computing::state_def();
    assert_eq!(def.name, "computing");
    assert_eq!(def.deferred_attributes.len(), 1);
    assert_eq!(def.deferred_attributes[0].name, "rows_processed");
}

#[test]
fn model_component_collects_fsm() {
    let mut builder = ModelBuilder::new();
    Task::collect(&mut builder);

    assert_eq!(builder.fsms.len(), 1);
    let fsm = &builder.fsms[0];
    assert_eq!(fsm.name, "task");
    assert_eq!(fsm.states.len(), 3); // Queueing, Computing, Sending
    assert_eq!(fsm.transitions.len(), 5); // entry->Q, Q->C, C->S, S->Q, C->exit
}

#[test]
fn model_composition() {
    type TestModel = Model<(Task,)>;

    let builder = TestModel::build();
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.fsms[0].name, "task");
}
