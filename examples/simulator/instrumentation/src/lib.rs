// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use quent_exporter::ExporterOptions;
use quent_instrumentation::{
    Context, EventSender,
    resource::{
        ChannelResourceObserver, MemoryResourceObserver, ProcessorResourceObserver,
        ResourceGroupObserver,
    },
    trace::TraceObserver,
};
use quent_query_engine_events::{engine, operator, plan, port, query_group, worker};
use quent_simulator_events::SimulatorEvent;
use uuid::Uuid;

pub struct SimulatorContext {
    inner: Context<SimulatorEvent>,
}

impl SimulatorContext {
    pub fn try_new(exporter: Option<ExporterOptions>, id: Uuid) -> Result<Self, Box<dyn Error>> {
        Context::try_new(exporter, id).map(|inner| Self { inner })
    }

    /// Returns the event sender for creating FSM handles (e.g., TaskHandle).
    pub fn events_sender(&self) -> EventSender<SimulatorEvent> {
        self.inner.events_sender()
    }

    // Generated entity observers from the model definitions

    pub fn engine_observer(&self) -> engine::EngineObserver<SimulatorEvent> {
        engine::EngineObserver::new(&self.inner.events_sender())
    }

    pub fn worker_observer(&self) -> worker::WorkerObserver<SimulatorEvent> {
        worker::WorkerObserver::new(&self.inner.events_sender())
    }

    pub fn query_group_observer(&self) -> query_group::QueryGroupObserver<SimulatorEvent> {
        query_group::QueryGroupObserver::new(&self.inner.events_sender())
    }

    pub fn plan_observer(&self) -> plan::PlanObserver<SimulatorEvent> {
        plan::PlanObserver::new(&self.inner.events_sender())
    }

    pub fn operator_observer(&self) -> operator::OperatorObserver<SimulatorEvent> {
        operator::OperatorObserver::new(&self.inner.events_sender())
    }

    pub fn port_observer(&self) -> port::PortObserver<SimulatorEvent> {
        port::PortObserver::new(&self.inner.events_sender())
    }

    // Resource observers (from quent_instrumentation, not yet model-generated)

    pub fn memory_resource_observer(&self) -> MemoryResourceObserver<SimulatorEvent> {
        MemoryResourceObserver::new(self.inner.events_sender())
    }

    pub fn processor_resource_observer(&self) -> ProcessorResourceObserver<SimulatorEvent> {
        ProcessorResourceObserver::new(self.inner.events_sender())
    }

    pub fn channel_resource_observer(&self) -> ChannelResourceObserver<SimulatorEvent> {
        ChannelResourceObserver::new(self.inner.events_sender())
    }

    pub fn resource_group_observer(&self) -> ResourceGroupObserver<SimulatorEvent> {
        ResourceGroupObserver::new(self.inner.events_sender())
    }

    pub fn trace_observer(&self, entity_id: Uuid) -> TraceObserver<SimulatorEvent> {
        TraceObserver::new(self.inner.events_sender(), entity_id)
    }
}
