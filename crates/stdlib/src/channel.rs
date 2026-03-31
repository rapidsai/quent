// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Channel resource FSM definition.

#[allow(unused_imports)]
use quent_model::prelude::*;

#[quent_model::state]
pub struct ChannelInitializing;

#[quent_model::state]
pub struct ChannelOperating {
    #[quent_model::capacity]
    pub capacity_bytes: Option<u64>,
    pub source_id: Uuid,
    pub target_id: Uuid,
}

#[quent_model::state]
pub struct ChannelFinalizing;

/// A unidirectional data transfer resource.
///
/// FSM: `entry -> initializing -> operating -> finalizing -> exit`
///
/// The transition into `operating` declares the capacity (optional, `None`
/// if unbounded) and the source/target entity IDs.
#[quent_model::fsm(
    resource(capacity = ChannelOperating),
    entry -> ChannelInitializing,
    ChannelInitializing -> ChannelOperating,
    ChannelOperating -> ChannelFinalizing,
    ChannelFinalizing -> exit,
)]
pub struct Channel;
