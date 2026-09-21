// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Temporary analyzed types for generated schema entities.

// TODO(johanpel): Generate this module from schema metadata. See
// https://github.com/rapidsai/quent/issues/288.

use quent_analyzer::{
    AnalyzerResult, Entity, RefTreeEntity,
    entity::native::{AnalyzedEntity, EntityEventAccumulator},
    fsm::{
        Fsm, FsmUsages,
        native::{AnalyzedFsm, AnalyzedFsmBuilder, AnalyzedTransition},
    },
    resource::{CapacityDecl, Resource, ResourceTypeDecl, Usage, Using},
};
use quent_events::Event;
use quent_query_engine_analyzer::{
    EngineEntity, OperatorEntity, OperatorEntityMut, PlanEntity, PortEntity, QueryEntity,
    QueryGroupEntity, WorkerEntity,
};
use quent_query_engine_ui as query_engine_ui;
use quent_simulator_store as schema;
use quent_time::{TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec, try_to_secs_relative};
use uuid::Uuid;

mod engine;
mod gpu;
mod gpu_memory;
mod host_memory;
mod network;
mod network_channel;
mod operator;
mod pcie_channel;
mod plan;
mod port;
mod query;
mod query_group;
mod storage;
mod storage_channel;
mod task;
mod task_executor;
mod task_executor_thread;
mod worker;

pub use engine::Engine;
pub(crate) use gpu::Gpu;
pub(crate) use gpu_memory::GpuMemory;
pub(crate) use host_memory::HostMemory;
pub(crate) use network::Network;
pub(crate) use network_channel::NetworkChannel;
pub use operator::Operator;
pub(crate) use pcie_channel::PcieChannel;
pub use plan::Plan;
pub use port::Port;
pub use query::Query;
pub(crate) use query::QueryBuilder;
pub use query_group::QueryGroup;
pub(crate) use storage::Storage;
pub(crate) use storage_channel::StorageChannel;
pub(crate) use task::TaskBuilder;
pub use task::{Task, TaskExt};
pub(crate) use task_executor::TaskExecutor;
pub(crate) use task_executor_thread::TaskExecutorThread;
pub use worker::Worker;
