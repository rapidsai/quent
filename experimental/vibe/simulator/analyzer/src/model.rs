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
use quent_events::Event;
use quent_query_engine_analyzer::{
    OperatorEntityMut, QueryEngineModel, QueryEngineModelMut, plan_tree::PlanTree,
};
use quent_query_engine_ui::EntityRef;
use quent_simulator_store::SimulatorEvent;
use quent_simulator_store::entity_events::{
    gpu::GpuEvents, gpu_memory::GpuMemoryEvents, host_memory::HostMemoryEvents,
    network::NetworkEvents, network_channel::NetworkChannelEvents, pcie_channel::PcieChannelEvents,
    storage::StorageEvents, storage_channel::StorageChannelEvents,
    task_executor::TaskExecutorEvents, task_executor_thread::TaskExecutorThreadEvents,
};
use quent_ui::ResourceGroupTypeDecl;
use uuid::Uuid;

pub use crate::boilerplate::{Engine, Operator, Plan, Port, Query, QueryGroup, Worker};

use crate::{
    boilerplate::{
        Gpu, GpuMemory, HostMemory, Network, NetworkChannel, PcieChannel, QueryBuilder, Storage,
        StorageChannel, Task, TaskBuilder, TaskExecutor, TaskExecutorThread, TaskExt,
    },
    view::SimulatorModelQueryView,
};

fn derive_resource_scope_types(
    model: &SimulatorModel,
) -> AnalyzerResult<HashMap<String, ResourceGroupTypeDecl>> {
    fn populate(
        node: &quent_analyzer::resource::tree::ResourceTreeNode,
        model: &SimulatorModel,
        declarations: &mut HashMap<String, (BTreeSet<String>, BTreeSet<String>)>,
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
                    used_by.extend(resource_type.used_by.iter().cloned());
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
            (
                name.clone(),
                ResourceGroupTypeDecl {
                    name,
                    used_by_entity_types: used_by_entity_types.into_iter().collect(),
                    contains_resource_types: contains_resource_types.into_iter().collect(),
                },
            )
        })
        .collect())
}

/// A model of the simulator engine
pub struct SimulatorModel {
    pub(crate) engine: Engine,
    pub(crate) workers: HashMap<Uuid, Worker>,
    pub(crate) query_groups: HashMap<Uuid, QueryGroup>,
    pub(crate) queries: HashMap<Uuid, Query>,
    pub(crate) plans: HashMap<Uuid, Plan>,
    pub(crate) operators: HashMap<Uuid, Operator>,
    pub(crate) ports: HashMap<Uuid, Port>,
    pub(crate) resource_types: HashMap<String, ResourceTypeDecl>,
    pub(crate) host_memories: HashMap<Uuid, HostMemory>,
    pub(crate) storages: HashMap<Uuid, Storage>,
    pub(crate) gpu_memories: HashMap<Uuid, GpuMemory>,
    pub(crate) task_executor_threads: HashMap<Uuid, TaskExecutorThread>,
    pub(crate) storage_channels: HashMap<Uuid, StorageChannel>,
    pub(crate) pcie_channels: HashMap<Uuid, PcieChannel>,
    pub(crate) network_channels: HashMap<Uuid, NetworkChannel>,
    pub(crate) task_executors: HashMap<Uuid, TaskExecutor>,
    pub(crate) networks: HashMap<Uuid, Network>,
    pub(crate) gpus: HashMap<Uuid, Gpu>,
    pub(crate) tasks: HashMap<Uuid, Task>,
    pub(crate) resource_group_types: HashMap<String, ResourceGroupTypeDecl>,
}

impl Model for SimulatorModel {
    type EntityIdType = EntityRef;

