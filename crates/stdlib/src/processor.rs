// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Processor (unit resource) definition.

#[allow(unused_imports)]
use quent_model::prelude::*;

/// A unit resource representing a processor (e.g., a thread).
///
/// FSM: `entry -> initializing -> operating -> finalizing -> exit`
///
/// A unit resource has no capacity fields.
#[derive(Resource)]
pub struct Processor;
