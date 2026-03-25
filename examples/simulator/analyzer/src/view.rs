// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Model,
    resource::{
        Resource, ResourceGroup, ResourceTypeDecl, Usage, Using,
        collection::ResourceCollection,
        runtime::{RtResource, RtResourceGroup},
    },
};
use quent_query_engine_analyzer::{
    QueryEngineModel,
    engine::Engine,
    model::QueryEngineEntityId as QeEntityRef,
    operator::Operator,
    plan::{Plan, tree::PlanTree},
    port::Port,
    query::Query,
    query_group::QueryGroup,
    view::InMemoryQueryEngineModelView,
    worker::Worker,
};
use quent_simulator_ui::EntityRef;
use rustc_hash::FxHashMap as HashMap;
use uuid::Uuid;

use crate::{model::SimulatorModel, task::Task};

/// A view of the simulator model filtered to a specific query
// TODO(johanpel): figure out a better way to construct these views, or to
// filter the data on a per query basis. This is generally tricky because the
// state of resources of engines that are shared across query groups or across
// the entire engine could be modified by other queries.
pub(crate) struct SimulatorModelQueryView<'a> {
    resource_types: HashMap<String, &'a ResourceTypeDecl>,
    query_engine: InMemoryQueryEngineModelView<'a>,
    resources: HashMap<Uuid, &'a RtResource>,
    resource_groups: HashMap<Uuid, &'a RtResourceGroup>,
    tasks: HashMap<Uuid, &'a Task>,
}

impl<'a> SimulatorModelQueryView<'a> {
    pub fn try_new(
        model: &'a SimulatorModel,
        query_id: Uuid,
    ) -> AnalyzerResult<SimulatorModelQueryView<'a>> {
        // QE scoped to single query
        let query_engine_view =
            InMemoryQueryEngineModelView::try_new(&model.query_engine, query_id)?;

        // Only keep arbitrary groups that reference one of the QE model groups
        let resource_groups = model
            .arbitrary_resources
            .resource_groups
            .iter()
            .filter(|(_, v)| {
                v.parent_group_id
                    .and_then(|parent| query_engine_view.resource_group(parent).ok())
                    .is_some()
            })
            .map(|(k, v)| (*k, v))
            .collect::<HashMap<_, _>>();

        let resources = model
            .arbitrary_resources
            .resources
            .iter()
            .filter(|(_, resource)| {
                // This needs to reference a QE resource group:
                let in_qe = query_engine_view
                    .resource_group(resource.parent_group_id())
                    .is_ok();
                // Or an arbitrary resource group.
                let in_sim = resource_groups.contains_key(&resource.parent_group_id());
                in_qe || in_sim
            })
            .map(|(k, v)| (*k, v))
            .collect::<HashMap<_, _>>();

        let resource_types = model
            .arbitrary_resources
            .resource_types
            .iter()
            .map(|(k, v)| (k.clone(), v))
            .collect();

        let mut result = SimulatorModelQueryView {
            resource_types,
            query_engine: query_engine_view,
            resource_groups,
            resources,
            tasks: HashMap::default(),
        };

        result.tasks = model
            .tasks
            .values()
            .map(|task| (task.id(), task))
            .filter(|(_, task)| {
                task.usages()
                    .any(|usage| result.resource(usage.resource_id()).is_ok())
            })
            .collect();
        Ok(result)
    }
}

impl<'a> QueryEngineModel for SimulatorModelQueryView<'a> {
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

impl<'a> Model for SimulatorModelQueryView<'a> {
    type EntityIdType = EntityRef;
    fn try_entity_ref(&self, entity_id: Uuid) -> AnalyzerResult<Self::EntityIdType> {
        if let Ok(qe_ref) = self.query_engine.try_entity_ref(entity_id) {
            Ok(match qe_ref {
                QeEntityRef::Engine(uuid) => EntityRef::Engine(uuid),
                QeEntityRef::Worker(uuid) => EntityRef::Worker(uuid),
                QeEntityRef::QueryGroup(uuid) => EntityRef::QueryGroup(uuid),
                QeEntityRef::Query(uuid) => EntityRef::Query(uuid),
                QeEntityRef::Plan(uuid) => EntityRef::Plan(uuid),
                QeEntityRef::Operator(uuid) => EntityRef::Operator(uuid),
                QeEntityRef::Port(uuid) => EntityRef::Port(uuid),
            })
        } else if self.resources.contains_key(&entity_id) {
            Ok(EntityRef::Resource(entity_id))
        } else if self.resource_groups.contains_key(&entity_id) {
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

impl<'a> ResourceCollection for SimulatorModelQueryView<'a> {
    fn resources(&self) -> impl Iterator<Item = &dyn Resource> {
        self.resources.values().map(|r| *r as &dyn Resource)
    }
    fn resource_groups(&self) -> impl Iterator<Item = &dyn ResourceGroup> {
        let qe_groups = self.query_engine.resource_groups();
        let sim_groups = self
            .resource_groups
            .values()
            .map(|r| *r as &dyn ResourceGroup);
        qe_groups.chain(sim_groups)
    }
    fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource> {
        // qe model has no leaf resources.
        self.resources
            .get(&resource_id)
            .map(|r| *r as &dyn Resource)
            .ok_or(AnalyzerError::InvalidId(resource_id))
    }
    fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl> {
        self.resource_types
            .get(resource_type_name)
            .copied()
            .ok_or_else(|| AnalyzerError::InvalidTypeName(resource_type_name.to_owned()))
    }
    fn resource_group(&self, resource_group_id: Uuid) -> AnalyzerResult<&dyn ResourceGroup> {
        let qe_group = self.query_engine.resource_group(resource_group_id);
        if qe_group.is_ok() {
            qe_group
        } else {
            self.resource_groups
                .get(&resource_group_id)
                .map(|r| *r as &dyn ResourceGroup)
                .ok_or(AnalyzerError::InvalidId(resource_group_id))
        }
    }
    fn resource_group_child_groups(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        self.resource_group(resource_group_id)?;
        Ok(self.resource_groups().filter_map(move |g| {
            g.parent_group_id()
                .is_some_and(|p| p == resource_group_id)
                .then_some(g.id())
        }))
    }
    fn resource_group_child_resources(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // Verify the resource group exists
        self.resource_group(resource_group_id)?;
        Ok(self
            .resources()
            .filter_map(move |r| (r.parent_group_id() == resource_group_id).then_some(r.id())))
    }
}
