// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Memory resource definitions.

#[allow(unused_imports)]
use quent_model::prelude::*;

/// A fixed-bounds memory resource.
///
/// FSM: `entry -> initializing -> operating -> finalizing -> exit`
///
/// The transition into `operating` declares the capacity in bytes.
#[derive(Resource)]
pub struct Memory {
    pub capacity_bytes: Capacity<Option<u64>, Occupancy>,
}

/// A resizable memory resource.
///
/// FSM: `entry -> initializing -> operating <-> resizing -> finalizing -> exit`
#[derive(ResizableResource)]
pub struct ResizableMemory {
    pub capacity_bytes: Capacity<Option<u64>, Occupancy>,
}
