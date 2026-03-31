// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Channel resource FSM definition.

#[allow(unused_imports)]
use quent_model::prelude::*;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct ChannelInitializing;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct ChannelOperating {
    #[capacity]
    pub capacity_bytes: Option<u64>,
    pub source_id: Uuid,
    pub target_id: Uuid,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct ChannelFinalizing;

/// A unidirectional data transfer resource.
///
/// FSM: `entry -> initializing -> operating -> finalizing -> exit`
///
/// The transition into `operating` declares the capacity (optional, `None`
/// if unbounded) and the source/target entity IDs.
#[derive(Fsm)]
#[resource(capacity = ChannelOperating)]
pub struct Channel {
    #[entry] #[to(ChannelOperating)]
    channel_initializing: ChannelInitializing,
    #[to(ChannelFinalizing)]
    channel_operating: ChannelOperating,
    #[to(exit)]
    channel_finalizing: ChannelFinalizing,
}
