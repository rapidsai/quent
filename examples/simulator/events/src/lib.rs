// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_events::{resource::ResourceEvent, trace::TraceEvent};
use quent_query_engine_events::QueryEngineEvent;
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
