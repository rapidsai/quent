// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Basic integration test for FSM and state macro code generation.

use quent_model::{FsmEvent, Model, ModelBuilder, ModelComponent, StateMetadata};
use uuid::Uuid;

// Define states

quent_model::state! {
    Queueing {
        attributes: {
            operator_id: Uuid,
        },
    }
}

quent_model::state! {
    Computing {
        attributes: {
            value: u64,
            rows_processed: Option<u64>,
        },
    }
}

quent_model::state! {
    Sending {
        attributes: {
            channel_id: Uuid,
        },
    }
}

// Define FSM

quent_model::fsm! {
    Task {
        states: {
            queueing: Queueing,
            computing: Computing,
            sending: Sending,
        },
        entry: queueing,
        exit_from: { computing },
        transitions: {
            queueing => computing,
            computing => sending,
            sending => queueing,
        },
    }
}

// Tests

#[test]
fn transition_enum_variants_exist() {
    let _q = TaskTransition::Queueing(Queueing {
        instance_name: "test".to_string(),
        operator_id: Uuid::nil(),
    });
    let _c = TaskTransition::Computing(Computing {
        instance_name: "test".to_string(),
        value: 42,
        rows_processed: None,
    });
    let _s = TaskTransition::Sending(Sending {
        instance_name: "test".to_string(),
        channel_id: Uuid::nil(),
    });
    let _e = TaskTransition::Exit;
}

#[test]
fn from_impl_works() {
    let q = Queueing {
        instance_name: "test".to_string(),
        operator_id: Uuid::nil(),
    };
    let _t: TaskTransition = q.into();
}

#[test]
fn event_type_alias_works() {
    let _event: TaskEvent = FsmEvent::Transition {
        seq: 0,
        state: TaskTransition::Exit,
    };
}

#[test]
fn state_metadata() {
    assert_eq!(Queueing::state_name(), "queueing");
    assert_eq!(Computing::state_name(), "computing");
    assert_eq!(Sending::state_name(), "sending");

    let def = Computing::state_def();
    assert_eq!(def.name, "computing");
}

#[test]
fn model_component_collects_fsm() {
    let mut builder = ModelBuilder::new();
    Task::collect(&mut builder);

    assert_eq!(builder.fsms.len(), 1);
    let fsm = &builder.fsms[0];
    assert_eq!(fsm.name, "task");
    assert_eq!(fsm.states.len(), 3); // Queueing, Computing, Sending
    assert_eq!(fsm.transitions.len(), 5); // entry->Q, Q->C, C->S, C->exit, S->Q
}

#[test]
fn model_composition() {
    type TestModel = Model<(Task,)>;

    let builder = TestModel::build();
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.fsms[0].name, "task");
}
