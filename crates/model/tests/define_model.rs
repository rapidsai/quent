// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for `define_model!` macro.

use quent_model::FsmEvent;
use quent_model::prelude::*;

// Minimal model components

#[derive(Debug, Clone, State)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Idle;

#[derive(Debug, Clone, State)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Active;

#[allow(dead_code)]
#[derive(Fsm)]
pub struct SimpleFsm {
    #[entry]
    #[to(Active)]
    pub idle: Idle,
    #[to(exit)]
    pub active: Active,
}

#[derive(Debug, Event)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ping {
    pub value: u64,
}

#[allow(dead_code)]
#[derive(Entity)]
pub struct SimpleEntity {
    pub ping: EmitOnce<Ping>,
}

// define_model! usage

quent_model::define_model! {
    Test {
        SimpleFsm,
        SimpleEntity,
    }
}

#[test]
fn define_model_generates_event_enum() {
    // The enum TestEvent should have variants for each component
    let _fsm_event: TestEvent = TestEvent::SimpleFsm(FsmEvent::Transition {
        seq: 0,
        state: SimpleFsmTransition::Exit,
    });
}

#[test]
fn define_model_generates_model_type() {
    let builder = TestModel::build();
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.entities.len(), 1);
}

#[test]
fn define_model_from_impls() {
    // SimpleFsmEvent should convert into TestEvent
    let fsm_event: SimpleFsmEvent = FsmEvent::Transition {
        seq: 0,
        state: SimpleFsmTransition::Exit,
    };
    let _: TestEvent = fsm_event.into();

    // SimpleEntityEvent should convert into TestEvent
    let entity_event: SimpleEntityEvent = SimpleEntityEvent::Ping(Ping { value: 42 });
    let _: TestEvent = entity_event.into();
}
