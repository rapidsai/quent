use std::collections::{HashMap, HashSet};

use quent_entities::{
    EntityRef,
    engine::Engine,
    operator::{Operator, OperatorState, Port, WaitingForInputs},
    plan::Plan,
    query::Query,
    query_group::QueryGroup,
    resource::{Resource, ResourceGroup, ResourceOperatingState, ResourceState},
    worker::Worker,
};
use quent_events::{
    EventData,
    attributes::{Attribute, Value},
    engine::EngineEvent,
    operator::OperatorEvent,
    query::QueryEvent,
    query_group::QueryGroupEvent,
    resource::{channel, group as resource_group, memory, processor},
    worker::WorkerEvent,
};

use py_rs::PY;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{Event, Result, error::Error};

// Get an entity from a map or insert a default entity.
//
// We require the entity type to implement Default, because events could arrive
// into the anlayzer unordered. When they have a Default, it won't matter which
// state transition comes first in order to start populating them.
//
// TODO(johanpel): this is incredibly annoying however, because when adding
// fields, you have to not forget to populate them properly once the associated
// event arrives down below. Figure out a compile-time way to force you not to
// forget this.
fn entry<T>(map: &mut HashMap<Uuid, T>, id: Uuid) -> &mut T
where
    T: Default,
{
    map.entry(id).or_default()
}

/// A set of entities stored in maps from entity id -> entity.
///
/// The maps provide an easy way for quick lookups based on entity IDs.
/// This also means the values in the maps are flattened entities.
/// Their relations are solely expressed through IDs serving as references.
#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct Entities {
    pub engine: Engine,
    pub query_groups: HashMap<Uuid, QueryGroup>,
    pub workers: HashMap<Uuid, Worker>,
    pub queries: HashMap<Uuid, Query>,
    pub plans: HashMap<Uuid, Plan>,
    pub operators: HashMap<Uuid, Operator>,
    pub ports: HashMap<Uuid, Port>,
    pub resource_groups: HashMap<Uuid, ResourceGroup>,
    pub resources: HashMap<Uuid, Resource>,
}

