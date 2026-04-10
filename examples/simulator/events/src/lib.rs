// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_events::{resource::ResourceEvent, trace::TraceEvent};
use quent_query_engine_events::QueryEngineEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod task {

    use super::*;

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Queueing {
        pub operator_id: Uuid,
        pub instance_name: String,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Computing {
        pub use_thread: Uuid,
        pub use_memory: Uuid,
        pub use_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Allocating {
        pub use_thread: Uuid,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Loading {
        pub use_thread: Uuid,
        pub use_fs_to_mem: Uuid,
        pub use_fs_to_mem_bytes: u64,
        pub use_memory: Uuid,
        pub use_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Spilling {
        pub use_thread: Uuid,
        pub use_mem_to_fs: Uuid,
        pub use_mem_to_fs_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Sending {
        pub use_thread: Uuid,
        pub use_memory: Uuid,
        pub use_memory_bytes: u64,
        pub use_link: Uuid,
        pub use_link_bytes: u64,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub enum TaskEvent {
        Queueing(Queueing),
        Computing(Computing),
        Allocating(Allocating),
        Loading(Loading),
        Spilling(Spilling),
        Sending(Sending),
        Exit,
    }
}

pub mod record_batch {
    use super::*;

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Init {
        pub operator_id: Uuid,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Idle {
        pub use_filesystem: Option<Uuid>,
        pub use_filesystem_bytes: u64,
        pub use_main_memory: Option<Uuid>,
        pub use_main_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Moving {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum RecordBatchEvent {
        Initializing(Init),
        Idle(Idle),
        Moving(Moving),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SimulatorEvent {
    QueryEngineEvent(QueryEngineEvent),
    Task(task::TaskEvent),
    Resource(ResourceEvent),
    Trace(TraceEvent),
    // TODO(johanpel):
    // RecordBatch(record_batch::RecordBatchEvent),
}

impl From<ResourceEvent> for SimulatorEvent {
    fn from(event: ResourceEvent) -> Self {
        SimulatorEvent::Resource(event)
    }
}

impl From<TraceEvent> for SimulatorEvent {
    fn from(event: TraceEvent) -> Self {
        SimulatorEvent::Trace(event)
    }
}
