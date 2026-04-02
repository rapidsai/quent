// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Query FSM: the top-level unit of work executed by an engine.

#[allow(unused_imports)]
use quent_model::prelude::*;

// --- States ---

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Init {
    #[parent_group]
    pub query_group_id: Uuid,
    #[instance_name]
    pub instance_name: String,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Planning;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Executing;

// --- FSM ---

/// Query FSM: `entry -> Init -> Planning -> Executing -> exit`
#[derive(Fsm)]
pub struct Query {
    #[entry] #[to(Planning)]
    pub init: Init,
    #[to(Executing)]
    pub planning: Planning,
    #[to(exit)]
    pub executing: Executing,
}
