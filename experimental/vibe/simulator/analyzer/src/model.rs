// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use rustc_hash::FxHashMap as HashMap;

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Model,
    fsm::collection::FsmCollection,
    resource::{
        CapacityDecl, Resource, ResourceCapacities, ResourceGroup, ResourceGroupTypeDecl,
        ResourceTypeDecl, Usage, Using,
        collection::{
            InMemoryResources, InMemoryResourcesBuilder, ResourceCollection,
            derive_resource_group_types,
        },
        runtime::RtResourceTransition,
    },
};
use quent_events::Event;
use quent_query_engine_analyzer::{
    OperatorEntityMut, QueryEngineEntityId, QueryEngineModel, QueryEngineModelMut,
    plan_tree::PlanTree,
};
use quent_query_engine_ui::EntityRef;
use quent_simulator_store::{self as schema, SimulatorEvent};
use uuid::Uuid;

use crate::{
    query_engine::{
        Engine, Operator, Plan, Port, Query, QueryEngine, QueryEngineBuilder, QueryGroup, Worker,
    },
    task::{Task, TaskBuilder, TaskExt},
    view::SimulatorModelQueryView,
};

/// A model of the simulator engine
pub struct SimulatorModel {
    pub(crate) query_engine: QueryEngine,
    pub(crate) arbitrary_resources: InMemoryResources,
    pub(crate) tasks: HashMap<Uuid, Task>,
    pub(crate) resource_group_types: HashMap<String, ResourceGroupTypeDecl>,
}

impl Model for SimulatorModel {
    type EntityIdType = EntityRef;

    fn try_entity_ref(&self, entity_id: Uuid) -> AnalyzerResult<Self::EntityIdType> {
        if let Ok(qe_ref) = self.query_engine.try_entity_ref(entity_id) {
            Ok(match qe_ref {
                QueryEngineEntityId::Engine(uuid) => EntityRef::Engine(uuid),
                QueryEngineEntityId::Worker(uuid) => EntityRef::Worker(uuid),
                QueryEngineEntityId::QueryGroup(uuid) => EntityRef::QueryGroup(uuid),
                QueryEngineEntityId::Query(uuid) => EntityRef::Query(uuid),
                QueryEngineEntityId::Plan(uuid) => EntityRef::Plan(uuid),
                QueryEngineEntityId::Operator(uuid) => EntityRef::Operator(uuid),
                QueryEngineEntityId::Port(uuid) => EntityRef::Port(uuid),
            })
        } else if self.arbitrary_resources.resources.contains_key(&entity_id) {
            Ok(EntityRef::Resource(entity_id))
        } else if self
            .arbitrary_resources
            .resource_groups
            .contains_key(&entity_id)
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

    fn root(&self) -> AnalyzerResult<&impl ResourceGroup> {
        self.query_engine.root()
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
        self.query_engine.engine()
    }
    fn query(&self, query_id: Uuid) -> AnalyzerResult<&Query> {
        self.query_engine.query(query_id)
    }
    fn query_group(&self, query_group_id: Uuid) -> AnalyzerResult<&QueryGroup> {
        self.query_engine.query_group(query_group_id)
    }
    fn worker(&self, worker_id: Uuid) -> AnalyzerResult<&Worker> {
        self.query_engine.worker(worker_id)
    }
    fn plan(&self, plan_id: Uuid) -> AnalyzerResult<&Plan> {
        self.query_engine.plan(plan_id)
    }
    fn operator(&self, operator_id: Uuid) -> AnalyzerResult<&Operator> {
        self.query_engine.operator(operator_id)
    }
    fn port(&self, port_id: Uuid) -> AnalyzerResult<&Port> {
        self.query_engine.port(port_id)
    }
    fn queries(&self) -> impl Iterator<Item = &Query> {
        self.query_engine.queries()
    }
    fn query_groups(&self) -> impl Iterator<Item = &QueryGroup> {
        self.query_engine.query_groups()
    }
    fn workers(&self) -> impl Iterator<Item = &Worker> {
        self.query_engine.workers()
    }
    fn plans(&self) -> impl Iterator<Item = &Plan> {
        self.query_engine.plans()
    }
    fn operators(&self) -> impl Iterator<Item = &Operator> {
        self.query_engine.operators()
    }
    fn ports(&self) -> impl Iterator<Item = &Port> {
        self.query_engine.ports()
    }
    fn plan_tree(&self, query_id: Uuid) -> AnalyzerResult<PlanTree> {
        self.query_engine.plan_tree(query_id)
    }
}

impl QueryEngineModelMut for SimulatorModel {
    fn operator_mut(&mut self, operator_id: Uuid) -> AnalyzerResult<&mut Operator> {
        self.query_engine.operator_mut(operator_id)
    }
}

impl SimulatorModel {
    pub(crate) fn query_view(&self, query_id: Uuid) -> AnalyzerResult<SimulatorModelQueryView<'_>> {
        SimulatorModelQueryView::try_new(self, query_id)
    }
}

