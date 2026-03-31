// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Task FSM for the simulator.
//!
//! A Task represents a unit of work executing on behalf of an operator.
//! It transitions through states like queueing, computing, allocating,
//! loading, spilling, and sending.

#[allow(unused_imports)]
use quent_model::prelude::*;

use quent_stdlib::{ChannelResource, MemoryResource, ProcessorResource};

// --- States ---

#[quent_model(state)]
pub struct Queueing {
    pub operator_id: Uuid,
    #[instance_name]
    pub instance_name: String,
}

#[quent_model(state)]
pub struct Computing {
    #[usage]
    pub use_thread: Usage<ProcessorResource>,
    #[usage]
    pub use_memory: Usage<MemoryResource>,
}

#[quent_model(state)]
pub struct Allocating {
    #[usage]
    pub use_thread: Usage<ProcessorResource>,
}

#[quent_model(state)]
pub struct Loading {
    #[usage]
    pub use_thread: Usage<ProcessorResource>,
    #[usage]
    pub use_fs_to_mem: Usage<ChannelResource>,
    #[usage]
    pub use_memory: Usage<MemoryResource>,
}

#[quent_model(state)]
pub struct Spilling {
    #[usage]
    pub use_thread: Usage<ProcessorResource>,
    #[usage]
    pub use_mem_to_fs: Usage<ChannelResource>,
}

#[quent_model(state)]
pub struct Sending {
    #[usage]
    pub use_thread: Usage<ProcessorResource>,
    #[usage]
    pub use_link: Usage<ChannelResource>,
}

// --- FSM ---

#[quent_model(fsm(
    entry -> Queueing,
    Queueing -> Allocating,
    Allocating -> Computing,
    Allocating -> Loading,
    Loading -> Computing,
    Computing -> Sending,
    Computing -> Spilling,
    Computing -> exit,
    Spilling -> Allocating,
    Sending -> Queueing,
))]
pub struct Task;
