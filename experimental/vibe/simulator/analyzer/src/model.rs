// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeSet, hash_map::Entry};

use rustc_hash::FxHashMap as HashMap;

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Model, RefTreeEntity,
    fsm::collection::FsmCollection,
    ref_tree::RefTreeCollection,
    resource::{Resource, ResourceTypeDecl, Usage, Using, collection::ResourceCollection},
};
use quent_events::{Event, EventPayload};
use quent_query_engine_analyzer::{
    OperatorEntityMut, QueryEngineModel, QueryEngineModelMut, plan_tree::PlanTree,
};
use quent_query_engine_ui::EntityRef;
use quent_simulator_store::{
    self as schema, SimulatorEvent, gpu::GpuEvents, gpu_memory::GpuMemoryEvents,
    host_memory::HostMemoryEvents, network::NetworkEvents, network_channel::NetworkChannelEvents,
    pcie_channel::PcieChannelEvents, storage::StorageEvents, storage_channel::StorageChannelEvents,
    task_executor::TaskExecutorEvents, task_executor_thread::TaskExecutorThreadEvents,
};
use quent_store::entity::{
    Id,
    native::{NativeEntityMarker, NativeStore, NativeStoreBuilder},
};
use quent_ui::{FsmTypeSelection, ResourceGroupTypeDecl};
use uuid::Uuid;

pub use crate::boilerplate::{Engine, Operator, Plan, Port, Query, Worker};

use crate::{
    boilerplate::{
        EntityAnalysis, Gpu, GpuMemory, HostMemory, Network, NetworkChannel, PcieChannel,
        QueryBuilder, Storage, StorageChannel, Task, TaskBuilder, TaskExecutor, TaskExecutorThread,
        TaskExt,
    },
    view::SimulatorModelQueryView,
};

const RESOURCE_TYPE_DISPLAY_ORDER: [&str; 7] = [
    schema::TaskExecutorThreadEvent::NAME,
    schema::HostMemoryEvent::NAME,
    schema::GpuMemoryEvent::NAME,
    schema::NetworkChannelEvent::NAME,
    schema::PcieChannelEvent::NAME,
    schema::StorageChannelEvent::NAME,
    schema::StorageEvent::NAME,
];

fn derive_resource_scope_types(
    model: &SimulatorModel,
) -> AnalyzerResult<HashMap<String, ResourceGroupTypeDecl>> {
    fn populate(
        node: &quent_analyzer::resource::tree::ResourceTreeNode,
        model: &SimulatorModel,
        declarations: &mut HashMap<String, (Vec<FsmTypeSelection>, BTreeSet<String>)>,
    ) -> AnalyzerResult<()> {
        if !node.is_resource {
            let mut contained_types = Vec::new();
            for resource_id in node.iter_resource_ids() {
                contained_types.push(model.resource_type_of(resource_id)?);
            }
            if !contained_types.is_empty() {
                let type_name = model
                    .ref_tree_entity(node.entity_id)?
                    .type_name()
                    .to_owned();
                let (used_by, contains) = declarations.entry(type_name).or_default();
                for resource_type in contained_types {
                    contains.insert(resource_type.name.clone());
                    for entity_type in &resource_type.used_by {
                        let entity_type = FsmTypeSelection::Type(entity_type.to_string());
                        if !used_by.contains(&entity_type) {
                            used_by.push(entity_type);
                        }
                    }
                }
            }
        }
        for child in &node.children {
            populate(child, model, declarations)?;
        }
        Ok(())
    }

    let tree = quent_analyzer::resource::tree::ResourceTreeNode::try_new(model)?;
    let mut declarations = HashMap::default();
    populate(&tree, model, &mut declarations)?;
    Ok(declarations
        .into_iter()
        .map(|(name, (used_by_entity_types, contains_resource_types))| {
            let mut resource_type_order: Vec<_> = contains_resource_types.into_iter().collect();
            resource_type_order.sort_by_key(|name| {
                RESOURCE_TYPE_DISPLAY_ORDER
                    .iter()
                    .position(|&resource_type| resource_type == name)
                    .unwrap_or(usize::MAX)
            });
            let declaration = ResourceGroupTypeDecl {
                name: name.clone(),
                display_order: std::iter::once(FsmTypeSelection::All)
                    .chain(used_by_entity_types)
                    .collect(),
                contains_resource_types: resource_type_order,
            };
            (name, declaration)
        })
        .collect())
}

