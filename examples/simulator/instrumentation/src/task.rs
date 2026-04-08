// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Task FSM for the simulator.
//!
//! A Task represents a unit of work executing on behalf of an operator.
//! It transitions through states like queueing, computing, allocating,
//! loading, spilling, and sending.

use quent_model::{Fsm, State, Usage};
use uuid::Uuid;

use quent_stdlib::{ChannelResource, MemoryResource, ProcessorResource};

// States

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Queueing {
    pub operator_id: Uuid,
    #[instance_name]
    pub instance_name: String,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Computing {
    pub use_thread: Usage<ProcessorResource>,
    pub use_memory: Usage<MemoryResource>,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Allocating {
    pub use_thread: Usage<ProcessorResource>,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Loading {
    pub use_thread: Usage<ProcessorResource>,
    pub use_fs_to_mem: Usage<ChannelResource>,
    pub use_memory: Usage<MemoryResource>,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Spilling {
    pub use_thread: Usage<ProcessorResource>,
    pub use_mem_to_fs: Usage<ChannelResource>,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Sending {
    pub use_thread: Usage<ProcessorResource>,
    pub use_link: Usage<ChannelResource>,
}

// FSM

#[derive(Fsm)]
pub struct Task {
    #[entry]
    #[to(Allocating)]
    pub queueing: Queueing,
    #[to(Computing, Loading)]
    pub allocating: Allocating,
    #[to(Computing)]
    pub loading: Loading,
    #[to(Sending, Spilling, exit)]
    pub computing: Computing,
    #[to(Allocating)]
    pub spilling: Spilling,
    #[to(Queueing)]
    pub sending: Sending,
}
