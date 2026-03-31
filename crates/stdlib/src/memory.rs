// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Memory resource FSM definitions.

#[allow(unused_imports)]
use quent_model::prelude::*;

// --- Fixed-bounds Memory ---

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct MemoryInitializing;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct MemoryOperating {
    #[capacity]
    pub capacity_bytes: u64,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct MemoryFinalizing;

/// A fixed-bounds memory resource FSM handle.
///
/// FSM: `entry -> initializing -> operating -> finalizing -> exit`
///
/// The transition into `operating` declares the capacity in bytes.
/// Use `MemoryResource` with `Usage<MemoryResource>` to reference this
/// resource type from FSM states.
#[derive(Fsm)]
#[resource(capacity = MemoryOperating)]
pub struct Memory {
    #[entry] #[to(MemoryOperating)]
    memory_initializing: MemoryInitializing,
    #[to(MemoryFinalizing)]
    memory_operating: MemoryOperating,
    #[to(exit)]
    memory_finalizing: MemoryFinalizing,
}

// --- Dynamic-bounds Memory ---

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct DynMemoryInitializing;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct DynMemoryOperating {
    #[capacity]
    pub capacity_bytes: u64,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct DynMemoryResizing;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct DynMemoryFinalizing;

/// A dynamic-bounds memory resource that supports resizing.
///
/// FSM: `entry -> initializing -> operating <-> resizing, operating -> finalizing -> exit`
#[derive(Fsm)]
#[resource(capacity = DynMemoryOperating)]
pub struct DynamicMemory {
    #[entry] #[to(DynMemoryOperating)]
    dyn_memory_initializing: DynMemoryInitializing,
    #[to(DynMemoryResizing, DynMemoryFinalizing)]
    dyn_memory_operating: DynMemoryOperating,
    #[to(DynMemoryOperating)]
    dyn_memory_resizing: DynMemoryResizing,
    #[to(exit)]
    dyn_memory_finalizing: DynMemoryFinalizing,
}
