// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! FSM type declarations exposed to UI clients.

use serde::Serialize;
use ts_rs::TS;

/// Provides an FSM type declaration for UI clients.
pub trait FsmTypeDeclaration {
    fn fsm_type_declaration() -> FsmTypeDecl;
}

/// Declares an FSM state for UI clients.
#[derive(Debug, Serialize, TS)]
pub struct FsmStateTypeDecl {
    /// The state name.
    pub name: String,
    /// The resource usage names available in this state.
    pub usages: Vec<String>,
}

/// Declares a possible FSM state transition for UI clients.
#[derive(Debug, Serialize, TS)]
pub enum FsmTransitionDecl {
    /// An initial transition into the named state.
    Entry(String),
    /// A transition between the named source and destination states.
    Transition(String, String),
    /// A final transition out of the named state.
    Exit(String),
}

/// Declares an FSM type for UI clients.
#[derive(Debug, Serialize, TS)]
pub struct FsmTypeDecl {
    /// The FSM type name.
    pub name: String,
    /// The FSM states.
    pub states: Vec<FsmStateTypeDecl>,
    /// The possible FSM transitions.
    pub transitions: Vec<FsmTransitionDecl>,
}
