// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Task FSM: running on a thread.

use quent_model::{Fsm, State, Usage};
use uuid::Uuid;

#[derive(Debug, State, serde::Serialize, serde::Deserialize)]
pub struct Queued {
    pub job_id: Uuid,
    #[instance_name]
    pub name: String,
}

#[derive(Debug, State, serde::Serialize, serde::Deserialize)]
pub struct Running {
    pub thread: Usage<quent_stdlib::ProcessorResource>,
}

#[derive(Fsm)]
pub struct Task {
    #[entry]
    #[to(Running)]
    pub queued: Queued,
    #[to(Queued, exit)]
    pub running: Running,
}
