use std::collections::{HashMap, HashSet};

use quent_entities::{
    Entity, EntityRef,
    engine::Engine,
    fsm::Fsm,
    operator::{Operator, OperatorState, Port, WaitingForInputs},
    plan::Plan,
    query::Query,
    query_group::QueryGroup,
    relation::Related,
    resource::{
        CapacityDecl, CapacityValue, Resource, ResourceGroup, ResourceOperatingState, ResourceState,
    },
    worker::Worker,
};
#[cfg(feature = "q")]
use quent_entities::{fsm::State, resource::Use};
use quent_events::{
    EventData,
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
    T: Entity,
{
    map.entry(id).or_insert_with(|| T::new(id))
}

/// A set of entities stored in maps from entity id -> entity.
///
/// The maps provide an easy way for quick lookups based on entity IDs.
/// This also means the values in the maps are flattened entities.
/// Their relations are solely expressed through IDs serving as references.
#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct Entities {
    pub engine: Engine,
    pub workers: HashMap<Uuid, Worker>,
    pub resource_groups: HashMap<Uuid, ResourceGroup>,
    pub resources: HashMap<Uuid, Resource>,
    pub query_groups: HashMap<Uuid, QueryGroup>,
    pub queries: HashMap<Uuid, Query>,
    pub plans: HashMap<Uuid, Plan>,
    pub operators: HashMap<Uuid, Operator>,
    pub ports: HashMap<Uuid, Port>,
    // TODO(johanpel): don't send this to the UI, this can get very big
    pub custom_fsms: HashMap<Uuid, Fsm>,
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
        #[allow(unused_mut)]
        let mut custom_fsms: HashMap<Uuid, Fsm> = HashMap::new();

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
                            entry.name = Some(init.name);
                            entry.query_id = Some(init.query_id);
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
                                    entry
                                        .state_sequence
                                        .push(ResourceState::Init(event.timestamp));
                                    entry.capacities = HashMap::from([(
                                        "bytes".to_string(),
                                        CapacityDecl::new_occupancy("bytes"),
                                    )]);
                                    entry.type_name = init.resource.type_name;
                                    entry.instance_name = Some(init.resource.instance_name);
                                    entry.scope = Some(init.resource.scope.into());
                                }
                                memory::MemoryEvent::Operating(operating) => entry
                                    .state_sequence
                                    .push(ResourceState::Operating(ResourceOperatingState {
                                        timestamp: event.timestamp,
                                        capacities: vec![CapacityValue::new(
                                            "bytes",
                                            operating.capacity_bytes,
                                        )],
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
                                entry
                                    .state_sequence
                                    .push(ResourceState::Init(event.timestamp));
                                entry.type_name = init.resource.type_name;
                                entry.instance_name = Some(init.resource.instance_name);
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
                                    entry
                                        .state_sequence
                                        .push(ResourceState::Init(event.timestamp));
                                    entry.capacities = HashMap::from([(
                                        "bytes".to_string(),
                                        CapacityDecl::new_rate("bytes"),
                                    )]);
                                    entry.type_name = init.resource.type_name;
                                    entry.instance_name = Some(init.resource.instance_name);
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
                            entry.type_name = Some(init.resource.type_name);
                            entry.instance_name = Some(init.resource.instance_name);
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
                #[cfg(feature = "q")]
                EventData::Q(q_event) => {
                    use quent_events::q;
                    match q_event {
                        q::QEvent::Task(task_event) => {
                            use quent_events::q::task::TaskEvent;
                            let entry = entry(&mut custom_fsms, event.id);
                            match task_event {
                                TaskEvent::Initializing(initializing) => {
                                    entry.type_name = "task".into();
                                    entry.instance_name = initializing.name;
                                    entry.state_sequence.push(State {
                                        name: "init".into(),
                                        uses: vec![],
                                        timestamp: event.timestamp,
                                        attributes: vec![],
                                        relations: vec![EntityRef::Operator(
                                            initializing.operator_id,
                                        )],
                                    })
                                }
                                TaskEvent::Queueing(_queueing) => {
                                    entry.state_sequence.push(State {
                                        name: "queueing".into(),
                                        uses: vec![],
                                        timestamp: event.timestamp,
                                        attributes: vec![],
                                        relations: vec![],
                                    })
                                }
                                TaskEvent::AllocatingMemory(allocating_memory) => {
                                    entry.state_sequence.push(State {
                                        name: "allocating memory".into(),
                                        uses: vec![Use::unit(allocating_memory.use_task_thread)],
                                        timestamp: event.timestamp,
                                        attributes: vec![],
                                        relations: vec![],
                                    })
                                }
                                TaskEvent::Loading(loading) => entry.state_sequence.push(State {
                                    name: "loading".into(),
                                    uses: vec![
                                        Use::unit(loading.use_task_thread),
                                        Use::new(
                                            loading.use_fs_to_mem,
                                            vec![CapacityValue::new(
                                                "bytes",
                                                loading.use_fs_to_mem_bytes,
                                            )],
                                        ),
                                        Use::new(
                                            loading.use_main_memory,
                                            vec![CapacityValue::new(
                                                "bytes",
                                                loading.use_main_memory_bytes,
                                            )],
                                        ),
                                    ],
                                    timestamp: event.timestamp,
                                    attributes: vec![],
                                    relations: vec![],
                                }),
                                TaskEvent::AllocatingStorage(allocating_storage) => {
                                    entry.state_sequence.push(State {
                                        name: "allocating storage".into(),
                                        uses: vec![Use::unit(allocating_storage.use_task_thread)],
                                        timestamp: event.timestamp,
                                        attributes: vec![],
                                        relations: vec![],
                                    })
                                }
                                TaskEvent::Spilling(spilling) => entry.state_sequence.push(State {
                                    name: "spilling".into(),
                                    uses: vec![
                                        Use::unit(spilling.use_task_thread),
                                        Use::new(
                                            spilling.use_mem_to_fs,
                                            vec![CapacityValue::new(
                                                "bytes",
                                                spilling.use_mem_to_fs_bytes,
                                            )],
                                        ),
                                    ],
                                    timestamp: event.timestamp,
                                    attributes: vec![],
                                    relations: vec![],
                                }),
                                TaskEvent::Sending(sending) => entry.state_sequence.push(State {
                                    name: "sending".into(),
                                    uses: vec![
                                        Use::unit(sending.use_task_thread),
                                        Use::new(
                                            sending.use_link,
                                            vec![CapacityValue::new(
                                                "bytes",
                                                sending.use_link_bytes,
                                            )],
                                        ),
                                    ],
                                    timestamp: event.timestamp,
                                    attributes: vec![],
                                    relations: vec![],
                                }),
                                TaskEvent::Computing(computing) => {
                                    entry.state_sequence.push(State {
                                        name: "computing".into(),
                                        uses: vec![
                                            Use::unit(computing.use_task_thread),
                                            Use {
                                                resource: computing.use_main_memory,
                                                capacities: vec![CapacityValue::new(
                                                    "bytes",
                                                    computing.use_main_memory_bytes,
                                                )],
                                            },
                                        ],
                                        timestamp: event.timestamp,
                                        attributes: vec![],
                                        relations: vec![],
                                    })
                                }
                                TaskEvent::Finalizing(_finalizing) => {
                                    entry.state_sequence.push(State {
                                        name: "finalizing".into(),
                                        uses: vec![],
                                        timestamp: event.timestamp,
                                        attributes: vec![],
                                        relations: vec![],
                                    })
                                }
                                TaskEvent::Exit(_exit) => entry.state_sequence.push(State {
                                    name: "exit".into(),
                                    uses: vec![],
                                    timestamp: event.timestamp,
                                    attributes: vec![],
                                    relations: vec![],
                                }),
                            }
                        }
                        // TODO(johanpel): record batches
                        quent_events::q::QEvent::RecordBatch(_record_batch_event) => (),
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
            custom_fsms,
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
            if let Some(query_id) = self.plans.get(&key).unwrap().query_id
                && !self.queries.contains_key(&query_id)
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
        // Filter ports that don't refer to any operators
        for key in self.ports.keys().cloned().collect::<Vec<_>>() {
            if !self
                .operators
                .contains_key(&self.ports.get(&key).unwrap().parent_operator_id)
            {
                self.ports.remove(&key);
            }
        }

        // Gather all IDs of required entities into a set.
        let mut parent_entity_ids: HashSet<Uuid> = HashSet::with_capacity(
            self.workers.len()
                + self.query_groups.len()
                + self.queries.len()
                + self.plans.len()
                + self.operators.len()
                + self.ports.len(),
        );
        parent_entity_ids.extend(self.workers.keys());
        parent_entity_ids.extend(self.query_groups.keys());
        parent_entity_ids.extend(self.queries.keys());
        parent_entity_ids.extend(self.plans.keys());
        parent_entity_ids.extend(self.operators.keys());
        parent_entity_ids.extend(self.ports.keys());
        parent_entity_ids.insert(self.engine.id);

        // Filter resource groups that don't refer to any of the above entities.
        for key in self.resource_groups.keys().cloned().collect::<Vec<_>>() {
            if let Some(root) = self.resource_group_tree_root(key) {
                if !parent_entity_ids.contains(&root.into()) {
                    self.resource_groups.remove(&key);
                }
            } else {
                self.resource_groups.remove(&key);
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

        // TODO(johanpel): Filter custom FSMs that don't refer to any of the above entities.
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
            .filter_map(|(k, v)| (v.query_id == Some(query_id)).then_some((*k, v.clone())))
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
            custom_fsms: HashMap::new(),
        };

        // ResourceGroups are a bit trickier as they can be nested. In order
        // to know whether we should add a resource group, we need to find its
        // non-resource group parent. If that is in any of the other filtered
        // entities, we know we need to add it.
        entities.resource_groups = self
            .resource_groups
            .iter()
            .filter_map(|(k, v)| {
                self.resource_group_tree_root(*k).and_then(|non_rg_parent| {
                    entities.contains(non_rg_parent).then_some((*k, v.clone()))
                })
            })
            .collect();

        entities.resources = self
            .resources
            .iter()
            .filter_map(|(k, v)| {
                v.scope
                    .and_then(|scope| entities.contains(scope).then_some((*k, v.clone())))
            })
            .collect();

        // samesies for custom entities
        entities.custom_fsms = self
            .custom_fsms
            .iter()
            .filter_map(|(id, fsm)| {
                fsm.relations()
                    .all(|rel| entities.contains(rel))
                    .then_some((*id, fsm.clone()))
            })
            .collect();

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
            EntityRef::CustomFsm(uuid) => self.custom_fsms.contains_key(&uuid),
        }
    }

    /// Traverse potential resource group parents until a non-resource group parent is found.
    fn resource_group_tree_root(&self, resource_group: Uuid) -> Option<EntityRef> {
        let mut current = self.resource_groups.get(&resource_group)?;
        loop {
            match current.scope? {
                EntityRef::ResourceGroup(uuid) => current = self.resource_groups.get(&uuid)?,
                other => return Some(other),
            }
        }
    }

    pub(crate) fn iter_use_relations(&self) -> impl Iterator<Item = (EntityRef, EntityRef)> {
        self.custom_fsms.iter().flat_map(|(id, fsm)| {
            fsm.use_relations()
                .map(|rel| (EntityRef::CustomFsm(*id), rel))
        })
    }

    pub(crate) fn iter_custom_fsms_using_resource(
        &self,
        resource_id: Uuid,
    ) -> impl Iterator<Item = &Fsm> {
        self
            // Iterate over all entities that use resources
            .iter_use_relations()
            // Filter out entities that don't use the target resource
            .filter_map(move |(user, resource)| {
                (Uuid::from(resource) == resource_id).then_some(user)
            })
            // Filter out anything that's not an FSM for now :tm:
            .filter_map(|user| match user {
                quent_entities::EntityRef::CustomFsm(uuid) => {
                    Some(self.custom_fsms.get(&uuid).unwrap())
                }
                _ => None,
            })
    }

    pub(crate) fn unique_operator_names(&self) -> impl Iterator<Item = &str> {
        self.operators
            .values()
            .filter_map(|op| op.name.as_deref())
            .collect::<HashSet<_>>()
            .into_iter()
    }

    pub(crate) fn unique_entity_type_names(&self) -> impl Iterator<Item = String> {
        // TODO(johanpel): consider allowing folks to rename these concepts for rendering
        let base = [
            "Engine",
            "Worker",
            "QueryGroup",
            "Query",
            "Operator",
            "Port",
        ]
        .into_iter()
        .map(ToString::to_string);
        let resources = self.resources.values().map(|res| res.type_name.clone());
        let resource_groups = self
            .resource_groups
            .values()
            .filter_map(|res| res.type_name.as_ref())
            .cloned();
        let custom_fsms = self.custom_fsms.values().map(|fsm| fsm.type_name.clone());

        base.chain(resources)
            .chain(resource_groups)
            .chain(custom_fsms)
            .collect::<HashSet<_>>()
            .into_iter()
    }

    /// Return an EntityRef for the provided id.
    pub(crate) fn get_entity_ref_from_id(&self, id: Uuid) -> Option<EntityRef> {
        if self.engine.id == id {
            return Some(EntityRef::Engine(id));
        }
        if self.query_groups.contains_key(&id) {
            return Some(EntityRef::QueryGroup(id));
        }
        if self.queries.contains_key(&id) {
            return Some(EntityRef::Query(id));
        }
        if self.plans.contains_key(&id) {
            return Some(EntityRef::Plan(id));
        }
        if self.workers.contains_key(&id) {
            return Some(EntityRef::Worker(id));
        }
        if self.operators.contains_key(&id) {
            return Some(EntityRef::Operator(id));
        }
        if self.ports.contains_key(&id) {
            return Some(EntityRef::Port(id));
        }
        if self.resource_groups.contains_key(&id) {
            return Some(EntityRef::ResourceGroup(id));
        }
        if self.resources.contains_key(&id) {
            return Some(EntityRef::Resource(id));
        }
        if self.custom_fsms.contains_key(&id) {
            return Some(EntityRef::CustomFsm(id));
        }
        None
    }
}
