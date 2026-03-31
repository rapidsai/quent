// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Memory resource FSM definitions.

#[allow(unused_imports)]
use quent_model::prelude::*;

// --- Fixed-bounds Memory ---

#[quent_model(state)]
pub struct MemoryInitializing;

#[quent_model(state)]
pub struct MemoryOperating {
    #[capacity]
    pub capacity_bytes: u64,
}

#[quent_model(state)]
pub struct MemoryFinalizing;

/// A fixed-bounds memory resource FSM handle.
///
/// FSM: `entry -> initializing -> operating -> finalizing -> exit`
///
/// The transition into `operating` declares the capacity in bytes.
/// Use `MemoryResource` with `Usage<MemoryResource>` to reference this
/// resource type from FSM states.
#[quent_model(fsm(
    resource(capacity = MemoryOperating),
    entry -> MemoryInitializing,
    MemoryInitializing -> MemoryOperating,
    MemoryOperating -> MemoryFinalizing,
    MemoryFinalizing -> exit,
))]
pub struct Memory;

// --- Dynamic-bounds Memory ---

#[quent_model(state)]
pub struct DynMemoryInitializing;

#[quent_model(state)]
pub struct DynMemoryOperating {
    #[capacity]
    pub capacity_bytes: u64,
}

#[quent_model(state)]
pub struct DynMemoryResizing;

#[quent_model(state)]
pub struct DynMemoryFinalizing;

/// A dynamic-bounds memory resource that supports resizing.
///
/// FSM: `entry -> initializing -> operating <-> resizing, operating -> finalizing -> exit`
#[quent_model(fsm(
    resource(capacity = DynMemoryOperating),
    entry -> DynMemoryInitializing,
    DynMemoryInitializing -> DynMemoryOperating,
    DynMemoryOperating -> DynMemoryResizing,
    DynMemoryResizing -> DynMemoryOperating,
    DynMemoryOperating -> DynMemoryFinalizing,
    DynMemoryFinalizing -> exit,
))]
pub struct DynamicMemory;