impl FsmCollection for SimulatorModel {
    type Fsm = Task;

    fn fsms(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }
}

impl ResourceCollection for SimulatorModel {
    fn resources(&self) -> impl Iterator<Item = &dyn Resource> {
        self.arbitrary_resources
            .resources()
            .chain(self.query_engine.resources())
    }
    fn resource_groups(&self) -> impl Iterator<Item = &dyn ResourceGroup> {
        self.arbitrary_resources
            .resource_groups()
            .chain(self.query_engine.resource_groups())
    }
    fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource> {
        self.arbitrary_resources
            .resource(resource_id)
            .or_else(|_| self.query_engine.resource(resource_id))
    }
    fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl> {
        self.query_engine
            .resource_type(resource_type_name)
            .or_else(|_| self.arbitrary_resources.resource_type(resource_type_name))
    }
    fn resource_group(&self, resource_group_id: Uuid) -> AnalyzerResult<&dyn ResourceGroup> {
        self.query_engine
            .resource_group(resource_group_id)
            .or_else(|_| self.arbitrary_resources.resource_group(resource_group_id))
    }

    fn resource_group_child_groups(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // Verify the resource group exists in at least one collection
        self.resource_group(resource_group_id)?;

        let engine = self
            .query_engine
            .resource_group_child_groups(resource_group_id)
            .ok();

        let sim = self
            .arbitrary_resources
            .resource_groups
            .values()
            .filter_map(move |group| {
                group
                    .parent_group_id
                    .and_then(|parent| (parent == resource_group_id).then_some(group.id))
            });

        Ok(engine.into_iter().flatten().chain(sim))
    }

    fn resource_group_child_resources(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // Verify the resource group exists in at least one collection
        self.resource_group(resource_group_id)?;

        let engine = self
            .query_engine
            .resource_group_child_resources(resource_group_id)
            .ok();

        let sim = self
            .arbitrary_resources
            .resources
            .values()
            .filter_map(move |resource| {
                (resource.parent_group_id() == resource_group_id).then_some(resource.id)
            });

        Ok(engine.into_iter().flatten().chain(sim))
    }
}

impl Using for SimulatorModel {
    fn usages(&self) -> impl Iterator<Item = impl Usage<'_>> {
        self.tasks.values().flat_map(|task| task.usages())
    }
}

pub struct SimulatorModelBuilder {
    query_engine: QueryEngineBuilder,
    arbitrary_resources: InMemoryResourcesBuilder,
    tasks: HashMap<Uuid, TaskBuilder>,
}

