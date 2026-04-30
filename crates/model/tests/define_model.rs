// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for `model!` macro.

use quent_model::{Attributes, FsmEvent};

// Minimal model components

quent_model::state! {
    Idle {}
}

quent_model::state! {
    Active {}
}

quent_model::fsm! {
    SimpleFsm {
        states: { idle: Idle, active: Active },
        entry: idle,
        exit_from: { active },
        transitions: { idle => active },
    }
}

#[derive(Debug, Attributes)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ping {
    pub value: u64,
}

quent_model::entity! {
    SimpleEntity {
        events: {
            ping: Ping,
        },
    }
}

quent_model::entity! {
    TestRoot: ResourceGroup<Root = true> {}
}

quent_model::model! {
    Test {
        root: TestRoot,
        SimpleFsm,
        SimpleEntity,
    }
}

#[test]
fn define_model_generates_event_enum() {
    // The enum TestEvent should have variants for each component
    let _fsm_event: TestEvent = TestEvent::SimpleFsm(FsmEvent {
        seq: 0,
        state: SimpleFsmTransition::Exit,
    });
}

#[test]
fn define_model_generates_model_type() {
    let builder = TestModel::build("Test");
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.entities.len(), 2); // TestRoot + SimpleEntity
}

#[test]
fn define_model_from_impls() {
    // SimpleFsmEvent should convert into TestEvent
    let fsm_event: SimpleFsmEvent = FsmEvent {
        seq: 0,
        state: SimpleFsmTransition::Exit,
    };
    let _: TestEvent = fsm_event.into();

    // SimpleEntityEvent should convert into TestEvent
    let entity_event: SimpleEntityEvent = SimpleEntityEvent::Ping(Ping { value: 42 });
    let _: TestEvent = entity_event.into();
}
