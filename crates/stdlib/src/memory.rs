// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Memory resource FSM definitions.

#[allow(unused_imports)]
use quent_model::prelude::*;

// --- Fixed-bounds Memory ---

#[quent_model::state]
pub struct MemoryInitializing;

#[quent_model::state]
pub struct MemoryOperating {
    pub capacity_bytes: u64,
}

#[quent_model::state]
pub struct MemoryFinalizing;

/// A fixed-bounds memory resource.
///
/// FSM: `entry -> initializing -> operating -> finalizing -> exit`
///
/// The transition into `operating` declares the capacity in bytes.
#[quent_model::fsm(
    entry -> MemoryInitializing,
    MemoryInitializing -> MemoryOperating,
    MemoryOperating -> MemoryFinalizing,
    MemoryFinalizing -> exit,
)]
#[quent_model::resource(capacity = MemoryOperating)]
pub struct Memory;

// --- Dynamic-bounds Memory ---

#[quent_model::state]
pub struct DynMemoryInitializing;

#[quent_model::state]
pub struct DynMemoryOperating {
    pub capacity_bytes: u64,
}

#[quent_model::state]
pub struct DynMemoryResizing;

#[quent_model::state]
pub struct DynMemoryFinalizing;

/// A dynamic-bounds memory resource that supports resizing.
///
/// FSM: `entry -> initializing -> operating <-> resizing, operating -> finalizing -> exit`
#[quent_model::fsm(
    entry -> DynMemoryInitializing,
    DynMemoryInitializing -> DynMemoryOperating,
    DynMemoryOperating -> DynMemoryResizing,
    DynMemoryResizing -> DynMemoryOperating,
    DynMemoryOperating -> DynMemoryFinalizing,
    DynMemoryFinalizing -> exit,
)]
#[quent_model::resource(capacity = DynMemoryOperating)]
pub struct DynamicMemory;
