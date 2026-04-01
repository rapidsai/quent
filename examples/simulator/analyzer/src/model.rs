// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use rustc_hash::FxHashMap as HashMap;

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Model,
    fsm::collection::FsmCollection,
    resource::{
        Resource, ResourceGroup, ResourceGroupTypeDecl, ResourceTypeDecl, Usage, Using,
        collection::{
            InMemoryResources, InMemoryResourcesBuilder, ResourceCollection,
            derive_resource_group_types,
        },
    },
    trace::RtTraceBuilder,
};
use quent_events::Event;
use quent_query_engine_analyzer::{
    QueryEngineModel,
    engine::Engine,
    model::{InMemoryQueryEngineModel, InMemoryQueryEngineModelBuilder, QueryEngineEntityId},
    operator::Operator,
    plan::{Plan, tree::PlanTree},
    port::Port,
    query::Query,
    query_group::QueryGroup,
    worker::Worker,
};
use quent_simulator_events::SimulatorEvent;
use quent_simulator_ui::EntityRef;
use uuid::Uuid;

use crate::{
    task::{Task, TaskBuilder},
    view::SimulatorModelQueryView,
};

/// A model of the simulator engine
pub struct SimulatorModel {
    pub(crate) query_engine: InMemoryQueryEngineModel,
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
                .contains_key(&entity_id)
                .then_some(EntityRef::Task(entity_id))
                .ok_or(AnalyzerError::InvalidId(entity_id))
        }
    }

    fn root(&self) -> AnalyzerResult<&impl ResourceGroup> {
        self.query_engine.root()
    }
}

impl QueryEngineModel for SimulatorModel {
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

impl SimulatorModel {
    pub(crate) fn query_view(&self, query_id: Uuid) -> AnalyzerResult<SimulatorModelQueryView<'_>> {
        SimulatorModelQueryView::try_new(self, query_id)
    }
}

impl FsmCollection<Task, crate::task::TaskTransition> for SimulatorModel {
    fn fsms<'a>(&'a self) -> impl Iterator<Item = &'a Task> + 'a
    where
        Task: 'a,
    {
        self.tasks.values()
    }

    fn contains_fsm_type(&self, type_name: &str) -> bool {
        !self.tasks.is_empty() && type_name == "task"
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
    query_engine: InMemoryQueryEngineModelBuilder,
    arbitrary_resources: InMemoryResourcesBuilder,
    traces: HashMap<Uuid, RtTraceBuilder>,
    tasks: HashMap<Uuid, TaskBuilder>,
}

impl SimulatorModelBuilder {
    pub(crate) fn try_new(engine_id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self {
            query_engine: InMemoryQueryEngineModelBuilder::try_new(engine_id)?,
            arbitrary_resources: InMemoryResourcesBuilder::default(),
            tasks: HashMap::default(),
            traces: HashMap::default(),
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
                    .entry(event.id)
                    .or_insert_with(|| TaskBuilder::try_new(event.id).unwrap());
                task_builder.push(Event::new(id, timestamp, t));
                Ok(())
            }
            SimulatorEvent::QueryEngineEvent(qe) => {
                self.query_engine.try_push(Event::new(id, timestamp, qe))
            }
            SimulatorEvent::Resource(r) => self
                .arbitrary_resources
                .try_push(Event::new(id, timestamp, r)),
            SimulatorEvent::Trace(t) => {
                let trace_builder = self
                    .traces
                    .entry(event.id)
                    .or_insert_with(|| RtTraceBuilder::try_new(event.id).unwrap());
                trace_builder.push(timestamp, t);
                Ok(())
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
            let task = task_builder.try_build()?;
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
                && let Some(operator) = query_engine.operators.get_mut(&operator_id)
            {
                operator.active_span = Some(match operator.active_span {
                    None => task_span,
                    Some(existing) => existing.extend(&task_span),
                });
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