/// A model of the simulator engine
pub struct SimulatorModel {
    engine_id: Uuid,
    pub(crate) engine: NativeStore<schema::Engine, Engine>,
    pub(crate) workers: NativeStore<schema::Worker, Worker>,
    pub(crate) queries: NativeStore<schema::Query, Query>,
    pub(crate) plans: NativeStore<schema::Plan, Plan>,
    pub(crate) operators: NativeStore<schema::Operator, Operator>,
    pub(crate) ports: NativeStore<schema::Port, Port>,
    pub(crate) resource_types: HashMap<String, ResourceTypeDecl>,
    pub(crate) host_memories: NativeStore<schema::HostMemory, HostMemory>,
    pub(crate) storages: NativeStore<schema::Storage, Storage>,
    pub(crate) gpu_memories: NativeStore<schema::GpuMemory, GpuMemory>,
    pub(crate) task_executor_threads: NativeStore<schema::TaskExecutorThread, TaskExecutorThread>,
    pub(crate) storage_channels: NativeStore<schema::StorageChannel, StorageChannel>,
    pub(crate) pcie_channels: NativeStore<schema::PcieChannel, PcieChannel>,
    pub(crate) network_channels: NativeStore<schema::NetworkChannel, NetworkChannel>,
    pub(crate) task_executors: NativeStore<schema::TaskExecutor, TaskExecutor>,
    pub(crate) networks: NativeStore<schema::Network, Network>,
    pub(crate) gpus: NativeStore<schema::Gpu, Gpu>,
    pub(crate) tasks: NativeStore<schema::Task, Task>,
    pub(crate) resource_group_types: HashMap<String, ResourceGroupTypeDecl>,
}

impl Model for SimulatorModel {
    type EntityIdType = EntityRef;

    fn try_entity_ref(&self, entity_id: Uuid) -> AnalyzerResult<Self::EntityIdType> {
        if self.engine_id == entity_id {
            Ok(EntityRef::Engine(entity_id))
        } else if self.workers.contains_key(entity_id) {
            Ok(EntityRef::Worker(entity_id))
        } else if self.queries.contains_key(entity_id) {
            Ok(EntityRef::Query(entity_id))
        } else if self.plans.contains_key(entity_id) {
            Ok(EntityRef::Plan(entity_id))
        } else if self.operators.contains_key(entity_id) {
            Ok(EntityRef::Operator(entity_id))
        } else if self.ports.contains_key(entity_id) {
            Ok(EntityRef::Port(entity_id))
        } else if self.resource(entity_id).is_ok() {
            Ok(EntityRef::Resource(entity_id))
        } else if self.task_executors.contains_key(entity_id)
            || self.networks.contains_key(entity_id)
            || self.gpus.contains_key(entity_id)
        {
            Ok(EntityRef::ResourceGroup(entity_id))
        } else {
            self.tasks
                .get(entity_id)
                .map(|task| EntityRef::Application {
                    type_name: task.type_name().to_owned(),
                    id: entity_id,
                })
                .ok_or(AnalyzerError::InvalidId(entity_id))
        }
    }
}

impl QueryEngineModel for SimulatorModel {
    type Engine = Engine;
    type Query = Query;
    type Worker = Worker;
    type Plan = Plan;
    type Operator = Operator;
    type Port = Port;