impl Entities {
    pub fn try_new(engine_id: Uuid, events: impl Iterator<Item = Event>) -> Result<Self> {
        // TODO(johanpel): we need to sit down and think about how to do this as quickly as
        //                 possible for larger datasets, this is just a trivial implementation
        //                 to make it work. This is known to get pretty intense.

        let mut engine = Engine::new(engine_id);
        let mut query_groups: HashMap<Uuid, QueryGroup> = HashMap::new();
        let mut workers: HashMap<Uuid, Worker> = HashMap::new();
        let mut queries: HashMap<Uuid, Query> = HashMap::new();
        let mut plans: HashMap<Uuid, Plan> = HashMap::new();
        let mut operators: HashMap<Uuid, Operator> = HashMap::new();
        let mut ports: HashMap<Uuid, Port> = HashMap::new();
        let mut resource_groups: HashMap<Uuid, ResourceGroup> = HashMap::new();
        let mut resources: HashMap<Uuid, Resource> = HashMap::new();

        for event in events {
            let ts = event.timestamp;

            match event.data {
                EventData::Engine(engine_event) => {
                    let engine = &mut engine;
                    match engine_event {
                        EngineEvent::Init(init) => {
                            engine.name = init.name;
                            engine.implementation = init.implementation;
                            engine.timestamps.init = Some(ts);
                        }
                        EngineEvent::Operating(_) => engine.timestamps.operating = Some(ts),
                        EngineEvent::Finalizing(_) => engine.timestamps.finalizing = Some(ts),
                        EngineEvent::Exit(_) => engine.timestamps.exit = Some(ts),
                    }
                }
                EventData::QueryGroup(query_group_event) => {
                    let entry = entry(&mut query_groups, event.id);
                    match query_group_event {
                        QueryGroupEvent::Init(init) => {
                            entry.id = event.id;
                            entry.name = init.name;
                            entry.engine_id = init.engine_id;
                            entry.timestamps.init = Some(ts);
                        }
                        QueryGroupEvent::Operating(_) => entry.timestamps.operating = Some(ts),
                        QueryGroupEvent::Finalizing(_) => entry.timestamps.finalizing = Some(ts),
                        QueryGroupEvent::Exit(_) => entry.timestamps.exit = Some(ts),
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
                            entry.query_group_id = init.query_group_id;
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
                            entry.name = init.name;
                            entry.query_id = init.query_id;
                            entry.parent_plan_id = init.parent_plan_id;
                            entry.worker_id = init.worker_id;
                            entry.edges = init.edges;
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
                            entry.parent_plan_id = init.plan_id;
                            entry.parent_operator_ids = init.parent_operator_ids;
                            entry
                                .state_sequence
                                .push(OperatorState::Init(event.timestamp));
                            // Port declarations travel with this event, but for
                            // consistency with other entities towards the UI
                            // we're flattening them into their own map and
                            // don't nest them under the Operator entity.
                            entry.ports = init.ports.iter().map(|p| p.id).collect();
                            ports.extend(init.ports.into_iter().map(|p| {
                                (
                                    p.id,
                                    Port {
                                        id: p.id,
                                        parent_operator_id: event.id,
                                        name: p.name,
                                    },
                                )
                            }));
                        }
                        OperatorEvent::WaitingForInputs(waiting) => {
                            entry.state_sequence.push(OperatorState::WaitingForInputs(
                                WaitingForInputs {
                                    timestamp: event.timestamp,
                                    ports: waiting.ports,
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
                EventData::Resource(resource_event) => {
                    let entry = entry(&mut resources, event.id);
                    match resource_event {
                        quent_events::resource::ResourceEvent::Memory(memory_event) => {
                            match memory_event {
                                memory::MemoryEvent::Init(init) => {
                                    entry.id = event.id;
                                    entry
                                        .state_sequence
                                        .push(ResourceState::Init(event.timestamp));
                                    entry.capacities = vec![Attribute {
                                        key: "bytes".to_string(),
                                        // TODO(johanpel): have a better
                                        // mechanism to convey this is bounded
                                        value: Some(Value::U64(0)),
                                    }];
                                    entry.name = Some(init.resource.name);
                                    entry.scope = Some(init.resource.scope.into());
                                }
                                memory::MemoryEvent::Operating(operating) => entry
                                    .state_sequence
                                    .push(ResourceState::Operating(ResourceOperatingState {
                                        timestamp: event.timestamp,
                                        capacities: vec![Attribute {
                                            key: "capacity_bytes".to_string(),
                                            value: Some(Value::U64(operating.capacity_bytes)),
                                        }],
                                    })),
                                memory::MemoryEvent::Resizing(_) => entry
                                    .state_sequence
                                    .push(ResourceState::Resizing(event.timestamp)),
                                memory::MemoryEvent::Finalizing(_) => entry
                                    .state_sequence
                                    .push(ResourceState::Finalizing(event.timestamp)),
                                memory::MemoryEvent::Exit(_) => entry
                                    .state_sequence
                                    .push(ResourceState::Exit(event.timestamp)),
                            }
                        }
                        quent_events::resource::ResourceEvent::Processor(
                            processor_resource_event,
                        ) => match processor_resource_event {
                            processor::ProcessorEvent::Init(init) => {
                                entry.id = event.id;
                                entry
                                    .state_sequence
                                    .push(ResourceState::Init(event.timestamp));
                                entry.name = Some(init.resource.name);
                                entry.scope = Some(init.resource.scope.into());
                            }
                            processor::ProcessorEvent::Operating(_) => entry.state_sequence.push(
                                ResourceState::Operating(ResourceOperatingState {
                                    timestamp: event.timestamp,
                                    capacities: vec![],
                                }),
                            ),
                            processor::ProcessorEvent::Finalizing(_) => entry
                                .state_sequence
                                .push(ResourceState::Finalizing(event.timestamp)),
                            processor::ProcessorEvent::Exit(_) => entry
                                .state_sequence
                                .push(ResourceState::Exit(event.timestamp)),
                        },
                        quent_events::resource::ResourceEvent::Channel(channel_resource_event) => {
                            match channel_resource_event {
                                channel::ChannelEvent::Init(init) => {
                                    entry.id = event.id;
                                    entry
                                        .state_sequence
                                        .push(ResourceState::Init(event.timestamp));
                                    // TODO(johanpel): figure out how to convey
                                    // to the UI that it has to render this as a
                                    // throughput thing.
                                    entry.capacities = vec![Attribute {
                                        key: "bytes".to_string(),
                                        value: None,
                                    }];
                                    entry.name = Some(init.resource.name);
                                    entry.scope = Some(init.resource.scope.into());
                                }
                                channel::ChannelEvent::Operating(_) => entry.state_sequence.push(
                                    ResourceState::Operating(ResourceOperatingState {
                                        timestamp: event.timestamp,
                                        capacities: vec![],
                                    }),
                                ),
                                channel::ChannelEvent::Finalizing(_) => entry
                                    .state_sequence
                                    .push(ResourceState::Finalizing(event.timestamp)),
                                channel::ChannelEvent::Exit(_) => entry
                                    .state_sequence
                                    .push(ResourceState::Exit(event.timestamp)),
                            }
                        }
                    }
                }
                EventData::ResourceGroup(resource_group_event) => {
                    let entry = entry(&mut resource_groups, event.id);
                    match resource_group_event {
                        resource_group::ResourceGroupEvent::Init(init) => {
                            entry.id = event.id;
                            entry.name = init.resource.name;
                            entry.scope = Some(init.resource.scope.into());
                            entry.timestamps.init = Some(event.timestamp);
                        }
                        resource_group::ResourceGroupEvent::Operating(_) => {
                            entry.timestamps.operating = Some(event.timestamp)
                        }
                        resource_group::ResourceGroupEvent::Finalizing(_) => {
                            entry.timestamps.finalizing = Some(event.timestamp)
                        }
                        resource_group::ResourceGroupEvent::Exit(_) => {
                            entry.timestamps.exit = Some(event.timestamp)
                        }
                    }
                }
            }
        }

        Ok(Self {
            engine,
            query_groups,
            workers,
            queries,
            plans,
            operators,
            ports,
            resource_groups,
            resources,
        })
    }

    /// Filter out parentless entities.
    // TODO(johanpel): collect the parentless entries and return them from this
    // function so they can be reported or otherwise processed.
    pub fn remove_parentless(&mut self) {
        // Filter query groups that don't refer to this engine.
        for key in self.query_groups.keys().cloned().collect::<Vec<_>>() {
            if self.query_groups.get(&key).unwrap().engine_id != self.engine.id {
                self.query_groups.remove(&key);
            }
        }
        // Filter workers that don't refer to this engine.
        for key in self.workers.keys().cloned().collect::<Vec<_>>() {
            if self.workers.get(&key).unwrap().engine_id != self.engine.id {
                self.workers.remove(&key);
            }
        }
        // Filter queries that don't refer to any query groups.
        for key in self.queries.keys().cloned().collect::<Vec<_>>() {
            if !self
                .query_groups
                .contains_key(&self.queries.get(&key).unwrap().query_group_id)
            {
                self.queries.remove(&key);
            }
        }
        // Filter plans that don't refer to any queries.
        for key in self.plans.keys().cloned().collect::<Vec<_>>() {
            if !self
                .queries
                .contains_key(&self.plans.get(&key).unwrap().query_id)
            {
                self.queries.remove(&key);
            }
        }
        // Filter operators that don't refer to any plans
        for key in self.operators.keys().cloned().collect::<Vec<_>>() {
            if !self
                .plans
                .contains_key(&self.operators.get(&key).unwrap().parent_plan_id)
            {
                self.operators.remove(&key);
            }
        }

        // Gather all IDs of required entities into a set.
        let mut parent_entity_ids: HashSet<Uuid> = HashSet::with_capacity(
            self.query_groups.len()
                + self.workers.len()
                + self.queries.len()
                + self.plans.len()
                + self.operators.len(),
        );
        parent_entity_ids.extend(self.query_groups.keys());
        parent_entity_ids.extend(self.workers.keys());
        parent_entity_ids.extend(self.queries.keys());
        parent_entity_ids.extend(self.plans.keys());
        parent_entity_ids.extend(self.operators.keys());

        // Filter resource groups that don't refer to any of the above entities.
        for key in self.resource_groups.keys().cloned().collect::<Vec<_>>() {
            if let Some(scope) = &self.resource_groups.get(&key).unwrap().scope {
                if !parent_entity_ids.contains(&(*scope).into()) {
                    self.operators.remove(&key);
                }
            } else {
                self.operators.remove(&key);
            }
        }

        parent_entity_ids.extend(self.resource_groups.keys());

        // Filter resources that don't refer to any of the above entities.
        for key in self.resources.keys().cloned().collect::<Vec<_>>() {
            if let Some(scope) = &self.resources.get(&key).unwrap().scope
                && parent_entity_ids.contains(&(*scope).into())
            {
                continue;
            }
            self.resources.remove(&key);
        }
    }

    /// Clone all entities that are involved in executing one single query.
    pub fn try_filter_by_query(&self, query_id: Uuid) -> Result<Self> {
        let engine = self.engine.clone();

        let query = self
            .queries
            .get(&query_id)
            .ok_or(Error::InvalidId(query_id))?;
        let queries = HashMap::from([(query_id, query.clone())]);

        let query_group = self
            .query_groups
            .get(&query.query_group_id)
            .ok_or(Error::Logic(format!(
                "unable to filter by query - query group {} of query {} does not exist",
                query.query_group_id, query.id
            )))?;
        let query_groups = HashMap::from([(query_group.id, query_group.clone())]);

        let plans: HashMap<Uuid, Plan> = self
            .plans
            .iter()
            .filter_map(|(k, v)| (v.query_id == query_id).then_some((*k, v.clone())))
            .collect();
        let operators: HashMap<Uuid, Operator> = self
            .operators
            .iter()
            .filter_map(|(k, v)| {
                plans
                    .contains_key(&v.parent_plan_id)
                    .then_some((*k, v.clone()))
            })
            .collect();
        let ports: HashMap<Uuid, Port> = self
            .ports
            .iter()
            .filter_map(|(k, v)| {
                operators
                    .contains_key(&v.parent_operator_id)
                    .then_some((*k, v.clone()))
            })
            .collect();
        let workers: HashMap<Uuid, Worker> = plans
            .values()
            .filter_map(|v| v.worker_id.and_then(|id| self.workers.get(&id)))
            .map(|w| (w.id, w.clone()))
            .collect();

        let mut entities = Self {
            engine,
            query_groups,
            workers,
            queries,
            plans,
            operators,
            ports,
            resource_groups: HashMap::new(),
            resources: HashMap::new(),
        };

        // ResourceGroups are a bit trickier as they can be nested. In order
        // to know whether we should add a resource group, we need to find its
        // non-resource group parent. If that is in any of the other filtered
        // entities, we know we need to add it.
        let resource_groups = self
            .resource_groups
            .iter()
            .filter_map(|(k, v)| {
                self.resource_group_tree_root(*k).and_then(|non_rg_parent| {
                    entities.contains(non_rg_parent).then_some((*k, v.clone()))
                })
            })
            .collect();
        entities.resource_groups = resource_groups;

        let resources = self
            .resources
            .iter()
            .filter_map(|(k, v)| {
                v.scope
                    .and_then(|scope| entities.contains(scope).then_some((*k, v.clone())))
            })
            .collect();
        entities.resources = resources;

        Ok(entities)
    }

    /// Return references of all resources and resource groups directly under some entity.
    pub fn get_resources_within_scope(&self, entity: EntityRef) -> Vec<EntityRef> {
        let resources = self.resources.iter().filter_map(|(k, v)| {
            v.scope
                .and_then(|s| (s == entity).then_some(EntityRef::Resource(*k)))
        });

        let resource_groups = self.resource_groups.iter().filter_map(|(k, v)| {
            v.scope
                .and_then(|s| (s == entity).then_some(EntityRef::ResourceGroup(*k)))
        });

        // TODO(johanpel): not collect
        resources.chain(resource_groups).collect()
    }

    /// Return true if reference to some entity is contained within this set.
    pub fn contains(&self, entity: EntityRef) -> bool {
        match entity {
            EntityRef::Engine(uuid) => self.engine.id == uuid,
            EntityRef::QueryGroup(uuid) => self.query_groups.contains_key(&uuid),
            EntityRef::Query(uuid) => self.queries.contains_key(&uuid),
            EntityRef::Plan(uuid) => self.plans.contains_key(&uuid),
            EntityRef::Worker(uuid) => self.workers.contains_key(&uuid),
            EntityRef::Operator(uuid) => self.operators.contains_key(&uuid),
            EntityRef::Port(uuid) => self.ports.contains_key(&uuid),
            EntityRef::ResourceGroup(uuid) => self.resource_groups.contains_key(&uuid),
            EntityRef::Resource(uuid) => self.resources.contains_key(&uuid),
        }
    }

    /// Traverse potential resource group parents until a non-resource group parent is found.
    pub fn resource_group_tree_root(&self, resource_group: Uuid) -> Option<EntityRef> {
        let mut current = self.resource_groups.get(&resource_group)?;
        loop {
            match current.scope? {
                EntityRef::ResourceGroup(uuid) => current = self.resource_groups.get(&uuid)?,
                other => return Some(other),
            }
        }
    }
}
