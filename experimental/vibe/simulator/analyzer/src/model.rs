// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeSet, hash_map::Entry};

use rustc_hash::FxHashMap as HashMap;

use nvtx_analyzer::{NvtxSource, NvtxSourcesBuilder, NvtxSpan};
use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Model, RefTreeEntity,
    context::ContextId,
    fsm::collection::FsmCollection,
    ref_tree::RefTreeCollection,
    resource::{Resource, ResourceTypeDecl, Usage, Using, collection::ResourceCollection},
};
use quent_events::Event;
use quent_query_engine_analyzer::{
    OperatorEntityMut, QueryEngineModel, QueryEngineModelMut, plan_tree::PlanTree,
};
use quent_query_engine_ui::EntityRef;
use quent_simulator_store::{self as schema, SimulatorEvent};
use quent_ui::ResourceGroupTypeDecl;
use uuid::Uuid;

pub use crate::boilerplate::{Engine, Operator, Plan, Port, Query, QueryGroup, Worker};

use crate::{
    boilerplate::{
        Gpu, GpuMemory, HostMemory, Network, NetworkChannel, PcieChannel, QueryBuilder,
        RuntimeProcess, RuntimeThread, Storage, StorageChannel, Task, TaskBuilder, TaskExecutor,
        TaskExecutorThread, TaskExt,
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
    pub(crate) runtime_processes: HashMap<Uuid, RuntimeProcess>,
    pub(crate) runtime_threads: HashMap<Uuid, RuntimeThread>,
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
    nvtx_sources: Vec<NvtxSource>,
}

impl Model for SimulatorModel {
    type EntityIdType = EntityRef;

    fn try_entity_ref(&self, entity_id: Uuid) -> AnalyzerResult<Self::EntityIdType> {
        if self.engine.id() == entity_id {
            Ok(EntityRef::Engine(entity_id))
        } else if self.workers.contains_key(&entity_id) {
            Ok(EntityRef::Worker(entity_id))
        } else if let Some(entity) = self
            .runtime_processes
            .get(&entity_id)
            .map(|entity| entity as &dyn Entity)
            .or_else(|| {
                self.runtime_threads
                    .get(&entity_id)
                    .map(|entity| entity as &dyn Entity)
            })
        {
            Ok(EntityRef::Application {
                type_name: entity.type_name().to_owned(),
                id: entity_id,
            })
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
    /// Independently reconstructed NVTX sources, retaining their context,
    /// process binding, and stream identity.
    pub fn nvtx_sources(&self) -> &[NvtxSource] {
        &self.nvtx_sources
    }

    /// Return the sole NVTX source for one context.
    ///
    /// The existing HTTP surface addresses contexts rather than process/stream
    /// pairs. Refuse an ambiguous context instead of merging source-local NVTX
    /// handles. A future source selector can expose every entry directly.
    pub fn nvtx_model(&self, context_id: Uuid) -> Option<&nvtx_analyzer::NvtxModel> {
        let mut sources = self
            .nvtx_sources
            .iter()
            .filter(|source| source.context_id() == context_id);
        let source = sources.next()?;
        sources.next().is_none().then(|| source.model())
    }

    /// Resolve one per-thread NVTX span to its simulated task-executor slot.
    ///
    /// A match must agree on context, bound runtime process, widened native
    /// thread ID, and the runtime thread's observed lifetime. Ambiguous or
    /// unmatched spans remain available in the NVTX model and return `None`.
    pub fn task_executor_thread_for_nvtx_span(
        &self,
        source: &NvtxSource,
        span: &NvtxSpan,
    ) -> Option<Uuid> {
        let native_id = u64::from(span.kind.thread_id()?);
        let process = self.runtime_processes.get(&source.process_id())?;
        if process.context_id().into_uuid() != source.context_id() || process.native_id().is_none()
        {
            return None;
        }

        let mut matches = self.runtime_threads.values().filter(|runtime_thread| {
            runtime_thread.context_id().into_uuid() == source.context_id()
                && runtime_thread.process_id() == Some(source.process_id())
                && runtime_thread.native_id() == Some(native_id)
                && runtime_thread.contains(span.start)
                && span
                    .end
                    .is_none_or(|timestamp| runtime_thread.contains(timestamp))
        });
        let logical_thread_id = matches.next()?.logical_thread_id()?;
        if matches.next().is_some() || !self.task_executor_threads.contains_key(&logical_thread_id)
        {
            return None;
        }
        Some(logical_thread_id)
    }

    pub(crate) fn query_view(&self, query_id: Uuid) -> AnalyzerResult<SimulatorModelQueryView<'_>> {
        SimulatorModelQueryView::try_new(self, query_id)
    }

    pub(crate) fn resource_instance_name(&self, resource_id: Uuid) -> Option<&str> {
        self.host_memories
            .get(&resource_id)
            .map(HostMemory::instance_name)
            .or_else(|| self.storages.get(&resource_id).map(Storage::instance_name))
            .or_else(|| {
                self.gpu_memories
                    .get(&resource_id)
                    .map(GpuMemory::instance_name)
            })
            .or_else(|| {
                self.task_executor_threads
                    .get(&resource_id)
                    .map(TaskExecutorThread::instance_name)
            })
            .or_else(|| {
                self.storage_channels
                    .get(&resource_id)
                    .map(StorageChannel::instance_name)
            })
            .or_else(|| {
                self.pcie_channels
                    .get(&resource_id)
                    .map(PcieChannel::instance_name)
            })
            .or_else(|| {
                self.network_channels
                    .get(&resource_id)
                    .map(NetworkChannel::instance_name)
            })
    }

    pub(crate) fn resource_scope_instance_name(&self, entity_id: Uuid) -> Option<&str> {
        self.task_executors
            .get(&entity_id)
            .map(TaskExecutor::instance_name)
            .or_else(|| self.networks.get(&entity_id).map(Network::instance_name))
            .or_else(|| self.gpus.get(&entity_id).map(Gpu::instance_name))
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
                self.runtime_processes
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
            .chain(
                self.runtime_threads
                    .values()
                    .map(|entity| entity as &dyn RefTreeEntity),
            )
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
        } else if let Some(entity) = self.runtime_processes.get(&entity_id) {
            Ok(entity)
        } else if let Some(entity) = self.runtime_threads.get(&entity_id) {
            Ok(entity)
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

pub struct SimulatorModelBuilder {
    engine_id: Uuid,
    engine: Option<Engine>,
    runtime_processes: HashMap<Uuid, RuntimeProcess>,
    runtime_threads: HashMap<Uuid, RuntimeThread>,
    nvtx_sources: NvtxSourcesBuilder<schema::NvtxEventEvent>,
    workers: HashMap<Uuid, Worker>,
    query_groups: HashMap<Uuid, QueryGroup>,
    queries: HashMap<Uuid, QueryBuilder>,
    plans: HashMap<Uuid, Plan>,
    operators: HashMap<Uuid, Operator>,
    ports: HashMap<Uuid, Port>,
    host_memories: HashMap<Uuid, HostMemory>,
    storages: HashMap<Uuid, Storage>,
    gpu_memories: HashMap<Uuid, GpuMemory>,
    task_executor_threads: HashMap<Uuid, TaskExecutorThread>,
    storage_channels: HashMap<Uuid, StorageChannel>,
    pcie_channels: HashMap<Uuid, PcieChannel>,
    network_channels: HashMap<Uuid, NetworkChannel>,
    task_executors: HashMap<Uuid, TaskExecutor>,
    networks: HashMap<Uuid, Network>,
    gpus: HashMap<Uuid, Gpu>,
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
            engine: None,
            runtime_processes: HashMap::default(),
            runtime_threads: HashMap::default(),
            nvtx_sources: NvtxSourcesBuilder::new(),
            workers: HashMap::default(),
            query_groups: HashMap::default(),
            queries: HashMap::default(),
            plans: HashMap::default(),
            operators: HashMap::default(),
            ports: HashMap::default(),
            host_memories: HashMap::default(),
            storages: HashMap::default(),
            gpu_memories: HashMap::default(),
            task_executor_threads: HashMap::default(),
            storage_channels: HashMap::default(),
            pcie_channels: HashMap::default(),
            network_channels: HashMap::default(),
            task_executors: HashMap::default(),
            networks: HashMap::default(),
            gpus: HashMap::default(),
            tasks: HashMap::default(),
        })
    }

    pub(crate) fn try_push_from_context(
        &mut self,
        context_id: ContextId,
        event: Event<SimulatorEvent>,
    ) -> AnalyzerResult<()> {
        let Event {
            id,
            timestamp,
            data,
        } = event;
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
                let event = Event::new(id, timestamp, event);
                if let Some(engine) = &mut self.engine {
                    engine.push(event)
                } else {
                    self.engine = Some(Engine::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::RuntimeProcess(event) => {
                let event = Event::new(id, timestamp, event);
                match self.runtime_processes.entry(id) {
                    Entry::Occupied(entry) => entry.into_mut().push(context_id, event),
                    Entry::Vacant(entry) => {
                        entry.insert(RuntimeProcess::try_from_event(context_id, event)?);
                        Ok(())
                    }
                }
            }
            SimulatorEvent::Worker(event) => {
                let event = Event::new(id, timestamp, event);
                match self.workers.entry(id) {
                    Entry::Occupied(entry) => entry.into_mut().push(event),
                    Entry::Vacant(entry) => {
                        entry.insert(Worker::try_from_event(event)?);
                        Ok(())
                    }
                }
            }
            SimulatorEvent::QueryGroup(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(group) = self.query_groups.get_mut(&id) {
                    group.push(event)
                } else {
                    self.query_groups
                        .insert(id, QueryGroup::try_from_event(event)?);
                    Ok(())
                }
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
                let event = Event::new(id, timestamp, event);
                if let Some(plan) = self.plans.get_mut(&id) {
                    plan.push(event)
                } else {
                    self.plans.insert(id, Plan::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::Operator(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(operator) = self.operators.get_mut(&id) {
                    operator.push(event)
                } else {
                    self.operators.insert(id, Operator::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::Port(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(port) = self.ports.get_mut(&id) {
                    port.push(event)
                } else {
                    self.ports.insert(id, Port::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::HostMemory(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(resource) = self.host_memories.get_mut(&id) {
                    resource.push(event)
                } else {
                    self.host_memories
                        .insert(id, HostMemory::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::StorageChannel(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(resource) = self.storage_channels.get_mut(&id) {
                    resource.push(event)
                } else {
                    self.storage_channels
                        .insert(id, StorageChannel::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::Storage(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(resource) = self.storages.get_mut(&id) {
                    resource.push(event)
                } else {
                    self.storages.insert(id, Storage::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::GpuMemory(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(resource) = self.gpu_memories.get_mut(&id) {
                    resource.push(event)
                } else {
                    self.gpu_memories
                        .insert(id, GpuMemory::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::TaskExecutorThread(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(resource) = self.task_executor_threads.get_mut(&id) {
                    resource.push(event)
                } else {
                    self.task_executor_threads
                        .insert(id, TaskExecutorThread::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::RuntimeThread(event) => {
                let event = Event::new(id, timestamp, event);
                match self.runtime_threads.entry(id) {
                    Entry::Occupied(entry) => entry.into_mut().push(context_id, event),
                    Entry::Vacant(entry) => {
                        entry.insert(RuntimeThread::try_from_event(context_id, event)?);
                        Ok(())
                    }
                }
            }
            SimulatorEvent::PcieChannel(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(resource) = self.pcie_channels.get_mut(&id) {
                    resource.push(event)
                } else {
                    self.pcie_channels
                        .insert(id, PcieChannel::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::NetworkChannel(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(resource) = self.network_channels.get_mut(&id) {
                    resource.push(event)
                } else {
                    self.network_channels
                        .insert(id, NetworkChannel::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::TaskExecutor(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(entity) = self.task_executors.get_mut(&id) {
                    entity.push(event)
                } else {
                    self.task_executors
                        .insert(id, TaskExecutor::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::Network(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(entity) = self.networks.get_mut(&id) {
                    entity.push(event)
                } else {
                    self.networks.insert(id, Network::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::Gpu(event) => {
                let event = Event::new(id, timestamp, event);
                if let Some(entity) = self.gpus.get_mut(&id) {
                    entity.push(event)
                } else {
                    self.gpus.insert(id, Gpu::try_from_event(event)?);
                    Ok(())
                }
            }
            SimulatorEvent::NvtxEvent(event) => {
                self.nvtx_sources
                    .push(context_id.into_uuid(), Event::new(id, timestamp, event));
                Ok(())
            }
        }
    }

    pub(crate) fn try_build(self) -> AnalyzerResult<SimulatorModel> {
        let engine = self.engine.ok_or_else(|| {
            AnalyzerError::IncompleteEntity(format!("engine {} has no events", self.engine_id))
        })?;
        let queries = self
            .queries
            .into_iter()
            .map(|(id, builder)| Query::try_from_builder(builder).map(|query| (id, query)))
            .collect::<AnalyzerResult<HashMap<_, _>>>()?;
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

        let nvtx_sources = self
            .nvtx_sources
            .build()
            .map_err(|error| AnalyzerError::Validation(error.to_string()))?;
        let mut model = SimulatorModel {
            engine,
            runtime_processes: self.runtime_processes,
            runtime_threads: self.runtime_threads,
            workers: self.workers,
            query_groups: self.query_groups,
            queries,
            plans: self.plans,
            operators: self.operators,
            ports: self.ports,
            resource_types,
            host_memories: self.host_memories,
            storages: self.storages,
            gpu_memories: self.gpu_memories,
            task_executor_threads: self.task_executor_threads,
            storage_channels: self.storage_channels,
            pcie_channels: self.pcie_channels,
            network_channels: self.network_channels,
            task_executors: self.task_executors,
            networks: self.networks,
            gpus: self.gpus,
            tasks: HashMap::default(),
            resource_group_types: HashMap::default(),
            nvtx_sources,
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
    use nvtx_analyzer::SpanKind;
    use quent_simulator_store::{
        EngineEvent, EngineImplementationAttributes, NvtxEventEvent, RuntimeProcessEvent,
        RuntimeThreadEvent, TaskEvent, TaskExecutorEvent, TaskExecutorThreadEvent, WorkerEvent,
        quent::{nvtx::Attributes, os},
    };

    use super::*;

    #[test]
    fn rejects_nil_task_id() {
        let mut builder = SimulatorModelBuilder::try_new(Uuid::from_u128(1)).unwrap();

        assert!(matches!(
            builder.try_push_from_context(
                ContextId::from(Uuid::nil()),
                Event::new(
                    Uuid::nil(),
                    0,
                    SimulatorEvent::Task(TaskEvent::Exit { seq: 0 }),
                ),
            ),
            Err(AnalyzerError::Validation(_))
        ));
    }

    fn push(
        builder: &mut SimulatorModelBuilder,
        context_id: Uuid,
        id: Uuid,
        timestamp: u64,
        data: SimulatorEvent,
    ) {
        builder
            .try_push_from_context(ContextId::from(context_id), Event::new(id, timestamp, data))
            .unwrap();
    }

    fn attributes() -> Attributes {
        Attributes {
            category: 0,
            color: None,
            message: None,
            payload: None,
        }
    }

    struct RuntimeIds {
        process: Uuid,
        worker: Uuid,
        executor: Uuid,
        logical_thread: Uuid,
        runtime_thread: Uuid,
        stream: Uuid,
    }

    fn emit_runtime(
        builder: &mut SimulatorModelBuilder,
        context_id: Uuid,
        engine_id: Uuid,
        base: u128,
        native_process_id: u32,
        native_thread_id: u64,
        lifetime: std::ops::Range<u64>,
    ) -> RuntimeIds {
        let start = lifetime.start;
        let end = lifetime.end;
        let ids = RuntimeIds {
            process: Uuid::from_u128(base),
            worker: Uuid::from_u128(base + 1),
            executor: Uuid::from_u128(base + 2),
            logical_thread: Uuid::from_u128(base + 3),
            runtime_thread: Uuid::from_u128(base + 4),
            stream: Uuid::from_u128(base + 5),
        };
        push(
            builder,
            context_id,
            ids.process,
            start - 2,
            SimulatorEvent::RuntimeProcess(RuntimeProcessEvent::Started {
                process: os::Process {
                    native_id: native_process_id,
                },
                engine_id: quent_events::EntityRef::new(engine_id, ()),
            }),
        );
        push(
            builder,
            context_id,
            ids.worker,
            start - 2,
            SimulatorEvent::Worker(WorkerEvent::Init {
                parent_engine_id: quent_events::EntityRef::new(engine_id, ()),
                instance_name: "worker".to_owned(),
            }),
        );
        push(
            builder,
            context_id,
            ids.executor,
            start - 2,
            SimulatorEvent::TaskExecutor(TaskExecutorEvent::Declaration {
                instance_name: "executor".to_owned(),
                worker_id: quent_events::EntityRef::new(ids.worker, ()),
            }),
        );
        push(
            builder,
            context_id,
            ids.logical_thread,
            start - 2,
            SimulatorEvent::TaskExecutorThread(TaskExecutorThreadEvent::Declaration {
                instance_name: "logical thread".to_owned(),
                task_executor_id: quent_events::EntityRef::new(ids.executor, ()),
            }),
        );
        push(
            builder,
            context_id,
            ids.runtime_thread,
            start,
            SimulatorEvent::RuntimeThread(RuntimeThreadEvent::Started {
                thread: os::Thread {
                    native_id: native_thread_id,
                },
                process_id: quent_events::EntityRef::new(ids.process, ()),
                logical_thread_id: quent_events::EntityRef::new(ids.logical_thread, ()),
            }),
        );
        push(
            builder,
            context_id,
            ids.runtime_thread,
            end,
            SimulatorEvent::RuntimeThread(RuntimeThreadEvent::Exit),
        );
        push(
            builder,
            context_id,
            ids.stream,
            start,
            SimulatorEvent::NvtxEvent(NvtxEventEvent::Initialized {
                process: quent_events::EntityRef::new(ids.process, ()),
            }),
        );
        push(
            builder,
            context_id,
            ids.stream,
            start + 1,
            SimulatorEvent::NvtxEvent(NvtxEventEvent::RangePush {
                domain: 0,
                thread_id: u32::try_from(native_thread_id).unwrap(),
                attributes: attributes(),
            }),
        );
        push(
            builder,
            context_id,
            ids.stream,
            end - 1,
            SimulatorEvent::NvtxEvent(NvtxEventEvent::RangePop {
                domain: 0,
                thread_id: u32::try_from(native_thread_id).unwrap(),
            }),
        );
        ids
    }

    fn emit_additional_runtime_thread(
        builder: &mut SimulatorModelBuilder,
        context_id: Uuid,
        process_id: Uuid,
        base: u128,
        native_thread_id: u64,
        start: u64,
        end: u64,
    ) -> Uuid {
        let engine_id = builder.engine_id;
        let worker = Uuid::from_u128(base);
        let executor = Uuid::from_u128(base + 1);
        let logical_thread = Uuid::from_u128(base + 2);
        let runtime_thread = Uuid::from_u128(base + 3);
        push(
            builder,
            context_id,
            worker,
            start - 2,
            SimulatorEvent::Worker(WorkerEvent::Init {
                parent_engine_id: quent_events::EntityRef::new(engine_id, ()),
                instance_name: "worker".to_owned(),
            }),
        );
        push(
            builder,
            context_id,
            executor,
            start - 2,
            SimulatorEvent::TaskExecutor(TaskExecutorEvent::Declaration {
                instance_name: "executor".to_owned(),
                worker_id: quent_events::EntityRef::new(worker, ()),
            }),
        );
        push(
            builder,
            context_id,
            logical_thread,
            start - 2,
            SimulatorEvent::TaskExecutorThread(TaskExecutorThreadEvent::Declaration {
                instance_name: "logical thread".to_owned(),
                task_executor_id: quent_events::EntityRef::new(executor, ()),
            }),
        );
        push(
            builder,
            context_id,
            runtime_thread,
            start,
            SimulatorEvent::RuntimeThread(RuntimeThreadEvent::Started {
                thread: os::Thread {
                    native_id: native_thread_id,
                },
                process_id: quent_events::EntityRef::new(process_id, ()),
                logical_thread_id: quent_events::EntityRef::new(logical_thread, ()),
            }),
        );
        push(
            builder,
            context_id,
            runtime_thread,
            end,
            SimulatorEvent::RuntimeThread(RuntimeThreadEvent::Exit),
        );
        logical_thread
    }

    fn push_pop_span(thread_id: u32, start: u64, end: u64) -> NvtxSpan {
        NvtxSpan {
            domain: 0,
            name: "work".to_owned(),
            category: None,
            color: None,
            payload: None,
            start,
            end: Some(end),
            kind: SpanKind::PushPop {
                thread_id,
                parent: None,
            },
        }
    }

    #[test]
    fn correlates_generated_nvtx_with_native_thread_lifetimes_per_context() {
        let engine_id = Uuid::from_u128(1);
        let first_context = Uuid::from_u128(2);
        let second_context = Uuid::from_u128(3);
        let mut builder = SimulatorModelBuilder::try_new(engine_id).unwrap();
        push(
            &mut builder,
            first_context,
            engine_id,
            0,
            SimulatorEvent::Engine(EngineEvent::Init {
                implementation: EngineImplementationAttributes {
                    name: Some("simulator".to_owned()),
                    version: None,
                    custom_attributes: Default::default(),
                },
                instance_name: None,
            }),
        );

        // The two processes and threads deliberately reuse their native IDs.
        // Context and process entity identity keep the source-local streams apart.
        let first = emit_runtime(&mut builder, first_context, engine_id, 10, 42, 7, 10..20);
        let second = emit_runtime(&mut builder, second_context, engine_id, 30, 42, 7, 10..20);

        let model = builder.try_build().unwrap();
        assert_eq!(model.nvtx_sources().len(), 2);
        let correlated: Vec<_> = model
            .nvtx_sources()
            .iter()
            .map(|source| {
                let span = &source.model().spans()[0];
                assert!(matches!(span.kind, SpanKind::PushPop { thread_id: 7, .. }));
                (
                    source.context_id(),
                    model.task_executor_thread_for_nvtx_span(source, span),
                )
            })
            .collect();
        assert_eq!(
            correlated,
            vec![
                (first_context, Some(first.logical_thread)),
                (second_context, Some(second.logical_thread)),
            ]
        );
    }

    #[test]
    fn uses_lifetimes_for_reused_native_thread_ids_and_rejects_ambiguity() {
        let engine_id = Uuid::from_u128(1);
        let context_id = Uuid::from_u128(2);
        let mut builder = SimulatorModelBuilder::try_new(engine_id).unwrap();
        push(
            &mut builder,
            context_id,
            engine_id,
            0,
            SimulatorEvent::Engine(EngineEvent::Init {
                implementation: EngineImplementationAttributes {
                    name: Some("simulator".to_owned()),
                    version: None,
                    custom_attributes: Default::default(),
                },
                instance_name: None,
            }),
        );
        let first = emit_runtime(&mut builder, context_id, engine_id, 10, 42, 7, 10..20);
        let reused =
            emit_additional_runtime_thread(&mut builder, context_id, first.process, 30, 7, 30, 40);

        let model = builder.try_build().unwrap();
        let source = &model.nvtx_sources()[0];
        assert_eq!(
            model.task_executor_thread_for_nvtx_span(source, &push_pop_span(7, 31, 39)),
            Some(reused)
        );
        assert_eq!(
            model.task_executor_thread_for_nvtx_span(source, &push_pop_span(7, 21, 29)),
            None
        );

        let mut builder = SimulatorModelBuilder::try_new(engine_id).unwrap();
        push(
            &mut builder,
            context_id,
            engine_id,
            0,
            SimulatorEvent::Engine(EngineEvent::Init {
                implementation: EngineImplementationAttributes {
                    name: Some("simulator".to_owned()),
                    version: None,
                    custom_attributes: Default::default(),
                },
                instance_name: None,
            }),
        );
        let first = emit_runtime(&mut builder, context_id, engine_id, 50, 42, 7, 10..20);
        emit_additional_runtime_thread(&mut builder, context_id, first.process, 70, 7, 30, 40);
        emit_additional_runtime_thread(&mut builder, context_id, first.process, 90, 7, 32, 38);
        let model = builder.try_build().unwrap();
        assert_eq!(
            model.task_executor_thread_for_nvtx_span(
                &model.nvtx_sources()[0],
                &push_pop_span(7, 33, 37),
            ),
            None
        );
    }
}