    fn engine(&self) -> AnalyzerResult<&Engine> {
        self.engine
            .get(self.engine_id)
            .ok_or(AnalyzerError::InvalidId(self.engine_id))
    }
    fn query(&self, query_id: Uuid) -> AnalyzerResult<&Query> {
        self.queries
            .get(query_id)
            .ok_or(AnalyzerError::InvalidId(query_id))
    }
    fn worker(&self, worker_id: Uuid) -> AnalyzerResult<&Worker> {
        self.workers
            .get(worker_id)
            .ok_or(AnalyzerError::InvalidId(worker_id))
    }
    fn plan(&self, plan_id: Uuid) -> AnalyzerResult<&Plan> {
        self.plans
            .get(plan_id)
            .ok_or(AnalyzerError::InvalidId(plan_id))
    }
    fn operator(&self, operator_id: Uuid) -> AnalyzerResult<&Operator> {
        self.operators
            .get(operator_id)
            .ok_or(AnalyzerError::InvalidId(operator_id))
    }
    fn port(&self, port_id: Uuid) -> AnalyzerResult<&Port> {
        self.ports
            .get(port_id)
            .ok_or(AnalyzerError::InvalidId(port_id))
    }
    fn queries(&self) -> impl Iterator<Item = &Query> {
        self.queries.values()
    }
    fn workers(&self) -> impl Iterator<Item = &Worker> {
        self.workers.values()
    }
    fn plans(&self) -> impl Iterator<Item = &Plan> {
        self.plans.values()
    }
    fn operators(&self) -> impl Iterator<Item = &Operator> {
        self.operators.values()
    }
    fn ports(&self) -> impl Iterator<Item = &Port> {
        self.ports.values()
    }
    fn plan_tree(&self, query_id: Uuid) -> AnalyzerResult<PlanTree> {
        PlanTree::try_new(self.plans.values(), query_id)
    }
}

impl QueryEngineModelMut for SimulatorModel {
    fn operator_mut(&mut self, operator_id: Uuid) -> AnalyzerResult<&mut Operator> {
        self.operators
            .get_mut(operator_id)
            .ok_or(AnalyzerError::InvalidId(operator_id))
    }
}

