// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Model, RefTreeEntity,
    ref_tree::RefTreeCollection,
    resource::{Resource, ResourceTypeDecl, collection::ResourceCollection},
};
use quent_query_engine_analyzer::{QueryEngineModel, plan_tree::PlanTree};
use quent_query_engine_ui::EntityRef;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use uuid::Uuid;

use crate::{
    boilerplate::{
        Engine, Gpu, Network, Operator, Plan, Port, Query, QueryGroup, Task, TaskExecutor, TaskExt,
        Worker,
    },
    model::SimulatorModel,
};

/// A view of the simulator model filtered to a specific query
// TODO(johanpel): figure out a better way to construct these views, or to
// filter the data on a per query basis. This is generally tricky because the
// state of resources of engines that are shared across query groups or across
// the entire engine could be modified by other queries.
pub(crate) struct SimulatorModelQueryView<'a> {
    resource_types: HashMap<String, &'a ResourceTypeDecl>,
    engine: &'a Engine,
    query_group: &'a QueryGroup,
    query: &'a Query,
    workers: HashMap<Uuid, &'a Worker>,
    plans: HashMap<Uuid, &'a Plan>,
    operators: HashMap<Uuid, &'a Operator>,
    ports: HashMap<Uuid, &'a Port>,
    resources: HashMap<Uuid, &'a dyn Resource>,
    ref_tree_entities: HashMap<Uuid, &'a dyn RefTreeEntity>,
    task_executors: HashMap<Uuid, &'a TaskExecutor>,
    networks: HashMap<Uuid, &'a Network>,
    gpus: HashMap<Uuid, &'a Gpu>,
    tasks: HashMap<Uuid, &'a Task>,
}

impl<'a> SimulatorModelQueryView<'a> {
    pub fn try_new(
        model: &'a SimulatorModel,
        query_id: Uuid,
    ) -> AnalyzerResult<SimulatorModelQueryView<'a>> {
        let query = model.query(query_id)?;
        let query_group = model.query_group(query.query_group_id().unwrap_or_default())?;
        let workers: HashMap<Uuid, &Worker> = model
            .query_workers(query_id)?
            .map(|entity| (entity.id(), entity))
            .collect();
        let plans: HashMap<Uuid, &Plan> = model
            .query_plans(query_id)?
            .map(|entity| (entity.id(), entity))
            .collect();
        let operators: HashMap<Uuid, &Operator> = model
            .plans_operators(plans.values().copied())?
            .map(|entity| (entity.id(), entity))
            .collect();
        let ports: HashMap<Uuid, &Port> = model
            .operators_ports(operators.values().copied())?
            .map(|entity| (entity.id(), entity))
            .collect();
        let query_engine_group_ids: HashSet<Uuid> = std::iter::once(model.engine.id())
            .chain(std::iter::once(query_group.id()))
            .chain(std::iter::once(query.id()))
            .chain(workers.keys().copied())
            .chain(plans.keys().copied())
            .chain(operators.keys().copied())
            .chain(ports.keys().copied())
            .collect();

        let task_executors = model
            .task_executors
            .iter()
            .filter(|(_, entity)| {
                entity
                    .parent_id()
                    .is_some_and(|parent| query_engine_group_ids.contains(&parent))
            })
            .map(|(id, entity)| (*id, entity))
            .collect::<HashMap<_, _>>();
        let networks = model
            .networks
            .iter()
            .filter(|(_, entity)| {
                entity
                    .parent_id()
                    .is_some_and(|parent| query_engine_group_ids.contains(&parent))
            })
            .map(|(id, entity)| (*id, entity))
            .collect::<HashMap<_, _>>();
        let gpus = model
            .gpus
            .iter()
            .filter(|(_, entity)| {
                entity
                    .parent_id()
                    .is_some_and(|parent| query_engine_group_ids.contains(&parent))
            })
            .map(|(id, entity)| (*id, entity))
            .collect::<HashMap<_, _>>();

        let resources = model
            .resources()
            .filter(|resource| {
                model
                    .ref_tree_entity(resource.id())
                    .ok()
                    .and_then(RefTreeEntity::parent_id)
                    .is_some_and(|parent_id| {
                        query_engine_group_ids.contains(&parent_id)
                            || task_executors.contains_key(&parent_id)
                            || networks.contains_key(&parent_id)
                            || gpus.contains_key(&parent_id)
                    })
            })
            .map(|resource| (resource.id(), resource))
            .collect::<HashMap<_, _>>();

        let resource_types = model
            .resource_types
            .iter()
            .map(|(k, v)| (k.clone(), v))
            .collect();

        let tasks: HashMap<Uuid, &Task> = model
            .tasks
            .values()
            .map(|task| (task.id(), task))
            .filter(|(_, task)| {
                task.operator_id()
                    .is_some_and(|operator_id| operators.contains_key(&operator_id))
            })
            .collect();
        let scoped_entity_ids = std::iter::once(model.engine.id())
            .chain(std::iter::once(query_group.id()))
            .chain(std::iter::once(query.id()))
            .chain(workers.keys().copied())
            .chain(plans.keys().copied())
            .chain(operators.keys().copied())
            .chain(ports.keys().copied())
            .chain(task_executors.keys().copied())
            .chain(networks.keys().copied())
            .chain(gpus.keys().copied())
            .chain(resources.keys().copied())
            .chain(tasks.keys().copied());
        let ref_tree_entities = scoped_entity_ids
            .map(|id| model.ref_tree_entity(id).map(|entity| (id, entity)))
            .collect::<AnalyzerResult<HashMap<_, _>>>()?;