impl SimulatorModelBuilder {
    pub(crate) fn try_new(engine_id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self {
            query_engine: QueryEngineBuilder::try_new(engine_id)?,
            arbitrary_resources: InMemoryResourcesBuilder::default(),
            tasks: HashMap::default(),
        })
    }

    pub(crate) fn try_push(&mut self, event: Event<SimulatorEvent>) -> AnalyzerResult<()> {
        let Event {
            id,
            timestamp,
            data,
        } = event;
        match data {
            SimulatorEvent::Task(t) => {
                let task_builder = self
                    .tasks
                    .entry(id)
                    .or_insert_with(|| TaskBuilder::try_new(id).unwrap());
                task_builder.push_transition(Event::new(id, timestamp, t));
                Ok(())
            }
            SimulatorEvent::Engine(event) => self
                .query_engine
                .push_engine(Event::new(id, timestamp, event)),
            SimulatorEvent::Worker(event) => self
                .query_engine
                .push_worker(Event::new(id, timestamp, event)),
            SimulatorEvent::QueryGroup(event) => self
                .query_engine
                .push_query_group(Event::new(id, timestamp, event)),
            SimulatorEvent::Query(event) => self
                .query_engine
                .push_query(Event::new(id, timestamp, event)),
            SimulatorEvent::Plan(event) => self
                .query_engine
                .push_plan(Event::new(id, timestamp, event)),
            SimulatorEvent::Operator(event) => self
                .query_engine
                .push_operator(Event::new(id, timestamp, event)),
            SimulatorEvent::Port(event) => self
                .query_engine
                .push_port(Event::new(id, timestamp, event)),
            SimulatorEvent::HostMemory(event) => self.push_host_memory(id, timestamp, event),
            SimulatorEvent::Storage(event) => self.push_storage(id, timestamp, event),
            SimulatorEvent::GpuMemory(event) => self.push_gpu_memory(id, timestamp, event),
            SimulatorEvent::TaskExecutorThread(event) => self.push_thread(id, timestamp, event),
            SimulatorEvent::StorageChannel(event) => {
                self.push_storage_channel(id, timestamp, event)
            }
            SimulatorEvent::PcieChannel(event) => self.push_pcie_channel(id, timestamp, event),
            SimulatorEvent::NetworkChannel(event) => {
                self.push_network_channel(id, timestamp, event)
            }
            SimulatorEvent::TaskExecutor(schema::TaskExecutorEvent::Declaration {
                instance_name,
                worker_id,
            }) => {
                self.arbitrary_resources.push_group_raw(
                    id,
                    "task_executor",
                    &instance_name,
                    Some(worker_id.target),
                );
                Ok(())
            }
            SimulatorEvent::Network(schema::NetworkEvent::Declaration {
                instance_name,
                engine_id,
            }) => {
                self.arbitrary_resources.push_group_raw(
                    id,
                    "network",
                    &instance_name,
                    Some(engine_id.target),
                );
                Ok(())
            }
            SimulatorEvent::Gpu(schema::GpuEvent::Declaration {
                instance_name,
                worker_id,
            }) => {
                self.arbitrary_resources.push_group_raw(
                    id,
                    "gpu",
                    &instance_name,
                    Some(worker_id.target),
                );
                Ok(())
            }
        }
    }

    fn initialize_resource(
        &mut self,
        id: Uuid,
        timestamp: quent_time::TimeUnixNanoSec,
        type_name: &str,
        instance_name: String,
        parent_id: Uuid,
        declaration: ResourceTypeDecl,
    ) -> AnalyzerResult<()> {
        self.arbitrary_resources.insert_resource_type(declaration);
        let builder = self.arbitrary_resources.try_builder(id)?;
        builder.push(RtResourceTransition::Init(timestamp));
        builder.set_type_name(type_name);
        builder.set_instance_name(Some(instance_name));
        builder.set_parent_group_id(parent_id);
        Ok(())
    }

    fn push_resource_state(
        &mut self,
        id: Uuid,
        transition: RtResourceTransition,
    ) -> AnalyzerResult<()> {
        self.arbitrary_resources.try_builder(id)?.push(transition);
        Ok(())
    }

    fn push_host_memory(
        &mut self,
        id: Uuid,
        timestamp: quent_time::TimeUnixNanoSec,
        event: schema::HostMemoryEvent,
    ) -> AnalyzerResult<()> {
        match event {
            schema::HostMemoryEvent::Initializing {
                instance_name,
                worker_id,
                ..
            } => self.initialize_resource(
                id,
                timestamp,
                "host_memory",
                instance_name,
                worker_id.target,
                ResourceTypeDecl::new("host_memory", [CapacityDecl::new_occupancy("bytes")]),
            ),
            schema::HostMemoryEvent::Operating { .. } => self.push_resource_state(
                id,
                RtResourceTransition::Operating(timestamp, ResourceCapacities(Vec::new())),
            ),
            schema::HostMemoryEvent::Finalizing { .. } => {
                self.push_resource_state(id, RtResourceTransition::Finalizing(timestamp))
            }
            schema::HostMemoryEvent::Exit { .. } => {
                self.push_resource_state(id, RtResourceTransition::Exit(timestamp))
            }
        }
    }

    fn push_storage(
        &mut self,
        id: Uuid,
        timestamp: quent_time::TimeUnixNanoSec,
        event: schema::StorageEvent,
    ) -> AnalyzerResult<()> {
        match event {
            schema::StorageEvent::Initializing {
                instance_name,
                worker_id,
                ..
            } => self.initialize_resource(
                id,
                timestamp,
                "storage",
                instance_name,
                worker_id.target,
                ResourceTypeDecl::new("storage", [CapacityDecl::new_occupancy("bytes")]),
            ),
            schema::StorageEvent::Operating { .. } => self.push_resource_state(
                id,
                RtResourceTransition::Operating(timestamp, ResourceCapacities(Vec::new())),
            ),
            schema::StorageEvent::Finalizing { .. } => {
                self.push_resource_state(id, RtResourceTransition::Finalizing(timestamp))
            }
            schema::StorageEvent::Exit { .. } => {
                self.push_resource_state(id, RtResourceTransition::Exit(timestamp))
            }
        }
    }

    fn push_gpu_memory(
        &mut self,
        id: Uuid,
        timestamp: quent_time::TimeUnixNanoSec,
        event: schema::GpuMemoryEvent,
    ) -> AnalyzerResult<()> {
        match event {
            schema::GpuMemoryEvent::Initializing {
                instance_name,
                gpu_id,
                ..
            } => self.initialize_resource(
                id,
                timestamp,
                "gpu_memory",
                instance_name,
                gpu_id.target,
                ResourceTypeDecl::new("gpu_memory", [CapacityDecl::new_occupancy("bytes")]),
            ),
            schema::GpuMemoryEvent::Operating { .. } => self.push_resource_state(
                id,
                RtResourceTransition::Operating(timestamp, ResourceCapacities(Vec::new())),
            ),
            schema::GpuMemoryEvent::Finalizing { .. } => {
                self.push_resource_state(id, RtResourceTransition::Finalizing(timestamp))
            }
            schema::GpuMemoryEvent::Exit { .. } => {
                self.push_resource_state(id, RtResourceTransition::Exit(timestamp))
            }
        }
    }

    fn push_thread(
        &mut self,
        id: Uuid,
        timestamp: quent_time::TimeUnixNanoSec,
        event: schema::TaskExecutorThreadEvent,
    ) -> AnalyzerResult<()> {
        match event {
            schema::TaskExecutorThreadEvent::Initializing {
                instance_name,
                task_executor_id,
                ..
            } => self.initialize_resource(
                id,
                timestamp,
                "task_executor_thread",
                instance_name,
                task_executor_id.target,
                ResourceTypeDecl::unit("task_executor_thread"),
            ),
            schema::TaskExecutorThreadEvent::Operating { .. } => self.push_resource_state(
                id,
                RtResourceTransition::Operating(timestamp, ResourceCapacities(Vec::new())),
            ),
            schema::TaskExecutorThreadEvent::Finalizing { .. } => {
                self.push_resource_state(id, RtResourceTransition::Finalizing(timestamp))
            }
            schema::TaskExecutorThreadEvent::Exit { .. } => {
                self.push_resource_state(id, RtResourceTransition::Exit(timestamp))
            }
        }
    }

    fn push_storage_channel(
        &mut self,
        id: Uuid,
        timestamp: quent_time::TimeUnixNanoSec,
        event: schema::StorageChannelEvent,
    ) -> AnalyzerResult<()> {
        match event {
            schema::StorageChannelEvent::Initializing {
                instance_name,
                worker_id,
                ..
            } => self.initialize_resource(
                id,
                timestamp,
                "storage_channel",
                instance_name,
                worker_id.target,
                ResourceTypeDecl::new("storage_channel", [CapacityDecl::new_rate("bytes")]),
            ),
            schema::StorageChannelEvent::Operating { .. } => self.push_resource_state(
                id,
                RtResourceTransition::Operating(timestamp, ResourceCapacities(Vec::new())),
            ),
            schema::StorageChannelEvent::Finalizing { .. } => {
                self.push_resource_state(id, RtResourceTransition::Finalizing(timestamp))
            }
            schema::StorageChannelEvent::Exit { .. } => {
                self.push_resource_state(id, RtResourceTransition::Exit(timestamp))
            }
        }
    }

    fn push_pcie_channel(
        &mut self,
        id: Uuid,
        timestamp: quent_time::TimeUnixNanoSec,
        event: schema::PcieChannelEvent,
    ) -> AnalyzerResult<()> {
        match event {
            schema::PcieChannelEvent::Initializing {
                instance_name,
                gpu_id,
                ..
            } => self.initialize_resource(
                id,
                timestamp,
                "pcie_channel",
                instance_name,
                gpu_id.target,
                ResourceTypeDecl::new("pcie_channel", [CapacityDecl::new_rate("bytes")]),
            ),
            schema::PcieChannelEvent::Operating { .. } => self.push_resource_state(
                id,
                RtResourceTransition::Operating(timestamp, ResourceCapacities(Vec::new())),
            ),
            schema::PcieChannelEvent::Finalizing { .. } => {
                self.push_resource_state(id, RtResourceTransition::Finalizing(timestamp))
            }
            schema::PcieChannelEvent::Exit { .. } => {
                self.push_resource_state(id, RtResourceTransition::Exit(timestamp))
            }
        }
    }

    fn push_network_channel(
        &mut self,
        id: Uuid,
        timestamp: quent_time::TimeUnixNanoSec,
        event: schema::NetworkChannelEvent,
    ) -> AnalyzerResult<()> {
        match event {
            schema::NetworkChannelEvent::Initializing {
                instance_name,
                network_id,
                ..
            } => self.initialize_resource(
                id,
                timestamp,
                "network_channel",
                instance_name,
                network_id.target,
                ResourceTypeDecl::new("network_channel", [CapacityDecl::new_rate("bytes")]),
            ),
            schema::NetworkChannelEvent::Operating { .. } => self.push_resource_state(
                id,
                RtResourceTransition::Operating(timestamp, ResourceCapacities(Vec::new())),
            ),
            schema::NetworkChannelEvent::Finalizing { .. } => {
                self.push_resource_state(id, RtResourceTransition::Finalizing(timestamp))
            }
            schema::NetworkChannelEvent::Exit { .. } => {
                self.push_resource_state(id, RtResourceTransition::Exit(timestamp))
            }
        }
    }

    pub(crate) fn try_build(self) -> AnalyzerResult<SimulatorModel> {
        // Build resources first. As we iterate over task builders and build all
        // tasks, we can populate the leaf resources used_by field.
        let mut resources = self.arbitrary_resources.try_build()?;
        let mut query_engine = self.query_engine.try_build()?;

        let mut tasks = HashMap::default();

        for (task_id, task_builder) in self.tasks.into_iter() {
            let task = Task::from_builder(task_builder)?;
            for usage in task.usages() {
                let resource_type_name = resources
                    .resource(usage.resource_id())?
                    .type_name()
                    .to_owned();
                let set = &mut resources
                    .resource_types
                    .get_mut(&resource_type_name)
                    .unwrap()
                    .used_by;
                if !set.contains(task.type_name()) {
                    set.insert(task.type_name().to_owned());
                }
            }
            if let Some(operator_id) = task.operator_id()
                && let Some(task_span) = task.active_span()
                && let Ok(operator) = query_engine.operator_mut(operator_id)
            {
                operator.extend_active_span(task_span);
            }

            tasks.insert(task_id, task);
        }

        // Construct the model without group type decls being populated yet, we
        // will populate it based on the resource tree.
        let temp_model = SimulatorModel {
            query_engine,
            arbitrary_resources: resources,
            tasks,
            resource_group_types: HashMap::default(),
        };
        let mut resource_group_types = derive_resource_group_types(&temp_model)?;
        // Bubble up all the used_by_entity fields in the group type decls.
        for group_type_decl in resource_group_types.values_mut() {
            for contained_resource_type in &group_type_decl.contains_resource_types {
                if let Ok(resource_type) = temp_model
                    .arbitrary_resources
                    .resource_type(contained_resource_type)
                {
                    for entity_type in &resource_type.used_by {
                        group_type_decl
                            .used_by_entity_types
                            .insert(entity_type.clone());
                    }
                }
            }
        }

        Ok(SimulatorModel {
            query_engine: temp_model.query_engine,
            arbitrary_resources: temp_model.arbitrary_resources,
            tasks: temp_model.tasks,
            resource_group_types,
        })
    }
}
