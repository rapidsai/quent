// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Temporary analyzed types for generated schema entities.

// TODO(johanpel): Generate this module from schema metadata. See
// https://github.com/rapidsai/quent/issues/288.

use std::ops::Deref;

use quent_analyzer::{
    AnalyzerResult, Entity, RefTreeEntity,
    fsm::{
        Fsm, FsmUsages,
        native::{AnalyzedFsm, AnalyzedFsmBuilder, AnalyzedTransition},
    },
    resource::{CapacityDecl, Resource, ResourceTypeDecl, Usage, Using},
};
use quent_dynamic_attributes::DynamicAttribute;
use quent_query_engine_analyzer::{
    EngineEntity, OperatorEntity, OperatorEntityMut, PlanEntity, PortEntity, QueryEntity,
    WorkerEntity,
};
use quent_query_engine_ui as query_engine_ui;
use quent_simulator_store::{
    self as schema, gpu::GpuEvents, gpu_memory::GpuMemoryEvents, host_memory::HostMemoryEvents,
    network::NetworkEvents, network_channel::NetworkChannelEvents, pcie_channel::PcieChannelEvents,
    storage::StorageEvents, storage_channel::StorageChannelEvents,
    task_executor::TaskExecutorEvents, task_executor_thread::TaskExecutorThreadEvents,
};
use quent_store::entity::{
    EntityHandle,
    native::{NativeEntity, NativeEntityMarker},
};
use quent_time::{TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec, try_to_secs_relative};
use uuid::Uuid;

mod engine;
mod operator;
mod plan;
mod port;
mod query;
mod task;
mod worker;

pub use engine::Engine;
pub(crate) use engine::engine_custom_attributes;
pub use operator::Operator;
pub use plan::Plan;
pub use port::Port;
pub use query::Query;
pub(crate) use query::QueryBuilder;
pub(crate) use task::TaskBuilder;
pub use task::{Task, TaskExt};
pub use worker::Worker;

// TODO(johanpel): Replace these macros with store-build output once it can
// generate entity parent relationships and resource declarations.
macro_rules! entity {
    ($name:ident, $parent:ident) => {
        pub(crate) type $name = AnalyzerEntity<schema::$name>;

        impl RefTreeEntity for $name {
            fn parent_id(&self) -> Option<Uuid> {
                Some(self.declaration().$parent().target)
            }
        }
    };
}

macro_rules! resource {
    ($name:ident, $parent:ident, $capacity:expr) => {
        entity!($name, $parent);

        impl $name {
            pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
                ResourceTypeDecl::new(
                    <NativeEntity<schema::$name> as EntityHandle>::type_name(),
                    [$capacity],
                )
            }
        }

        impl Resource for $name {}
    };
}

entity!(Gpu, worker_id);
entity!(Network, engine_id);
entity!(TaskExecutor, worker_id);
resource!(GpuMemory, gpu_id, CapacityDecl::new_occupancy("bytes"));
resource!(HostMemory, worker_id, CapacityDecl::new_occupancy("bytes"));
resource!(Storage, worker_id, CapacityDecl::new_occupancy("bytes"));
resource!(NetworkChannel, network_id, CapacityDecl::new_rate("bytes"));
resource!(PcieChannel, gpu_id, CapacityDecl::new_rate("bytes"));
resource!(StorageChannel, worker_id, CapacityDecl::new_rate("bytes"));
resource!(TaskExecutorThread, task_executor_id, CapacityDecl::unit());

/// Adapts indexed entity events to the simulator analyzer interfaces.
// TODO(johanpel): Remove this adapter as analyzer traits move to quent-store
// and store-build can generate the remaining implementations. Orphan rules
// currently prevent implementing external traits for NativeEntity here.
#[derive(Debug)]
pub struct AnalyzerEntity<E: NativeEntityMarker>(NativeEntity<E>);

impl<E: NativeEntityMarker> AnalyzerEntity<E> {
    pub(crate) fn new(events: NativeEntity<E>) -> Self {
        Self(events)
    }
}

impl<E: NativeEntityMarker> Entity for AnalyzerEntity<E> {
    fn id(&self) -> Uuid {
        self.0.id().uuid()
    }

    fn type_name(&self) -> &str {
        <NativeEntity<E> as EntityHandle>::type_name()
    }

    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.earliest_timestamp()
    }

    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.latest_timestamp()
    }
}

impl<E: NativeEntityMarker> Deref for AnalyzerEntity<E> {
    type Target = NativeEntity<E>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
