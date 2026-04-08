// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Instrumentation for the C++ integration example.
//!
//! A job scheduler that runs tasks on a thread pool:
//! - Job: entity that groups tasks (root resource group)
//! - Task: FSM (queued -> running -> exit) using thread resources
//! - ThreadPool: resource group containing Thread resources
//! - Thread: processor resource (from stdlib)

pub mod job;
pub mod task;
pub mod thread_pool;

quent_model::define_model! {
    Example {
        root: job::Job,
        thread_pool::ThreadPool,
        task::Task,
    }
}

quent_model::define_instrumentation!(Example);
