// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_events::{resource::ResourceEvent, trace::TraceEvent};
use quent_query_engine_model::QueryEngineEvent;
use serde::{Deserialize, Serialize};

// Re-export the model-generated task types.
pub use quent_simulator_model::task;

/// The model-generated task event type.
pub type TaskEvent = task::TaskEvent;

#[derive(Debug, Deserialize, Serialize)]
pub enum SimulatorEvent {
    QueryEngineEvent(QueryEngineEvent),
    Task(TaskEvent),
    Resource(ResourceEvent),
    Trace(TraceEvent),
}

impl From<QueryEngineEvent> for SimulatorEvent {
    fn from(event: QueryEngineEvent) -> Self {
        SimulatorEvent::QueryEngineEvent(event)
    }
}

// Transitive From impls: entity event → QueryEngineEvent → SimulatorEvent
macro_rules! impl_from_via_qe {
    ($($event_type:ty),* $(,)?) => {
        $(
            impl From<$event_type> for SimulatorEvent {
                fn from(event: $event_type) -> Self {
                    SimulatorEvent::QueryEngineEvent(QueryEngineEvent::from(event))
                }
            }
        )*
    };
}

use quent_query_engine_model::{engine, worker, query_group, query, plan, operator, port};

impl_from_via_qe!(
    engine::EngineEvent,
    worker::WorkerEvent,
    query_group::QueryGroupEvent,
    query::QueryEvent,
    plan::PlanEvent,
    operator::OperatorEvent,
    port::PortEvent,
);

impl From<TaskEvent> for SimulatorEvent {
    fn from(event: TaskEvent) -> Self {
        SimulatorEvent::Task(event)
    }
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
