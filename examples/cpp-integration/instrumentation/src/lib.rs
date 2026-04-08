// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Instrumentation for the C++ integration example.
//!
//! A job scheduler that runs tasks on a thread pool:
//! - Job: entity that groups tasks (root resource group)
//! - Task: FSM (queued → running → exit) using thread resources
//! - ThreadPool: resource group containing Thread resources
//! - Thread: processor resource (from stdlib)

#[allow(unused_imports)]
use quent_model::prelude::*;

// Job: groups tasks, root resource group

#[derive(Debug, Event, serde::Serialize, serde::Deserialize)]
pub struct Submit {
    pub name: String,
    pub num_tasks: u32,
}

#[derive(Debug, Event, serde::Serialize, serde::Deserialize)]
pub struct Complete;

#[derive(Entity)]
#[resource_group(root)]
pub struct Job {
    pub submit: EmitOnce<Submit>,
    pub complete: EmitOnce<Complete>,
}

// ThreadPool: resource group containing threads

#[derive(Debug, Event, serde::Serialize, serde::Deserialize)]
pub struct ThreadPoolInit {
    pub num_threads: u32,
}

#[derive(Entity)]
#[resource_group]
pub struct ThreadPool {
    pub init: EmitOnce<ThreadPoolInit>,
}

// Task: FSM running on a thread

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Queued {
    pub job_id: Uuid,
    #[instance_name]
    pub name: String,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
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

// Model + Context

quent_model::define_model! {
    Example {
        root: Job,
        ThreadPool,
        Task,
    }
}

quent_model::define_instrumentation!(Example);
