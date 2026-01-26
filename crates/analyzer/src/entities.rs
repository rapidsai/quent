use std::collections::{HashMap, HashSet};

use quent_entities::{
    EntityRef, IncompleteEntity,
    engine::Engine,
    fsm::{DynamicFsm, DynamicFsmStateDecl, DynamicFsmTypeDecl, Fsm, FsmBuilder},
    operator::{Operator, OperatorState, Port, WaitingForInputs},
    plan::Plan,
    query::Query,
    query_group::QueryGroup,
    relation::Related,
    resource::{
        CapacityDecl, CapacityValue, Resource, ResourceBuilder, ResourceGroup,
        ResourceOperatingState, ResourceState, ResourceTypeDecl,
    },
    worker::Worker,
};
#[cfg(feature = "q")]
use quent_entities::{fsm::DynamicState, resource::Use};
use quent_events::{
    EventData,
    engine::EngineEvent,
    operator::OperatorEvent,
    query::QueryEvent,
    query_group::QueryGroupEvent,
    resource::{channel, group as resource_group, memory, processor},
    worker::WorkerEvent,
};
use quent_time::{SpanNanoSec, TimeUnixNanoSec};

use serde::Serialize;
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
    T: IncompleteEntity,
{
    map.entry(id).or_insert_with(|| T::new(id))
}

/// A set of entities stored in maps from entity id -> entity.
///
/// The maps provide an easy way for quick lookups based on entity IDs.
/// This also means the values in the maps are flattened entities.
/// Their relations are solely expressed through IDs serving as references.
#[derive(Debug)]
pub struct Entities {
    // Domain-specific entities, subject to change:
    pub(crate) engine: Engine,
    pub(crate) workers: HashMap<Uuid, Worker>,
    pub(crate) query_groups: HashMap<Uuid, QueryGroup>,
    pub(crate) queries: HashMap<Uuid, Query>,
    pub(crate) plans: HashMap<Uuid, Plan>,
    pub(crate) operators: HashMap<Uuid, Operator>,
    pub(crate) ports: HashMap<Uuid, Port>,

    // Generic entities:
    pub(crate) resource_types: HashMap<String, ResourceTypeDecl>,
    pub(crate) resources: HashMap<Uuid, Resource>,
    pub(crate) resource_groups: HashMap<Uuid, ResourceGroup>,
    pub(crate) fsm_types: HashMap<String, DynamicFsmTypeDecl>,
    pub(crate) fsms: HashMap<Uuid, DynamicFsm>,

    /// The total span of time over all events.
    pub(crate) span: SpanNanoSec,
}