impl SimulatorModel {
    pub(crate) fn query_view(&self, query_id: Uuid) -> AnalyzerResult<SimulatorModelQueryView<'_>> {
        SimulatorModelQueryView::try_new(self, query_id)
    }

    pub(crate) fn resource_instance_name(&self, resource_id: Uuid) -> Option<&str> {
        self.host_memories
            .get(resource_id)
            .map(|entity| entity.declaration().instance_name().as_str())
            .or_else(|| {
                self.storages
                    .get(resource_id)
                    .map(|entity| entity.declaration().instance_name().as_str())
            })
            .or_else(|| {
                self.gpu_memories
                    .get(resource_id)
                    .map(|entity| entity.declaration().instance_name().as_str())
            })
            .or_else(|| {
                self.task_executor_threads
                    .get(resource_id)
                    .map(|entity| entity.declaration().instance_name().as_str())
            })
            .or_else(|| {
                self.storage_channels
                    .get(resource_id)
                    .map(|entity| entity.declaration().instance_name().as_str())
            })
            .or_else(|| {
                self.pcie_channels
                    .get(resource_id)
                    .map(|entity| entity.declaration().instance_name().as_str())
            })
            .or_else(|| {
                self.network_channels
                    .get(resource_id)
                    .map(|entity| entity.declaration().instance_name().as_str())
            })
    }

    pub(crate) fn resource_scope_instance_name(&self, entity_id: Uuid) -> Option<&str> {
        self.task_executors
            .get(entity_id)
            .map(|entity| entity.declaration().instance_name().as_str())
            .or_else(|| {
                self.networks
                    .get(entity_id)
                    .map(|entity| entity.declaration().instance_name().as_str())
            })
            .or_else(|| {
                self.gpus
                    .get(entity_id)
                    .map(|entity| entity.declaration().instance_name().as_str())
            })
    }

    fn simulator_resource(&self, resource_id: Uuid) -> Option<&dyn Resource> {
        self.host_memories
            .get(resource_id)
            .map(|resource| resource as &dyn Resource)
            .or_else(|| {
                self.storages
                    .get(resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
            .or_else(|| {
                self.gpu_memories
                    .get(resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
            .or_else(|| {
                self.task_executor_threads
                    .get(resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
            .or_else(|| {
                self.storage_channels
                    .get(resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
            .or_else(|| {
                self.pcie_channels
                    .get(resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
            .or_else(|| {
                self.network_channels
                    .get(resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
    }

    fn simulator_resources(&self) -> impl Iterator<Item = &dyn Resource> {
        self.host_memories
            .values()
            .map(|resource| resource as &dyn Resource)
            .chain(
                self.storages
                    .values()
                    .map(|resource| resource as &dyn Resource),
            )
            .chain(
                self.gpu_memories
                    .values()
                    .map(|resource| resource as &dyn Resource),
            )
            .chain(
                self.task_executor_threads
                    .values()
                    .map(|resource| resource as &dyn Resource),
            )
            .chain(
                self.storage_channels
                    .values()
                    .map(|resource| resource as &dyn Resource),
            )
            .chain(
                self.pcie_channels
                    .values()
                    .map(|resource| resource as &dyn Resource),
            )
            .chain(
                self.network_channels
                    .values()
                    .map(|resource| resource as &dyn Resource),
            )
    }
}

impl FsmCollection for SimulatorModel {
    type Fsm = Task;

    fn fsms(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }
}

impl RefTreeCollection for SimulatorModel {
    fn ref_tree_entities(&self) -> impl Iterator<Item = &dyn RefTreeEntity> {
        self.engine
            .values()
            .map(|entity| entity as &dyn RefTreeEntity)
            .chain(
                self.workers
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.queries
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.plans
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.operators
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.ports
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.tasks
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.task_executors
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.networks
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.gpus
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.host_memories
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.storages
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.gpu_memories
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.task_executor_threads
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.storage_channels
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.pcie_channels
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.network_channels
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
    }

    fn ref_tree_entity(&self, entity_id: Uuid) -> AnalyzerResult<&dyn RefTreeEntity> {
        if self.engine_id == entity_id {
            self.engine().map(|entity| entity as &dyn RefTreeEntity)
        } else if let Some(entity) = self.workers.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.queries.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.plans.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.operators.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.ports.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.tasks.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.task_executors.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.networks.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.gpus.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.host_memories.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.storages.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.gpu_memories.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.task_executor_threads.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.storage_channels.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.pcie_channels.get(entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.network_channels.get(entity_id) {
            Ok(entity)
        } else {
            Err(AnalyzerError::InvalidId(entity_id))
        }
    }
}

impl ResourceCollection for SimulatorModel {
    fn resources(&self) -> impl Iterator<Item = &dyn Resource> {
        self.simulator_resources()
    }
    fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource> {
        self.simulator_resource(resource_id)
            .ok_or(AnalyzerError::InvalidId(resource_id))
    }
    fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl> {
        self.resource_types
            .get(resource_type_name)
            .ok_or_else(|| AnalyzerError::InvalidTypeName(resource_type_name.to_owned()))
    }
}

impl Using for SimulatorModel {
    fn usages(&self) -> impl Iterator<Item = impl Usage<'_>> {
        self.tasks.values().flat_map(|task| task.usages())
    }
}

fn build_entities<E>(
    builder: NativeStoreBuilder<E>,
) -> AnalyzerResult<NativeStore<E, EntityAnalysis<E>>>
where
    E: NativeEntityMarker,
{
    Ok(builder
        .try_build()
        .map_err(|error| AnalyzerError::Validation(error.to_string()))?
        .map_entities(EntityAnalysis::new))
}

pub struct SimulatorModelBuilder {
    engine_id: Uuid,
    engine: NativeStoreBuilder<schema::Engine>,
    workers: NativeStoreBuilder<schema::Worker>,
    queries: HashMap<Uuid, QueryBuilder>,
    plans: NativeStoreBuilder<schema::Plan>,
    operators: NativeStoreBuilder<schema::Operator>,
    ports: NativeStoreBuilder<schema::Port>,
    host_memories: NativeStoreBuilder<schema::HostMemory>,
    storages: NativeStoreBuilder<schema::Storage>,
    gpu_memories: NativeStoreBuilder<schema::GpuMemory>,
    task_executor_threads: NativeStoreBuilder<schema::TaskExecutorThread>,
    storage_channels: NativeStoreBuilder<schema::StorageChannel>,
    pcie_channels: NativeStoreBuilder<schema::PcieChannel>,
    network_channels: NativeStoreBuilder<schema::NetworkChannel>,
    task_executors: NativeStoreBuilder<schema::TaskExecutor>,
    networks: NativeStoreBuilder<schema::Network>,
    gpus: NativeStoreBuilder<schema::Gpu>,
    tasks: HashMap<Uuid, TaskBuilder>,
}

impl SimulatorModelBuilder {
    pub(crate) fn try_new(engine_id: Uuid) -> AnalyzerResult<Self> {
        if engine_id.is_nil() {
            return Err(AnalyzerError::Validation(
                "engine id cannot be nil".to_owned(),
            ));
        }
        Ok(Self {
            engine_id,
            engine: NativeStoreBuilder::default(),
            workers: NativeStoreBuilder::default(),
            queries: HashMap::default(),
            plans: NativeStoreBuilder::default(),
            operators: NativeStoreBuilder::default(),
            ports: NativeStoreBuilder::default(),
            host_memories: NativeStoreBuilder::default(),
            storages: NativeStoreBuilder::default(),
            gpu_memories: NativeStoreBuilder::default(),
            task_executor_threads: NativeStoreBuilder::default(),
            storage_channels: NativeStoreBuilder::default(),
            pcie_channels: NativeStoreBuilder::default(),
            network_channels: NativeStoreBuilder::default(),
            task_executors: NativeStoreBuilder::default(),
            networks: NativeStoreBuilder::default(),
            gpus: NativeStoreBuilder::default(),
            tasks: HashMap::default(),
        })
    }

    pub(crate) fn try_push(&mut self, event: Event<SimulatorEvent>) -> AnalyzerResult<()> {
        let Event {
            id,
            timestamp,
            data,
        } = event;
        if id.is_nil() {
            return Err(AnalyzerError::Validation(
                "entity id cannot be nil".to_owned(),
            ));
        }
        match data {
            SimulatorEvent::Task(event) => {
                let task_builder = match self.tasks.entry(id) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => entry.insert(TaskBuilder::try_new(id)?),
                };
                task_builder.push_transition(Event::new(id, timestamp, event));
            }
            SimulatorEvent::Query(event) => {
                let query_builder = match self.queries.entry(id) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => entry.insert(QueryBuilder::try_new(id)?),
                };
                query_builder.push_transition(Event::new(id, timestamp, event));
            }
            SimulatorEvent::Engine(event) => {
                if id != self.engine_id {
                    return Err(AnalyzerError::Validation(format!(
                        "multiple engine instances in one model: expected {}, found {id}",
                        self.engine_id
                    )));
                }
                self.engine.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::Worker(event) => {
                self.workers.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::Plan(event) => {
                self.plans.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::Operator(event) => {
                self.operators.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::Port(event) => {
                self.ports.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::HostMemory(event) => {
                self.host_memories.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::Storage(event) => {
                self.storages.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::GpuMemory(event) => {
                self.gpu_memories.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::TaskExecutorThread(event) => {
                self.task_executor_threads
                    .push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::StorageChannel(event) => {
                self.storage_channels.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::PcieChannel(event) => {
                self.pcie_channels.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::NetworkChannel(event) => {
                self.network_channels.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::TaskExecutor(event) => {
                self.task_executors.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::Network(event) => {
                self.networks.push(Event::new(id, timestamp, event));
            }
            SimulatorEvent::Gpu(event) => {
                self.gpus.push(Event::new(id, timestamp, event));
            }
        }
        Ok(())
    }

    pub(crate) fn try_build(self) -> AnalyzerResult<SimulatorModel> {
        let engine = build_entities::<schema::Engine>(self.engine)?;
        if engine.is_empty() {
            return Err(AnalyzerError::IncompleteEntity(format!(
                "engine {} has no events",
                self.engine_id
            )));
        }
        let queries = self
            .queries
            .into_iter()
            .map(|(id, builder)| Query::try_from_builder(builder).map(|query| (Id::new(id), query)))
            .collect::<AnalyzerResult<NativeStore<schema::Query, Query>>>()?;
        let resource_types = [
            HostMemory::resource_type_decl(),
            Storage::resource_type_decl(),
            GpuMemory::resource_type_decl(),
            TaskExecutorThread::resource_type_decl(),
            StorageChannel::resource_type_decl(),
            PcieChannel::resource_type_decl(),
            NetworkChannel::resource_type_decl(),
        ]
        .into_iter()
        .map(|declaration| (declaration.name.clone(), declaration))
        .collect();

        let mut model = SimulatorModel {
            engine_id: self.engine_id,
            engine,
            workers: build_entities::<schema::Worker>(self.workers)?,
            queries,
            plans: build_entities::<schema::Plan>(self.plans)?,
            operators: build_entities::<schema::Operator>(self.operators)?
                .map_entities(Operator::new),
            ports: build_entities::<schema::Port>(self.ports)?,
            resource_types,
            host_memories: build_entities::<schema::HostMemory>(self.host_memories)?,
            storages: build_entities::<schema::Storage>(self.storages)?,
            gpu_memories: build_entities::<schema::GpuMemory>(self.gpu_memories)?,
            task_executor_threads: build_entities::<schema::TaskExecutorThread>(
                self.task_executor_threads,
            )?,
            storage_channels: build_entities::<schema::StorageChannel>(self.storage_channels)?,
            pcie_channels: build_entities::<schema::PcieChannel>(self.pcie_channels)?,
            network_channels: build_entities::<schema::NetworkChannel>(self.network_channels)?,
            task_executors: build_entities::<schema::TaskExecutor>(self.task_executors)?,
            networks: build_entities::<schema::Network>(self.networks)?,
            gpus: build_entities::<schema::Gpu>(self.gpus)?,
            tasks: NativeStore::default(),
            resource_group_types: HashMap::default(),
        };

        for (task_id, task_builder) in self.tasks.into_iter() {
            let task = Task::from_builder(task_builder)?;
            let task_type_path = task
                .type_name()
                .parse()
                .map_err(|_| AnalyzerError::InvalidTypeName(task.type_name().to_owned()))?;
            for usage in task.usages() {
                let resource_type_name = model
                    .resource(usage.resource_id())
                    .map(Entity::type_name)?
                    .to_owned();
                let used_by = &mut model
                    .resource_types
                    .get_mut(&resource_type_name)
                    .ok_or_else(|| AnalyzerError::InvalidTypeName(resource_type_name.clone()))?
                    .used_by;
                if !used_by.contains(&task_type_path) {
                    used_by.push(task_type_path.clone());
                }
            }
            if let Some(operator_id) = task.operator_id()
                && let Some(task_span) = task.active_span()
                && let Ok(operator) = model.operator_mut(operator_id)
            {
                operator.extend_active_span(task_span);
            }

            model.tasks.insert(task_id, task);
        }

        model.resource_group_types = derive_resource_scope_types(&model)?;
        Ok(model)
    }
}

#[cfg(test)]
mod tests {
    use quent_simulator_store::TaskEvent;

    use super::*;

    #[test]
    fn rejects_nil_task_id() {
        let mut builder = SimulatorModelBuilder::try_new(Uuid::from_u128(1)).unwrap();

        assert!(matches!(
            builder.try_push(Event::new(
                Uuid::nil(),
                0,
                SimulatorEvent::Task(TaskEvent::Exit { seq: 0 }),
            )),
            Err(AnalyzerError::Validation(_))
        ));
    }
}
