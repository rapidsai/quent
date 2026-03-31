// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Query FSM: the top-level unit of work executed by an engine.

#[allow(unused_imports)]
use quent_model::prelude::*;

// --- States ---

#[quent_model::state]
pub struct Init {
    pub query_group_id: Uuid,
    #[quent_model::instance_name]
    pub instance_name: String,
}

#[quent_model::state]
pub struct Planning;

#[quent_model::state]
pub struct Executing;

// --- FSM ---

/// Query FSM: `entry -> Init -> Planning -> Executing -> exit`
#[quent_model::fsm(
    entry -> Init,
    Init -> Planning,
    Planning -> Executing,
    Executing -> exit,
)]
pub struct Query;