impl Entities {
    /// Consume all events and construct the application model in memory.
    ///
    /// Events are assumed to follow no particular order.
    pub fn try_new(engine_id: Uuid, events: impl Iterator<Item = Event>) -> Result<Self> {
        // TODO(johanpel): we need to sit down and think about how to do this as quickly as
        //                 possible for larger datasets, this is just a trivial implementation
        //                 to make it work. This is known to get pretty intense

        // Domain-specific entities, subject to change:
        let mut engine = Engine::new(engine_id);
        let mut query_groups: HashMap<Uuid, QueryGroup> = HashMap::new();
        let mut workers: HashMap<Uuid, Worker> = HashMap::new();
        let mut queries: HashMap<Uuid, Query> = HashMap::new();
        let mut plans: HashMap<Uuid, Plan> = HashMap::new();
        let mut operators: HashMap<Uuid, Operator> = HashMap::new();
        let mut ports: HashMap<Uuid, Port> = HashMap::new();

        // Generic entities:
        let mut resource_types: HashMap<String, ResourceTypeDecl> = HashMap::new();
        let mut resource_builders: HashMap<Uuid, ResourceBuilder> = HashMap::new();
        let mut resource_groups: HashMap<Uuid, ResourceGroup> = HashMap::new();

        #[allow(unused_mut)]
        let mut fsm_builders: HashMap<Uuid, FsmBuilder<DynamicState>> = HashMap::new();

        let mut earliest_event: TimeUnixNanoSec = TimeUnixNanoSec::MAX;
        let mut latest_event: TimeUnixNanoSec = 0;

        for event in events {
            let ts = event.timestamp;
            earliest_event = earliest_event.min(ts);
            latest_event = latest_event.max(ts);

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
                    let entry = entry(&mut resource_builders, event.id);
                    match resource_event {
                        quent_events::resource::ResourceEvent::Memory(memory_event) => {
                            match memory_event {
                                memory::MemoryEvent::Init(init) => {
                                    let type_decl = resource_types
                                        .entry(init.resource.type_name.clone())
                                        .or_insert_with(|| {
                                            ResourceTypeDecl::new(
                                                init.resource.type_name,
                                                [CapacityDecl::new_occupancy("bytes")],
                                            )
                                        });
                                    entry.push_state(ResourceState::Init(event.timestamp));
                                    entry.set_type_name(type_decl.name.clone());
                                    entry.set_instance_name(Some(init.resource.instance_name));
                                    entry.set_scope(init.resource.scope.into());
                                }
                                memory::MemoryEvent::Operating(operating) => entry.push_state(
                                    ResourceState::Operating(ResourceOperatingState {
                                        timestamp: event.timestamp,
                                        capacities: vec![CapacityValue::new(
                                            "bytes",
                                            operating.capacity_bytes,
                                        )],
                                    }),
                                ),
                                memory::MemoryEvent::Resizing(_) => {
                                    entry.push_state(ResourceState::Resizing(event.timestamp))
                                }
                                memory::MemoryEvent::Finalizing(_) => {
                                    entry.push_state(ResourceState::Finalizing(event.timestamp))
                                }
                                memory::MemoryEvent::Exit(_) => {
                                    entry.push_state(ResourceState::Exit(event.timestamp))
                                }
                            }
                        }
                        quent_events::resource::ResourceEvent::Processor(
                            processor_resource_event,
                        ) => match processor_resource_event {
                            processor::ProcessorEvent::Init(init) => {
                                let type_decl = resource_types
                                    .entry(init.resource.type_name.clone())
                                    .or_insert_with(|| {
                                        ResourceTypeDecl::unit(init.resource.type_name)
                                    });
                                entry.push_state(ResourceState::Init(event.timestamp));
                                entry.set_type_name(type_decl.name.clone());
                                entry.set_instance_name(Some(init.resource.instance_name));
                                entry.set_scope(init.resource.scope.into());
                            }
                            processor::ProcessorEvent::Operating(_) => {
                                entry.push_state(ResourceState::Operating(ResourceOperatingState {
                                    timestamp: event.timestamp,
                                    capacities: vec![],
                                }))
                            }
                            processor::ProcessorEvent::Finalizing(_) => {
                                entry.push_state(ResourceState::Finalizing(event.timestamp))
                            }
                            processor::ProcessorEvent::Exit(_) => {
                                entry.push_state(ResourceState::Exit(event.timestamp))
                            }
                        },
                        quent_events::resource::ResourceEvent::Channel(channel_resource_event) => {
                            match channel_resource_event {
                                channel::ChannelEvent::Init(init) => {
                                    let type_decl = resource_types
                                        .entry(init.resource.type_name.clone())
                                        .or_insert_with(|| {
                                            ResourceTypeDecl::new(
                                                init.resource.type_name,
                                                [CapacityDecl::new_occupancy("bytes")],
                                            )
                                        });
                                    entry.push_state(ResourceState::Init(event.timestamp));
                                    entry.set_type_name(type_decl.name.clone());
                                    entry.set_instance_name(Some(init.resource.instance_name));
                                    entry.set_scope(init.resource.scope.into());
                                }
                                channel::ChannelEvent::Operating(_) => entry.push_state(
                                    ResourceState::Operating(ResourceOperatingState {
                                        timestamp: event.timestamp,
                                        capacities: vec![],
                                    }),
                                ),
                                channel::ChannelEvent::Finalizing(_) => {
                                    entry.push_state(ResourceState::Finalizing(event.timestamp))
                                }
                                channel::ChannelEvent::Exit(_) => {
                                    entry.push_state(ResourceState::Exit(event.timestamp))
                                }
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
                            let entry = entry(&mut fsm_builders, event.id);
                            match task_event {
                                TaskEvent::Init(init) => entry
                                    .with_type_name("task")
                                    .with_instance_name(init.name)
                                    .push_state(DynamicState {
                                        name: "init".into(),
                                        uses: vec![],
                                        timestamp: event.timestamp,
                                        attributes: vec![],
                                        relations: vec![EntityRef::Operator(init.operator_id)],
                                    }),
                                TaskEvent::Queueing(_queueing) => entry.push_state(DynamicState {
                                    name: "queueing".into(),
                                    uses: vec![],
                                    timestamp: event.timestamp,
                                    attributes: vec![],
                                    relations: vec![],
                                }),
                                TaskEvent::AllocatingMemory(allocating_memory) => {
                                    entry.push_state(DynamicState {
                                        name: "allocating memory".into(),
                                        uses: vec![Use::unit(allocating_memory.use_task_thread)],
                                        timestamp: event.timestamp,
                                        attributes: vec![],
                                        relations: vec![],
                                    });
                                }
                                TaskEvent::Loading(loading) => entry.push_state(DynamicState {
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
                                TaskEvent::AllocatingStorage(allocating_storage) => entry
                                    .push_state(DynamicState {
                                        name: "allocating storage".into(),
                                        uses: vec![Use::unit(allocating_storage.use_task_thread)],
                                        timestamp: event.timestamp,
                                        attributes: vec![],
                                        relations: vec![],
                                    }),
                                TaskEvent::Spilling(spilling) => entry.push_state(DynamicState {
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
                                TaskEvent::Sending(sending) => entry.push_state(DynamicState {
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
                                TaskEvent::Computing(computing) => entry.push_state(DynamicState {
                                    name: "computing".into(),
                                    uses: vec![
                                        Use::unit(computing.use_task_thread),
                                        Use::new(
                                            computing.use_main_memory,
                                            [CapacityValue::new(
                                                "bytes",
                                                computing.use_main_memory_bytes,
                                            )],
                                        ),
                                    ],
                                    timestamp: event.timestamp,
                                    attributes: vec![],
                                    relations: vec![],
                                }),
                                TaskEvent::Finalizing(_finalizing) => {
                                    entry.push_state(DynamicState {
                                        name: "finalizing".into(),
                                        uses: vec![],
                                        timestamp: event.timestamp,
                                        attributes: vec![],
                                        relations: vec![],
                                    })
                                }
                                TaskEvent::Exit(_exit) => entry.push_state(DynamicState {
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

        // Build all resources
        let resources: HashMap<Uuid, Resource> = resource_builders
            .into_iter()
            .map(|(id, builder)| (id, builder.try_build()))
            .collect();

        // Build all FSMs, derive FSM types from the data, and populate the
        // "used_by_fsm" field of resources.
        let mut fsms: HashMap<Uuid, DynamicFsm> = HashMap::with_capacity(fsm_builders.capacity());
        let mut fsm_types: HashMap<String, DynamicFsmTypeDecl> = HashMap::new();

        for (k, fsm) in fsm_builders.into_iter() {
            // TODO(johanpel): for now bubble up this error but if there are
            // e.g. abrupt failures we may want to move incomplete FSMs into
            // their own bucket.
            let fsm = fsm.try_build()?;

            // Check whether this type decl already exists.
            // TODO(johanpel): consider either adding FSM type decl events or
            // generating all this with a DSL instead of having to do this, or
            // worst case still do this work but parallelize this don't clone so
            // many strings.
            let fsm_type_decl = fsm_types
                .entry(fsm.type_name().to_owned())
                .or_insert_with(|| {
                    DynamicFsmTypeDecl::new(
                        fsm.type_name().to_owned(),
                        fsm.states()
                            .map(|state| DynamicFsmStateDecl::new(state.name.clone())),
                    )
                });
            for state in fsm.states() {
                fsm_type_decl.insert(DynamicFsmStateDecl::new(state.name.clone()));
                for usage in state.uses.iter() {
                    if let Some(resource) = resources.get(&usage.resource)
                        && let Some(resource_type_decl) =
                            resource_types.get_mut(&resource.type_name)
                    {
                        resource_type_decl
                            .used_by_fsms
                            .insert(fsm.type_name().to_owned());
                    }
                }
            }

            fsms.insert(k, fsm);
        }

        Ok(Self {
            engine,
            query_groups,
            workers,
            queries,
            plans,
            operators,
            ports,
            resource_types,
            resources,
            resource_groups,
            fsm_types,
            fsms,
            span: SpanNanoSec::try_new(earliest_event, latest_event.saturating_add(1))?,
        })
    }

    /// Clone all entities that are involved in executing one single query.
    // TODO(johanpel): we need to sit down and think about this. If you take the
    // output of this and do analysis on it, you're not going to know what went
    // on in resources shared across multiple concurrently running workloads. It
    // would probably be best to calculate the start state within this filtered
    // dataset of the resources, then include all entities with a time-based
    // filter.
    pub fn try_filter_by_query(&self, query_id: Uuid) -> Result<Self> {
        let engine = self.engine.clone();

        let query = self
            .queries
            .get(&query_id)
            .ok_or(Error::InvalidId(query_id))?;
        // TODO(johanpel): for now :tm: assume that the query timestamps exceed
        // any other query-related timestamps in both directions, but this would
        // ultimately need to be determined otherwise.
        let span = SpanNanoSec::try_new(
            query.timestamps.init.unwrap_or(self.span.start()),
            query.timestamps.exit.unwrap_or(self.span.end()),
        )?;
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
            resource_types: self.resource_types.clone(),
            resources: HashMap::new(),
            resource_groups: HashMap::new(),
            fsm_types: self.fsm_types.clone(),
            fsms: HashMap::new(),
            span,
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
        entities.fsms = self
            .fsms
            .iter()
            .filter_map(|(id, fsm)| {
                fsm.relations()
                    .all(|rel| entities.contains(rel))
                    .then_some((*id, fsm.clone()))
            })
            .collect();

        Ok(entities)
    }

    pub fn filter_by_time_window(&self, window: SpanNanoSec) -> Entities {
        Self {
            // TODO(johanpel): domain specific things shouldn't all be cloned.
            engine: self.engine.clone(),
            workers: self.workers.clone(),
            query_groups: self.query_groups.clone(),
            queries: self.queries.clone(),
            plans: self.plans.clone(),
            operators: self.operators.clone(),
            ports: self.ports.clone(),
            // TODO(johanpel): filter the below
            resource_types: self.resource_types.clone(),
            resources: self.resources.clone(),
            resource_groups: self.resource_groups.clone(),
            fsm_types: self.fsm_types.clone(),
            fsms: self.fsms.clone(),
            span: window,
        }
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
            EntityRef::Fsm(uuid) => self.fsms.contains_key(&uuid),
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
        self.fsms
            .iter()
            .flat_map(|(id, fsm)| fsm.use_relations().map(|rel| (EntityRef::Fsm(*id), rel)))
    }

    pub(crate) fn iter_dynamic_fsms(&self, resource_id: Uuid) -> impl Iterator<Item = &DynamicFsm> {
        self
            // Iterate over all entities that use resources
            .iter_use_relations()
            // Filter out entities that don't use the target resource
            .filter_map(move |(user, resource)| {
                (Uuid::from(resource) == resource_id).then_some(user)
            })
            // Filter out anything that's not an FSM for now :tm:
            .filter_map(|user| match user {
                quent_entities::EntityRef::Fsm(uuid) => Some(self.fsms.get(&uuid).unwrap()),
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
        let custom_fsms = self.fsms.values().map(|fsm| fsm.type_name().to_owned());

        base.chain(resources)
            .chain(resource_groups)
            .chain(custom_fsms)
            .collect::<HashSet<_>>()
            .into_iter()
    }

    #[inline]
    pub(crate) fn resource(&self, resource_id: Uuid) -> Result<&Resource> {
        self.resources
            .get(&resource_id)
            .ok_or(Error::InvalidId(resource_id))
    }

    pub(crate) fn resource_type(&self, resource_type_name: &str) -> Result<&ResourceTypeDecl> {
        self.resource_types
            .get(resource_type_name)
            .ok_or(Error::InvalidTypeName(format!(
                "unknown resource type {resource_type_name}"
            )))
    }
}

/// A set of high-level manageable entities stored in maps from entity id -> entity.
///
/// The maps provide an easy way for quick lookups based on entity IDs.
/// This also means the values in the maps are flattened entities.
/// Their relations are solely expressed through IDs.
#[derive(TS, Serialize, Debug)]
pub struct EntitiesUI {
    pub engine: Engine,
    pub workers: HashMap<Uuid, Worker>,
    pub resources_types: HashMap<String, ResourceTypeDecl>,
    pub resources: HashMap<Uuid, Resource>,
    pub resource_groups: HashMap<Uuid, ResourceGroup>,
    pub query_groups: HashMap<Uuid, QueryGroup>,
    pub queries: HashMap<Uuid, Query>,
    pub plans: HashMap<Uuid, Plan>,
    pub operators: HashMap<Uuid, Operator>,
    pub ports: HashMap<Uuid, Port>,
    pub fsm_types: HashMap<String, DynamicFsmTypeDecl>,
}

impl From<Entities> for EntitiesUI {
    fn from(value: Entities) -> Self {
        Self {
            // Domain-specific entities
            engine: value.engine.clone(),
            workers: value.workers.clone(),
            query_groups: value.query_groups.clone(),
            queries: value.queries.clone(),
            plans: value.plans.clone(),
            operators: value.operators.clone(),
            ports: value.ports.clone(),
            // Generic entities:
            resources_types: value.resource_types.clone(),
            resource_groups: value.resource_groups.clone(),
            // TODO(johanpel): This has the potential to get huge too, figure
            // out how to mitigate:
            resources: value.resources.clone(),
            fsm_types: value.fsm_types.clone(),
        }
    }
}
