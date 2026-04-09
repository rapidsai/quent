// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Query FSM: the top-level unit of work executed by an engine.

use quent_model::{Fsm, Ref, State};

// States

#[derive(Debug, State, serde::Serialize, serde::Deserialize)]
pub struct Init {
    #[parent_group]
    pub query_group_id: Ref<super::query_group::QueryGroup>,
    #[instance_name]
    pub instance_name: String,
}

#[derive(Debug, State, serde::Serialize, serde::Deserialize)]
pub struct Planning;

#[derive(Debug, State, serde::Serialize, serde::Deserialize)]
pub struct Executing;

// FSM

/// Query FSM: `entry -> Init -> Planning -> Executing -> exit`
#[derive(Fsm)]
#[resource_group]
pub struct Query {
    #[entry]
    #[to(Planning)]
    pub init: Init,
    #[to(Executing)]
    pub planning: Planning,
    #[to(exit)]
    pub executing: Executing,
}
