// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Channel resource definition.

#[allow(unused_imports)]
use quent_model::prelude::*;

/// A unidirectional data transfer resource.
///
/// FSM: `entry -> initializing -> operating -> finalizing -> exit`
///
/// The `source_id` and `target_id` identify the connected entities.
/// The `capacity_bytes` is optional (`None` if unbounded).
#[derive(Resource)]
pub struct Channel {
    pub source_id: Uuid,
    pub target_id: Uuid,
    #[capacity]
    pub capacity_bytes: Option<u64>,
}