    fn try_entity_ref(&self, entity_id: Uuid) -> AnalyzerResult<Self::EntityIdType> {
        if self.engine.id() == entity_id {
            Ok(EntityRef::Engine(entity_id))
        } else if self.workers.contains_key(&entity_id) {
            Ok(EntityRef::Worker(entity_id))
        } else if self.query_groups.contains_key(&entity_id) {
            Ok(EntityRef::QueryGroup(entity_id))
        } else if self.queries.contains_key(&entity_id) {
            Ok(EntityRef::Query(entity_id))
        } else if self.plans.contains_key(&entity_id) {
            Ok(EntityRef::Plan(entity_id))
        } else if self.operators.contains_key(&entity_id) {
            Ok(EntityRef::Operator(entity_id))
        } else if self.ports.contains_key(&entity_id) {
            Ok(EntityRef::Port(entity_id))
        } else if self.resource(entity_id).is_ok() {
            Ok(EntityRef::Resource(entity_id))
        } else if self.task_executors.contains_key(&entity_id)
            || self.networks.contains_key(&entity_id)
            || self.gpus.contains_key(&entity_id)
        {
            Ok(EntityRef::ResourceGroup(entity_id))
        } else {
            self.tasks
                .get(&entity_id)
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
    type QueryGroup = QueryGroup;
    type Worker = Worker;
    type Plan = Plan;
    type Operator = Operator;
    type Port = Port;

    fn engine(&self) -> AnalyzerResult<&Engine> {
        Ok(&self.engine)
    }
    fn query(&self, query_id: Uuid) -> AnalyzerResult<&Query> {
        self.queries
            .get(&query_id)
            .ok_or(AnalyzerError::InvalidId(query_id))
    }
    fn query_group(&self, query_group_id: Uuid) -> AnalyzerResult<&QueryGroup> {
        self.query_groups
            .get(&query_group_id)
            .ok_or(AnalyzerError::InvalidId(query_group_id))
    }
    fn worker(&self, worker_id: Uuid) -> AnalyzerResult<&Worker> {
        self.workers
            .get(&worker_id)
            .ok_or(AnalyzerError::InvalidId(worker_id))
    }
    fn plan(&self, plan_id: Uuid) -> AnalyzerResult<&Plan> {
        self.plans
            .get(&plan_id)
            .ok_or(AnalyzerError::InvalidId(plan_id))
    }
    fn operator(&self, operator_id: Uuid) -> AnalyzerResult<&Operator> {
        self.operators
            .get(&operator_id)
            .ok_or(AnalyzerError::InvalidId(operator_id))
    }
    fn port(&self, port_id: Uuid) -> AnalyzerResult<&Port> {
        self.ports
            .get(&port_id)
            .ok_or(AnalyzerError::InvalidId(port_id))
    }
    fn queries(&self) -> impl Iterator<Item = &Query> {
        self.queries.values()
    }
    fn query_groups(&self) -> impl Iterator<Item = &QueryGroup> {
        self.query_groups.values()
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
            .get_mut(&operator_id)
            .ok_or(AnalyzerError::InvalidId(operator_id))
    }
}

impl SimulatorModel {
    pub(crate) fn query_view(&self, query_id: Uuid) -> AnalyzerResult<SimulatorModelQueryView<'_>> {
        SimulatorModelQueryView::try_new(self, query_id)
    }

    pub(crate) fn resource_instance_name(&self, resource_id: Uuid) -> Option<&str> {
        self.host_memories
            .get(&resource_id)
            .map(|entity| entity.declaration().data.instance_name.as_str())
            .or_else(|| {
                self.storages
                    .get(&resource_id)
                    .map(|entity| entity.declaration().data.instance_name.as_str())
            })
            .or_else(|| {
                self.gpu_memories
                    .get(&resource_id)
                    .map(|entity| entity.declaration().data.instance_name.as_str())
            })
            .or_else(|| {
                self.task_executor_threads
                    .get(&resource_id)
                    .map(|entity| entity.declaration().data.instance_name.as_str())
            })
            .or_else(|| {
                self.storage_channels
                    .get(&resource_id)
                    .map(|entity| entity.declaration().data.instance_name.as_str())
            })
            .or_else(|| {
                self.pcie_channels
                    .get(&resource_id)
                    .map(|entity| entity.declaration().data.instance_name.as_str())
            })
            .or_else(|| {
                self.network_channels
                    .get(&resource_id)
                    .map(|entity| entity.declaration().data.instance_name.as_str())
            })
    }

    pub(crate) fn resource_scope_instance_name(&self, entity_id: Uuid) -> Option<&str> {
        self.task_executors
            .get(&entity_id)
            .map(|entity| entity.declaration().data.instance_name.as_str())
            .or_else(|| {
                self.networks
                    .get(&entity_id)
                    .map(|entity| entity.declaration().data.instance_name.as_str())
            })
            .or_else(|| {
                self.gpus
                    .get(&entity_id)
                    .map(|entity| entity.declaration().data.instance_name.as_str())
            })
    }

    fn simulator_resource(&self, resource_id: Uuid) -> Option<&dyn Resource> {
        self.host_memories
            .get(&resource_id)
            .map(|resource| resource as &dyn Resource)
            .or_else(|| {
                self.storages
                    .get(&resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
            .or_else(|| {
                self.gpu_memories
                    .get(&resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
            .or_else(|| {
                self.task_executor_threads
                    .get(&resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
            .or_else(|| {
                self.storage_channels
                    .get(&resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
            .or_else(|| {
                self.pcie_channels
                    .get(&resource_id)
                    .map(|resource| resource as &dyn Resource)
            })
            .or_else(|| {
                self.network_channels
                    .get(&resource_id)
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
        std::iter::once(&self.engine as &dyn RefTreeEntity)
            .chain(
                self.workers
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.query_groups
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
        if self.engine.id() == entity_id {
            Ok(&self.engine)
        } else if let Some(entity) = self.workers.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.query_groups.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.queries.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.plans.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.operators.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.ports.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.tasks.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.task_executors.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.networks.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.gpus.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.host_memories.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.storages.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.gpu_memories.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.task_executor_threads.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.storage_channels.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.pcie_channels.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.network_channels.get(&entity_id) {
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

fn build_entities<E: quent_events::EntityMarker, T>(
    events: Vec<Event<E::Payload>>,
    build: impl Fn(quent_store::entity::sequence::EventSequence<E>) -> AnalyzerResult<T>,
) -> AnalyzerResult<HashMap<Uuid, T>> {
    quent_store::entity::native::Store::<E>::new(events)
        .into_sequences()
        .map(|sequence| {
            let id = sequence.id();
            build(sequence).map(|entity| (id, entity))
        })
        .collect()
}

pub struct SimulatorModelBuilder {
    engine_id: Uuid,
    engine: Vec<Event<quent_simulator_store::EngineEvent>>,
    workers: Vec<Event<quent_simulator_store::WorkerEvent>>,
    query_groups: Vec<Event<quent_simulator_store::QueryGroupEvent>>,
    queries: HashMap<Uuid, QueryBuilder>,
    plans: Vec<Event<quent_simulator_store::PlanEvent>>,
    operators: Vec<Event<quent_simulator_store::OperatorEvent>>,
    ports: Vec<Event<quent_simulator_store::PortEvent>>,
    host_memories: Vec<Event<quent_simulator_store::HostMemoryEvent>>,
    storages: Vec<Event<quent_simulator_store::StorageEvent>>,
    gpu_memories: Vec<Event<quent_simulator_store::GpuMemoryEvent>>,
    task_executor_threads: Vec<Event<quent_simulator_store::TaskExecutorThreadEvent>>,
    storage_channels: Vec<Event<quent_simulator_store::StorageChannelEvent>>,
    pcie_channels: Vec<Event<quent_simulator_store::PcieChannelEvent>>,
    network_channels: Vec<Event<quent_simulator_store::NetworkChannelEvent>>,
    task_executors: Vec<Event<quent_simulator_store::TaskExecutorEvent>>,
    networks: Vec<Event<quent_simulator_store::NetworkEvent>>,
    gpus: Vec<Event<quent_simulator_store::GpuEvent>>,
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
            engine: Vec::new(),
            workers: Vec::new(),
            query_groups: Vec::new(),
            queries: HashMap::default(),
            plans: Vec::new(),
            operators: Vec::new(),
            ports: Vec::new(),
            host_memories: Vec::new(),
            storages: Vec::new(),
            gpu_memories: Vec::new(),
            task_executor_threads: Vec::new(),
            storage_channels: Vec::new(),
            pcie_channels: Vec::new(),
            network_channels: Vec::new(),
            task_executors: Vec::new(),
            networks: Vec::new(),
            gpus: Vec::new(),
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
            SimulatorEvent::Task(t) => {
                let task_builder = match self.tasks.entry(id) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => entry.insert(TaskBuilder::try_new(id)?),
                };
                task_builder.push_transition(Event::new(id, timestamp, t));
                Ok(())
            }
            SimulatorEvent::Engine(event) => {
                if id != self.engine_id {
                    return Err(AnalyzerError::Validation(format!(
                        "multiple engine instances in one model: expected {}, found {id}",
                        self.engine_id
                    )));
                }
                self.engine.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::Worker(event) => {
                self.workers.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::QueryGroup(event) => {
                self.query_groups.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::Query(event) => {
                match self.queries.entry(id) {
                    std::collections::hash_map::Entry::Occupied(entry) => {
                        entry
                            .into_mut()
                            .push_transition(Event::new(id, timestamp, event));
                    }
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        let mut builder = QueryBuilder::try_new(id)?;
                        builder.push_transition(Event::new(id, timestamp, event));
                        entry.insert(builder);
                    }
                }
                Ok(())
            }
            SimulatorEvent::Plan(event) => {
                self.plans.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::Operator(event) => {
                self.operators.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::Port(event) => {
                self.ports.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::HostMemory(event) => {
                self.host_memories.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::StorageChannel(event) => {
                self.storage_channels.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::Storage(event) => {
                self.storages.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::GpuMemory(event) => {
                self.gpu_memories.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::TaskExecutorThread(event) => {
                self.task_executor_threads
                    .push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::PcieChannel(event) => {
                self.pcie_channels.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::NetworkChannel(event) => {
                self.network_channels.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::TaskExecutor(event) => {
                self.task_executors.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::Network(event) => {
                self.networks.push(Event::new(id, timestamp, event));
                Ok(())
            }
            SimulatorEvent::Gpu(event) => {
                self.gpus.push(Event::new(id, timestamp, event));
                Ok(())
            }
        }
    }

    pub(crate) fn try_build(self) -> AnalyzerResult<SimulatorModel> {
        let engine = build_entities::<quent_simulator_store::Engine, _>(
            self.engine,
            Engine::try_from_sequence,
        )?
        .remove(&self.engine_id)
        .ok_or_else(|| {
            AnalyzerError::IncompleteEntity(format!("engine {} has no events", self.engine_id))
        })?;
        let queries = self
            .queries
            .into_iter()
            .map(|(id, builder)| Query::try_from_builder(builder).map(|query| (id, query)))
            .collect::<AnalyzerResult<HashMap<_, _>>>()?;
        let workers = build_entities::<quent_simulator_store::Worker, _>(
            self.workers,
            Worker::try_from_sequence,
        )?;
        let query_groups = build_entities::<quent_simulator_store::QueryGroup, _>(
            self.query_groups,
            QueryGroup::try_from_sequence,
        )?;
        let plans =
            build_entities::<quent_simulator_store::Plan, _>(self.plans, Plan::try_from_sequence)?;
        let operators = build_entities::<quent_simulator_store::Operator, _>(
            self.operators,
            Operator::try_from_sequence,
        )?;
        let ports =
            build_entities::<quent_simulator_store::Port, _>(self.ports, Port::try_from_sequence)?;
        let host_memories = build_entities::<quent_simulator_store::HostMemory, _>(
            self.host_memories,
            HostMemory::try_from_sequence,
        )?;
        let storages = build_entities::<quent_simulator_store::Storage, _>(
            self.storages,
            Storage::try_from_sequence,
        )?;
        let gpu_memories = build_entities::<quent_simulator_store::GpuMemory, _>(
            self.gpu_memories,
            GpuMemory::try_from_sequence,
        )?;
        let task_executor_threads = build_entities::<quent_simulator_store::TaskExecutorThread, _>(
            self.task_executor_threads,
            TaskExecutorThread::try_from_sequence,
        )?;
        let storage_channels = build_entities::<quent_simulator_store::StorageChannel, _>(
            self.storage_channels,
            StorageChannel::try_from_sequence,
        )?;
        let pcie_channels = build_entities::<quent_simulator_store::PcieChannel, _>(
            self.pcie_channels,
            PcieChannel::try_from_sequence,
        )?;
        let network_channels = build_entities::<quent_simulator_store::NetworkChannel, _>(
            self.network_channels,
            NetworkChannel::try_from_sequence,
        )?;
        let task_executors = build_entities::<quent_simulator_store::TaskExecutor, _>(
            self.task_executors,
            TaskExecutor::try_from_sequence,
        )?;
        let networks = build_entities::<quent_simulator_store::Network, _>(
            self.networks,
            Network::try_from_sequence,
        )?;
        let gpus =
            build_entities::<quent_simulator_store::Gpu, _>(self.gpus, Gpu::try_from_sequence)?;
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
            engine,
            workers,
            query_groups,
            queries,
            plans,
            operators,
            ports,
            resource_types,
            host_memories,
            storages,
            gpu_memories,
            task_executor_threads,
            storage_channels,
            pcie_channels,
            network_channels,
            task_executors,
            networks,
            gpus,
            tasks: HashMap::default(),
            resource_group_types: HashMap::default(),
        };

        for (task_id, task_builder) in self.tasks.into_iter() {
            let task = Task::from_builder(task_builder)?;
            for usage in task.usages() {
                let resource_type_name = model
                    .resource(usage.resource_id())
                    .map(Entity::type_name)?
                    .to_owned();
                let set = &mut model
                    .resource_types
                    .get_mut(&resource_type_name)
                    .ok_or_else(|| AnalyzerError::InvalidTypeName(resource_type_name.clone()))?
                    .used_by;
                if !set.contains(task.type_name()) {
                    set.insert(task.type_name().to_owned());
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
    use quent_query_engine_analyzer::{QueryEntity, WorkerEntity};
    use quent_simulator_store::{EngineEvent, TaskEvent, WorkerEvent};

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

    fn builder_with_engine() -> SimulatorModelBuilder {
        let mut builder = SimulatorModelBuilder::try_new(Uuid::from_u128(1)).unwrap();
        builder
            .try_push(Event::new(
                Uuid::from_u128(1),
                0,
                SimulatorEvent::Engine(EngineEvent::Exit),
            ))
            .unwrap();
        builder
    }

    fn worker_init() -> WorkerEvent {
        WorkerEvent::Init {
            parent_engine_id: quent_events::EntityRef::new(Uuid::from_u128(1), ()),
            instance_name: "worker".to_owned(),
        }
    }

    #[test]
    fn groups_worker_events_and_preserves_ui_timestamp_bounds() {
        let mut builder = builder_with_engine();
        let first = Uuid::from_u128(2);
        let second = Uuid::from_u128(3);
        for (id, timestamp, event) in [
            (first, 10, WorkerEvent::Exit),
            (second, 30, worker_init()),
            (first, 20, worker_init()),
        ] {
            builder
                .try_push(Event::new(id, timestamp, SimulatorEvent::Worker(event)))
                .unwrap();
        }
        let model = builder.try_build().unwrap();
        assert_eq!(model.workers.len(), 2);
        let first_ui = model.worker(first).unwrap().to_ui(0);
        assert_eq!(first_ui.id, first);
        assert_eq!(first_ui.parent_engine_id, Some(Uuid::from_u128(1)));
        assert_eq!(first_ui.instance_name.as_deref(), Some("worker"));
        assert_eq!(first_ui.start_unix_ns, Some(10));
        assert_eq!(first_ui.end_unix_ns, Some(20));
        let second_ui = model.worker(second).unwrap().to_ui(0);
        assert_eq!(second_ui.start_unix_ns, Some(30));
        assert_eq!(second_ui.end_unix_ns, None);
    }

    #[test]
    fn rejects_duplicate_once_worker_events_when_building() {
        for event_name in ["init", "exit"] {
            let mut builder = builder_with_engine();
            let id = Uuid::from_u128(2);
            for timestamp in [20, 10] {
                let event = if event_name == "init" {
                    worker_init()
                } else {
                    WorkerEvent::Exit
                };
                builder
                    .try_push(Event::new(id, timestamp, SimulatorEvent::Worker(event)))
                    .unwrap();
            }
            assert!(
                matches!(builder.try_build(), Err(AnalyzerError::Validation(message))
                if message.contains(event_name) && message.contains(&id.to_string()))
            );
        }
    }

    #[test]
    fn rejects_nil_worker_id_before_buffering() {
        let mut builder = builder_with_engine();
        assert!(matches!(
            builder.try_push(Event::new(
                Uuid::nil(),
                0,
                SimulatorEvent::Worker(WorkerEvent::Exit)
            )),
            Err(AnalyzerError::Validation(_))
        ));
        assert!(builder.try_build().unwrap().workers.is_empty());
    }
    #[test]
    fn exit_without_init_retains_partial_worker_ui() {
        let id = Uuid::from_u128(2);
        let sequence =
            quent_store::entity::native::Store::<quent_simulator_store::Worker>::new([Event::new(
                id,
                10,
                quent_simulator_store::WorkerEvent::Exit,
            )])
            .into_sequences()
            .next()
            .unwrap();
        let worker = Worker::try_from_sequence(sequence).unwrap();
        let ui = worker.to_ui(0);
        assert_eq!(ui.id, id);
        assert_eq!(ui.parent_engine_id, None);
        assert_eq!(ui.instance_name, None);
        assert_eq!(ui.start_unix_ns, Some(10));
        assert_eq!(ui.end_unix_ns, Some(10));
        assert_eq!(worker.type_name(), "Worker");
    }

    #[test]
    fn query_ui_retains_metadata_and_phase_times() {
        use quent_simulator_store::QueryEvent;

        let id = Uuid::from_u128(4);
        let group_id = Uuid::from_u128(3);
        let mut builder = QueryBuilder::try_new(id).unwrap();
        for (timestamp, event) in [
            (4_000_000_000, QueryEvent::Done { seq: 3 }),
            (3_000_000_000, QueryEvent::Executing { seq: 2 }),
            (
                1_000_000_000,
                QueryEvent::Init {
                    seq: 0,
                    instance_name: "query".to_owned(),
                    query_group_id: quent_events::EntityRef::new(group_id, ()),
                },
            ),
            (2_000_000_000, QueryEvent::Planning { seq: 1 }),
        ] {
            builder.push_transition(Event::new(id, timestamp, event));
        }
        let query = Query::try_from_builder(builder).unwrap();
        let ui = query.to_ui().unwrap();
        assert_eq!(query.parent_id(), Some(group_id));
        assert_eq!(ui.id, id);
        assert_eq!(ui.query_group_id, group_id);
        assert_eq!(ui.instance_name.as_deref(), Some("query"));
        assert_eq!(ui.start_unix_ns, Some(1_000_000_000));
        assert_eq!(ui.planning_s, Some(1.0));
        assert_eq!(ui.executing_s, Some(2.0));
        assert_eq!(ui.completed_s, Some(3.0));
    }
}
