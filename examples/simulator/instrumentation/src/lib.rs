// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use quent_events::Event;
use quent_exporter::ExporterOptions;
use quent_instrumentation::{
    Context, EventSender,
    resource::{
        ChannelResourceObserver, MemoryResourceObserver, ProcessorResourceObserver,
        ResourceGroupObserver,
    },
    trace::TraceObserver,
};
use quent_query_engine_events::{
    QueryEngineEvent, engine, operator, plan,
    port::{self, PortEvent},
    query, query_group, worker,
};
use quent_simulator_events::SimulatorEvent;
use uuid::Uuid;

pub struct SimulatorContext {
    inner: Context<SimulatorEvent>,
}

impl SimulatorContext {
    pub fn try_new(exporter: Option<ExporterOptions>, id: Uuid) -> Result<Self, Box<dyn Error>> {
        Context::try_new(exporter, id).map(|inner| Self { inner })
    }

    // This is a lot of repetition but some FFIs don't allow generics so either
    // we need to do macros or just keep it spelled out like this.
    // Or move this burden to the FFI layer itself.

    pub fn engine_observer(&self) -> EngineObserver {
        EngineObserver {
            tx: self.inner.events_sender(),
        }
    }
    pub fn query_group_observer(&self) -> QueryGroupObserver {
        QueryGroupObserver {
            tx: self.inner.events_sender(),
        }
    }
    pub fn worker_observer(&self) -> WorkerObserver {
        WorkerObserver {
            tx: self.inner.events_sender(),
        }
    }
    pub fn query_observer(&self) -> QueryObserver {
        QueryObserver {
            tx: self.inner.events_sender(),
        }
    }
    pub fn plan_observer(&self) -> PlanObserver {
        PlanObserver {
            tx: self.inner.events_sender(),
        }
    }
    pub fn operator_observer(&self) -> OperatorObserver {
        OperatorObserver {
            tx: self.inner.events_sender(),
        }
    }

    pub fn port_observer(&self) -> PortObserver {
        PortObserver {
            tx: self.inner.events_sender(),
        }
    }

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

    /// Returns the event sender for creating FSM handles (e.g., TaskHandle).
    pub fn events_sender(&self) -> EventSender<SimulatorEvent> {
        self.inner.events_sender()
    }

    pub fn trace_observer(&self, entity_id: Uuid) -> TraceObserver<SimulatorEvent> {
        TraceObserver::new(self.inner.events_sender(), entity_id)
    }
}

fn push_event(tx: &EventSender<SimulatorEvent>, event: Event<SimulatorEvent>) {
    tx.send(event)
}

#[derive(Clone)]
pub struct EngineObserver {
    tx: EventSender<SimulatorEvent>,
}

impl EngineObserver {
    pub fn init(&self, id: Uuid, init: engine::Init) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::Engine(
                    engine::EngineEvent::Init(init),
                )),
            ),
        )
    }

    pub fn exit(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::Engine(
                    engine::EngineEvent::Exit(engine::Exit),
                )),
            ),
        )
    }
}

#[derive(Clone)]
pub struct QueryGroupObserver {
    tx: EventSender<SimulatorEvent>,
}

impl QueryGroupObserver {
    pub fn group(&self, id: Uuid, event: query_group::QueryGroupEvent) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::QueryGroup(event)),
            ),
        )
    }
}

#[derive(Clone)]
pub struct WorkerObserver {
    tx: EventSender<SimulatorEvent>,
}

impl WorkerObserver {
    pub fn init(&self, id: Uuid, init: worker::Init) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::Worker(
                    worker::WorkerEvent::Init(init),
                )),
            ),
        )
    }

    pub fn exit(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::Worker(
                    worker::WorkerEvent::Exit(worker::Exit),
                )),
            ),
        )
    }
}

#[derive(Clone)]
pub struct QueryObserver {
    tx: EventSender<SimulatorEvent>,
}

impl QueryObserver {
    pub fn init(&self, id: Uuid, init: query::Init) {
        self.emit_query_transition(id, query::QueryTransition::Init(init));
    }

    pub fn planning(&self, id: Uuid) {
        self.emit_query_transition(id, query::QueryTransition::Planning(query::Planning));
    }

    pub fn executing(&self, id: Uuid) {
        self.emit_query_transition(id, query::QueryTransition::Executing(query::Executing));
    }

    pub fn exit(&self, id: Uuid) {
        self.emit_query_transition(id, query::QueryTransition::Exit);
    }

    fn emit_query_transition(&self, id: Uuid, state: query::QueryTransition) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::Query(
                    quent_model::FsmEvent::Transition { seq: 0, state },
                )),
            ),
        )
    }
}

#[derive(Clone)]
pub struct PlanObserver {
    tx: EventSender<SimulatorEvent>,
}

impl PlanObserver {
    pub fn plan(&self, id: Uuid, event: plan::PlanEvent) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::Plan(event)),
            ),
        )
    }
}

#[derive(Clone)]
pub struct OperatorObserver {
    tx: EventSender<SimulatorEvent>,
}

impl OperatorObserver {
    pub fn operator(&self, id: Uuid, declaration: operator::Declaration) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::Operator(
                    operator::OperatorEvent::Declaration(declaration),
                )),
            ),
        )
    }

    pub fn statistics(&self, id: Uuid, statistics: operator::Statistics) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::Operator(
                    operator::OperatorEvent::Statistics(statistics),
                )),
            ),
        )
    }
}

#[derive(Clone)]
pub struct PortObserver {
    tx: EventSender<SimulatorEvent>,
}

impl PortObserver {
    pub fn port(&self, id: Uuid, event: port::Declaration) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::Port(PortEvent::Declaration(
                    event,
                ))),
            ),
        )
    }

    pub fn statistics(&self, id: Uuid, event: port::Statistics) {
        push_event(
            &self.tx,
            Event::new_now(
                id,
                SimulatorEvent::QueryEngineEvent(QueryEngineEvent::Port(PortEvent::Statistics(
                    event,
                ))),
            ),
        )
    }
}

// TaskObserver removed — replaced by quent_simulator_model::task::TaskHandle<SimulatorEvent>