        Ok(SimulatorModelQueryView {
            resource_types,
            engine: &model.engine,
            query_group,
            query,
            workers,
            plans,
            operators,
            ports,
            resources,
            ref_tree_entities,
            task_executors,
            networks,
            gpus,
            tasks,
        })
    }
}

impl<'a> QueryEngineModel for SimulatorModelQueryView<'a> {
    type Engine = Engine;
    type Query = Query;
    type QueryGroup = QueryGroup;
    type Worker = Worker;
    type Plan = Plan;
    type Operator = Operator;
    type Port = Port;

    fn engine(&self) -> AnalyzerResult<&Engine> {
        Ok(self.engine)
    }
    fn query(&self, query_id: Uuid) -> AnalyzerResult<&Query> {
        (self.query.id() == query_id)
            .then_some(self.query)
            .ok_or(AnalyzerError::InvalidId(query_id))
    }
    fn query_group(&self, query_group_id: Uuid) -> AnalyzerResult<&QueryGroup> {
        (self.query_group.id() == query_group_id)
            .then_some(self.query_group)
            .ok_or(AnalyzerError::InvalidId(query_group_id))
    }
    fn worker(&self, worker_id: Uuid) -> AnalyzerResult<&Worker> {
        self.workers
            .get(&worker_id)
            .copied()
            .ok_or(AnalyzerError::InvalidId(worker_id))
    }
    fn plan(&self, plan_id: Uuid) -> AnalyzerResult<&Plan> {
        self.plans
            .get(&plan_id)
            .copied()
            .ok_or(AnalyzerError::InvalidId(plan_id))
    }
    fn operator(&self, operator_id: Uuid) -> AnalyzerResult<&Operator> {
        self.operators
            .get(&operator_id)
            .copied()
            .ok_or(AnalyzerError::InvalidId(operator_id))
    }
    fn port(&self, port_id: Uuid) -> AnalyzerResult<&Port> {
        self.ports
            .get(&port_id)
            .copied()
            .ok_or(AnalyzerError::InvalidId(port_id))
    }
    fn queries(&self) -> impl Iterator<Item = &Query> {
        std::iter::once(self.query)
    }
    fn query_groups(&self) -> impl Iterator<Item = &QueryGroup> {
        std::iter::once(self.query_group)
    }
    fn workers(&self) -> impl Iterator<Item = &Worker> {
        self.workers.values().copied()
    }
    fn plans(&self) -> impl Iterator<Item = &Plan> {
        self.plans.values().copied()
    }
    fn operators(&self) -> impl Iterator<Item = &Operator> {
        self.operators.values().copied()
    }
    fn ports(&self) -> impl Iterator<Item = &Port> {
        self.ports.values().copied()
    }
    fn plan_tree(&self, query_id: Uuid) -> AnalyzerResult<PlanTree> {
        PlanTree::try_new(self.plans.values().copied(), query_id)
    }
}

impl<'a> Model for SimulatorModelQueryView<'a> {
    type EntityIdType = EntityRef;
    fn try_entity_ref(&self, entity_id: Uuid) -> AnalyzerResult<Self::EntityIdType> {
        if self.engine.id() == entity_id {
            Ok(EntityRef::Engine(entity_id))
        } else if self.workers.contains_key(&entity_id) {
            Ok(EntityRef::Worker(entity_id))
        } else if self.query_group.id() == entity_id {
            Ok(EntityRef::QueryGroup(entity_id))
        } else if self.query.id() == entity_id {
            Ok(EntityRef::Query(entity_id))
        } else if self.plans.contains_key(&entity_id) {
            Ok(EntityRef::Plan(entity_id))
        } else if self.operators.contains_key(&entity_id) {
            Ok(EntityRef::Operator(entity_id))
        } else if self.ports.contains_key(&entity_id) {
            Ok(EntityRef::Port(entity_id))
        } else if self.resources.contains_key(&entity_id) {
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

impl<'a> ResourceCollection for SimulatorModelQueryView<'a> {
    fn resources(&self) -> impl Iterator<Item = &dyn Resource> {
        self.resources.values().map(|r| *r as &dyn Resource)
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
}

impl RefTreeCollection for SimulatorModelQueryView<'_> {
    fn ref_tree_entities(&self) -> impl Iterator<Item = &dyn RefTreeEntity> {
        self.ref_tree_entities.values().copied()
    }

    fn ref_tree_entity(&self, entity_id: Uuid) -> AnalyzerResult<&dyn RefTreeEntity> {
        self.ref_tree_entities
            .get(&entity_id)
            .copied()
            .ok_or(AnalyzerError::InvalidId(entity_id))
    }
}
