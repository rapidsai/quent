// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for FSM validation (compile-time checks via trybuild would be ideal,
//! but for now we verify the validation logic at runtime through the model
//! metadata and by testing valid FSMs compile correctly).
//!
//! Invalid FSMs (missing entry, unreachable states, etc.) produce compile
//! errors via `syn::Error` in the proc macro. The valid cases below confirm
//! that the validation passes for well-formed FSMs.
//!
//! TODO: Add `trybuild` compile-fail tests for invalid FSM definitions:
//! - `fsm_no_entry.rs` — FSM with no `#[entry]` field
//! - `fsm_unreachable_state.rs` — FSM with a state not reachable from entry
//! - `fsm_no_exit.rs` — FSM with no path to exit

use quent_model::prelude::*;

// --- A valid linear FSM ---

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct A;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct B;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct C;

#[derive(Fsm)]
pub struct LinearFsm {
    #[entry] #[to(B)]
    pub a: A,
    #[to(C)]
    pub b: B,
    #[to(exit)]
    pub c: C,
}

#[test]
fn linear_fsm_valid() {
    let mut builder = ModelBuilder::new();
    LinearFsm::collect(&mut builder);
    let fsm = &builder.fsms[0];
    assert_eq!(fsm.states.len(), 3);
    assert_eq!(fsm.transitions.len(), 4);
}

// --- A cyclic FSM ---

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Idle;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Working;

#[derive(Fsm)]
pub struct CyclicFsm {
    #[entry] #[to(Working)]
    pub idle: Idle,
    #[to(Idle, exit)]
    pub working: Working,
}

#[test]
fn cyclic_fsm_valid() {
    let mut builder = ModelBuilder::new();
    CyclicFsm::collect(&mut builder);
    let fsm = &builder.fsms[0];
    assert_eq!(fsm.states.len(), 2);
    assert_eq!(fsm.transitions.len(), 4);
}

// --- Unit state (no fields) ---

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct EmptyState;

#[test]
fn unit_state_metadata() {
    let def = EmptyState::state_def();
    assert_eq!(def.name, "empty_state");
    assert!(def.attributes.is_empty());
    assert!(def.deferred_attributes.is_empty());
    assert!(def.usages.is_empty());
}

// --- Complex model with nested composition ---

type InnerModel = Model<(LinearFsm,)>;
type OuterModel = Model<(InnerModel, CyclicFsm)>;

#[test]
fn nested_model_composition() {
    let builder = OuterModel::build();
    assert_eq!(builder.fsms.len(), 2);
    assert!(builder.fsms.iter().any(|f| f.name == "linear_fsm"));
    assert!(builder.fsms.iter().any(|f| f.name == "cyclic_fsm"));
}

// --- Transition endpoint values ---

#[test]
fn transition_endpoints() {
    let mut builder = ModelBuilder::new();
    LinearFsm::collect(&mut builder);
    let fsm = &builder.fsms[0];

    // First transition: entry -> A
    assert_eq!(fsm.transitions[0].from, quent_model::TransitionEndpoint::Entry);
    assert_eq!(
        fsm.transitions[0].to,
        quent_model::TransitionEndpoint::State("a".to_string())
    );

    // Last transition: C -> exit
    assert_eq!(
        fsm.transitions[3].from,
        quent_model::TransitionEndpoint::State("c".to_string())
    );
    assert_eq!(fsm.transitions[3].to, quent_model::TransitionEndpoint::Exit);
}
