//! Analyzes raw events to produce useful performance insights

use std::collections::HashMap;

use quent_entities::{
    coordinator::Coordinator,
    engine::Engine,
    operator::{Operator, OperatorState, Port, WaitingForInputs},
    plan::{self, Plan},
    query::Query,
    worker::Worker,
};
use quent_events::{
    Event as RawEvent, EventData, coordinator::CoordinatorEvent, engine::EngineEvent,
    operator::OperatorEvent, query::QueryEvent, worker::WorkerEvent,
};
use tracing::trace;
use uuid::Uuid;

pub mod error;

pub type Result<T> = std::result::Result<T, error::Error>;

pub type Event = RawEvent<EventData>;

// TODO(johanpel): make it fast
pub struct Analyzer {
    engine: Engine,
    workers: HashMap<Uuid, Worker>,
    coordinators: HashMap<Uuid, Coordinator>,
    queries: HashMap<Uuid, Query>,
}

// Just a slightly shorter way to get entry or insert default
fn entry<T>(map: &mut HashMap<Uuid, T>, id: Uuid) -> &mut T
where
    T: Default,
{
    map.entry(id).or_default()
}

impl Analyzer {
    pub fn try_new(engine_id: Uuid, mut events: impl Iterator<Item = Event>) -> Result<Self> {
        // TODO(johanpel): we need to sit down and think about how to do this as quickly as
        //                 possible for larger datasets, this is just a trivial implementation
        //                 to make it work. This is known to get pretty intense.
        let mut engine = Engine::new(engine_id);
        let mut coordinators: HashMap<Uuid, Coordinator> = HashMap::new();
        let mut workers: HashMap<Uuid, Worker> = HashMap::new();
        let mut queries: HashMap<Uuid, Query> = HashMap::new();
        let mut plans: HashMap<Uuid, Plan> = HashMap::new();
        let mut operators: HashMap<Uuid, Operator> = HashMap::new();

        events.try_for_each(|event| {
            let event: Event = event;
            let ts = event.timestamp;

            match event.data {
                // TODO(johanpel): validation logic
                EventData::Engine(engine_event) => match engine_event {
                    EngineEvent::Init(init) => {
                        engine.name = init.name;
                        engine.implementation = init.implementation;
                        engine.timestamps.init = Some(ts);
                    }
                    EngineEvent::Operating(_) => engine.timestamps.operating = Some(ts),
                    EngineEvent::Finalizing(_) => engine.timestamps.finalizing = Some(ts),
                    EngineEvent::Exit(_) => engine.timestamps.exit = Some(ts),
                },
                EventData::Coordinator(coordinator_event) => {
                    let entry = entry(&mut coordinators, event.id);
                    match coordinator_event {
                        CoordinatorEvent::Init(init) => {
                            entry.id = event.id;
                            entry.name = init.name;
                            entry.engine_id = init.engine_id;
                            entry.timestamps.init = Some(ts);
                        }
                        CoordinatorEvent::Operating(_) => entry.timestamps.operating = Some(ts),
                        CoordinatorEvent::Finalizing(_) => entry.timestamps.finalizing = Some(ts),
                        CoordinatorEvent::Exit(_) => entry.timestamps.exit = Some(ts),
                    }
                }
                EventData::Worker(worker_event) => {
                    let entry = entry(&mut workers, event.id);
                    match worker_event {
                        WorkerEvent::Init(init) => {
                            entry.id = event.id;
                            entry.name = init.name;
                            entry.engine_id = init.engine_id;
                            entry.timestamps.init = Some(ts);
                        }
                        WorkerEvent::Operating(_) => entry.timestamps.operating = Some(ts),
                        WorkerEvent::Finalizing(_) => entry.timestamps.finalizing = Some(ts),
                        WorkerEvent::Exit(_) => entry.timestamps.exit = Some(ts),
                    }
                }
                EventData::Query(query_event) => {
                    let entry = entry(&mut queries, event.id);
                    match query_event {
                        QueryEvent::Init(init) => {
                            entry.id = event.id;
                            entry.coordinator_id = init.coordinator_id;
                            entry.timestamps.init = Some(ts);
                        }
                        QueryEvent::Planning(_) => entry.timestamps.planning = Some(ts),
                        QueryEvent::Executing(_) => entry.timestamps.executing = Some(ts),
                        QueryEvent::Idle(_) => entry.timestamps.idle = Some(ts),
                        QueryEvent::Finalizing(_) => entry.timestamps.finalizing = Some(ts),
                        QueryEvent::Exit(_) => entry.timestamps.exit = Some(ts),
                    }
                }
                EventData::Plan(plan_event) => {
                    let entry = entry(&mut plans, event.id);
                    match plan_event {
                        quent_events::plan::PlanEvent::Init(init) => {
                            // TODO(johanpel): validate edges have ids of existing operator ports.
                            entry.id = event.id;
                            entry.query_id = init.query_id;
                            entry.edges = init
                                .edges
                                .into_iter()
                                .map(|(source, target)| plan::Edge { source, target })
                                .collect();
                            entry.timestamps.init = Some(event.timestamp);
                        }
                        quent_events::plan::PlanEvent::Executing(_) => {
                            entry.timestamps.executing = Some(event.timestamp)
                        }
                        quent_events::plan::PlanEvent::Idle(_) => {
                            entry.timestamps.idle = Some(event.timestamp)
                        }
                        quent_events::plan::PlanEvent::Finalizing(_) => {
                            entry.timestamps.finalizing = Some(event.timestamp)
                        }
                        quent_events::plan::PlanEvent::Exit(_) => {
                            entry.timestamps.exit = Some(event.timestamp)
                        }
                    }
                }
                EventData::Operator(operator_event) => {
                    let entry = entry(&mut operators, event.id);
                    // TODO(johanpel): sequence numbers
                    match operator_event {
                        OperatorEvent::Init(init) => {
                            entry.id = event.id;
                            entry.name = init.name;
                            entry.plan_id = init.plan_id;
                            entry.parent_plan_ids = init.parent_plan_ids;
                            entry
                                .state_sequence
                                .push(OperatorState::Init(event.timestamp));
                            entry.ports = init
                                .ports
                                .into_iter()
                                .map(|ep| Port {
                                    id: ep.id,
                                    is_input: ep.is_input,
                                    name: ep.name,
                                })
                                .collect();
                        }
                        OperatorEvent::WaitingForInputs(_) => {
                            entry.state_sequence.push(OperatorState::WaitingForInputs(
                                WaitingForInputs {
                                    timestamp: event.timestamp,
                                    ports: vec![], // TODO(johanpel)
                                },
                            ));
                        }
                        OperatorEvent::Executing(_) => entry
                            .state_sequence
                            .push(OperatorState::Executing(event.timestamp)),
                        OperatorEvent::Blocked(_) => entry
                            .state_sequence
                            .push(OperatorState::Blocked(event.timestamp)),
                        OperatorEvent::Finalizing(_) => entry
                            .state_sequence
                            .push(OperatorState::Finalizing(event.timestamp)),
                        OperatorEvent::Exit(_) => entry
                            .state_sequence
                            .push(OperatorState::Exit(event.timestamp)),
                    }
                }
            }
            Ok(())
        })?;

        // All events are transformed into entities. Filter out parentless entities.
        for key in coordinators.keys().cloned().collect::<Vec<_>>() {
            if coordinators.get(&key).unwrap().engine_id != engine_id {
                coordinators.remove(&key);
            }
        }
        for key in workers.keys().cloned().collect::<Vec<_>>() {
            if workers.get(&key).unwrap().engine_id != engine_id {
                workers.remove(&key);
            }
        }
        for key in queries.keys().cloned().collect::<Vec<_>>() {
            if !coordinators.contains_key(&queries.get(&key).unwrap().coordinator_id) {
                queries.remove(&key);
            }
        }
        for key in operators.keys().cloned().collect::<Vec<_>>() {
            let op = operators.get_mut(&key).unwrap();
            if let Some(plan) = plans.get_mut(&op.plan_id) {
                trace!("plan {} -> operator {}", plan.id, op.id);
                op.state_sequence.sort_by_key(|a| a.timestamp());
                plan.operators.push(op.clone());
            } else {
                operators.remove(&key);
            }
        }
        for key in plans.keys().cloned().collect::<Vec<_>>() {
            let plan = plans.get_mut(&key).unwrap();
            if let Some(query) = queries.get_mut(&plan.query_id) {
                trace!("query {} <- plan {}", query.id, plan.id);
                query.plans.push(plan.clone());
            } else {
                plans.remove(&key);
            }
        }

        Ok(Self {
            engine,
            coordinators,
            workers,
            queries,
        })
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    // TODO(johanpel): this is separated from an engine, since we assume engines can have
    //                 immense lifetimes so they could be running lots of coordinators, in
    //                 which case we may want to implement pagination for this.
    pub fn worker_ids(&self) -> Vec<Uuid> {
        self.workers.keys().cloned().collect()
    }
    pub fn worker(&self, id: Uuid) -> Option<&Worker> {
        self.workers.get(&id)
    }
    // TODO(johanpel): pagination
    pub fn coordinator_ids(&self) -> Vec<Uuid> {
        self.coordinators.keys().cloned().collect()
    }
    pub fn coordinator(&self, id: Uuid) -> Option<&Coordinator> {
        self.coordinators.get(&id)
    }
    // TODO(johanpel): pagination
    pub fn query_ids(&self, coordinator_id: Uuid) -> Vec<Uuid> {
        self.queries
            .iter()
            .filter_map(|(k, v)| (v.coordinator_id == coordinator_id).then_some(*k))
            .collect()
    }

    pub fn query(&self, id: Uuid) -> Option<&Query> {
        self.queries.get(&id)
    }
}
