// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, EntityId, Model,
    resource::{Resource, ResourceGroup, ResourceTypeDecl, collection::ResourceCollection},
};
use quent_events::Event;
use quent_query_engine_events::QueryEngineEvent;
use rustc_hash::FxHashMap as HashMap;
use uuid::Uuid;

use crate::{
    QueryEngineModel,
    engine::Engine,
    operator::Operator,
    plan::{Plan, tree::PlanTree},
    port::Port,
    query::{Query, QueryBuilder},
    query_group::QueryGroup,
    view::InMemoryQueryEngineModelView,
    worker::Worker,
};

#[derive(Debug)]
pub struct InMemoryQueryEngineModel {
    pub engine: Engine,
    pub workers: HashMap<Uuid, Worker>,
    pub query_groups: HashMap<Uuid, QueryGroup>,
    pub queries: HashMap<Uuid, Query>,
    pub plans: HashMap<Uuid, Plan>,
    pub operators: HashMap<Uuid, Operator>,
    pub ports: HashMap<Uuid, Port>,
}

/// Entity ID with entity type information.
pub enum QueryEngineEntityId {
    Engine(Uuid),
    Worker(Uuid),
    QueryGroup(Uuid),
    Query(Uuid),
    Plan(Uuid),
    Operator(Uuid),
    Port(Uuid),
}

impl EntityId for QueryEngineEntityId {
    fn is_resource(&self) -> bool {
        false
    }
    fn is_resource_group(&self) -> bool {
        true
    }
}

impl From<QueryEngineEntityId> for Uuid {
    fn from(value: QueryEngineEntityId) -> Self {
        match value {
            QueryEngineEntityId::Engine(i) => i,
            QueryEngineEntityId::Worker(i) => i,
            QueryEngineEntityId::QueryGroup(i) => i,
            QueryEngineEntityId::Query(i) => i,
            QueryEngineEntityId::Plan(i) => i,
            QueryEngineEntityId::Operator(i) => i,
            QueryEngineEntityId::Port(i) => i,
        }
    }
}

impl Model for InMemoryQueryEngineModel {
    type EntityIdType = QueryEngineEntityId;

    fn try_entity_ref(&self, entity_id: Uuid) -> AnalyzerResult<Self::EntityIdType> {
        if self.engine.id == entity_id {
            Ok(QueryEngineEntityId::Engine(entity_id))
        } else if self.workers.contains_key(&entity_id) {
            Ok(QueryEngineEntityId::Worker(entity_id))
        } else if self.query_groups.contains_key(&entity_id) {
            Ok(QueryEngineEntityId::QueryGroup(entity_id))
        } else if self.queries.contains_key(&entity_id) {
            Ok(QueryEngineEntityId::Query(entity_id))
        } else if self.plans.contains_key(&entity_id) {
            Ok(QueryEngineEntityId::Plan(entity_id))
        } else if self.operators.contains_key(&entity_id) {
            Ok(QueryEngineEntityId::Operator(entity_id))
        } else if self.ports.contains_key(&entity_id) {
            Ok(QueryEngineEntityId::Port(entity_id))
        } else {
            Err(AnalyzerError::InvalidId(entity_id))
        }
    }

    fn root(&self) -> AnalyzerResult<&impl ResourceGroup> {
        Ok(&self.engine)
    }
}

impl QueryEngineModel for InMemoryQueryEngineModel {
    fn engine(&self) -> AnalyzerResult<&Engine> {
        Ok(&self.engine)
    }
    fn query(&self, query_id: Uuid) -> AnalyzerResult<&Query> {
        self.queries
            .get(&query_id)
            .ok_or(AnalyzerError::InvalidId(query_id))
    }
    fn query_group(&self, query_group: Uuid) -> AnalyzerResult<&QueryGroup> {
        self.query_groups
            .get(&query_group)
            .ok_or(AnalyzerError::InvalidId(query_group))
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

    fn plan_tree(&self, query_id: Uuid) -> AnalyzerResult<crate::plan::tree::PlanTree> {
        PlanTree::try_new(self.plans.values(), query_id)
    }
}

impl ResourceCollection for InMemoryQueryEngineModel {
    fn resources(&self) -> impl Iterator<Item = &dyn Resource> {
        std::iter::empty()
    }

    fn resource_groups(&self) -> impl Iterator<Item = &dyn ResourceGroup> {
        std::iter::once(&self.engine as &dyn ResourceGroup)
            .chain(self.workers.values().map(|w| w as &dyn ResourceGroup))
            .chain(self.query_groups.values().map(|g| g as &dyn ResourceGroup))
            .chain(self.queries.values().map(|q| q as &dyn ResourceGroup))
            .chain(self.plans.values().map(|p| p as &dyn ResourceGroup))
            .chain(self.operators.values().map(|o| o as &dyn ResourceGroup))
            .chain(self.ports.values().map(|p| p as &dyn ResourceGroup))
    }

    fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource> {
        Err(AnalyzerError::InvalidId(resource_id))
    }

    fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl> {
        Err(AnalyzerError::InvalidArgument(format!(
            "resource type {resource_type_name} is unknown to the query engine model"
        )))
    }

    fn resource_group(&self, resource_group_id: Uuid) -> AnalyzerResult<&dyn ResourceGroup> {
        match self.try_entity_ref(resource_group_id)? {
            QueryEngineEntityId::Engine(_) => Ok(&self.engine),
            QueryEngineEntityId::Worker(_) => Ok(self.workers.get(&resource_group_id).unwrap()),
            QueryEngineEntityId::QueryGroup(_) => {
                Ok(self.query_groups.get(&resource_group_id).unwrap())
            }
            QueryEngineEntityId::Query(_) => Ok(self.queries.get(&resource_group_id).unwrap()),
            QueryEngineEntityId::Plan(_) => Ok(self.plans.get(&resource_group_id).unwrap()),
            QueryEngineEntityId::Operator(_) => Ok(self.operators.get(&resource_group_id).unwrap()),
            QueryEngineEntityId::Port(_) => Ok(self.ports.get(&resource_group_id).unwrap()),
        }
    }

    fn resource_group_child_groups(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // Sanity check
        self.resource_group(resource_group_id)?;

        let workers = self.workers.values().filter_map(move |w| {
            (w.parent_group_id() == Some(resource_group_id)).then_some(w.id())
        });
        let query_groups = self.query_groups.values().filter_map(move |qg| {
            (qg.parent_group_id() == Some(resource_group_id)).then_some(qg.id())
        });
        let queries = self.queries.values().filter_map(move |q| {
            (q.parent_group_id() == Some(resource_group_id)).then_some(q.id())
        });
        let plans = self.plans.values().filter_map(move |p| {
            (p.parent_group_id() == Some(resource_group_id)).then_some(p.id())
        });
        let operators = self.operators.values().filter_map(move |o| {
            (o.parent_group_id() == Some(resource_group_id)).then_some(o.id())
        });
        let ports = self.ports.values().filter_map(move |p| {
            (p.parent_group_id() == Some(resource_group_id)).then_some(p.id())
        });
        Ok(workers
            .chain(query_groups)
            .chain(queries)
            .chain(plans)
            .chain(operators)
            .chain(ports))
    }

    fn resource_group_child_resources(
        &self,
        _resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // Query engine model only contains resource groups, no leaf resources
        Ok(std::iter::empty())
    }
}

impl InMemoryQueryEngineModel {
    pub fn query_view<'a>(
        &'a self,
        query_id: Uuid,
    ) -> AnalyzerResult<InMemoryQueryEngineModelView<'a>> {
        InMemoryQueryEngineModelView::try_new(self, query_id)
    }
}

pub struct InMemoryQueryEngineModelBuilder {
    engine: Engine,
    workers: HashMap<Uuid, Worker>,
    query_groups: HashMap<Uuid, QueryGroup>,
    queries: HashMap<Uuid, QueryBuilder>,
    plans: HashMap<Uuid, Plan>,
    operators: HashMap<Uuid, Operator>,
    ports: HashMap<Uuid, Port>,
}

impl InMemoryQueryEngineModelBuilder {
    pub fn try_new(engine_id: Uuid) -> AnalyzerResult<Self> {
        if engine_id.is_nil() {
            Err(AnalyzerError::Validation(
                "engine id cannot be nil".to_string(),
            ))?
        } else {
            Ok(Self {
                engine: Engine::new(engine_id),
                workers: Default::default(),
                query_groups: Default::default(),
                queries: Default::default(),
                plans: Default::default(),
                operators: Default::default(),
                ports: Default::default(),
            })
        }
    }

    pub fn try_push(&mut self, event: Event<QueryEngineEvent>) -> AnalyzerResult<()> {
        let Event {
            id,
            timestamp,
            data,
        } = event;
        match data {
            QueryEngineEvent::Engine(e) => self.engine.push(Event::new(id, timestamp, e)),
            QueryEngineEvent::Worker(e) => {
                if let std::collections::hash_map::Entry::Vacant(e) = self.workers.entry(id) {
                    e.insert(Worker::try_new(id)?);
                }
                self.workers
                    .get_mut(&id)
                    .unwrap()
                    .push(Event::new(id, timestamp, e));
            }
            QueryEngineEvent::QueryGroup(e) => {
                if let std::collections::hash_map::Entry::Vacant(e) = self.query_groups.entry(id) {
                    e.insert(QueryGroup::try_new(id)?);
                }
                self.query_groups
                    .get_mut(&id)
                    .unwrap()
                    .push(Event::new(id, timestamp, e));
            }
            QueryEngineEvent::Query(e) => {
                if let std::collections::hash_map::Entry::Vacant(e) = self.queries.entry(id) {
                    e.insert(QueryBuilder::try_new(id)?);
                }
                self.queries
                    .get_mut(&id)
                    .unwrap()
                    .push(Event::new(id, timestamp, e));
            }
            QueryEngineEvent::Plan(e) => {
                if let std::collections::hash_map::Entry::Vacant(e) = self.plans.entry(id) {
                    e.insert(Plan::try_new(id)?);
                }
                self.plans
                    .get_mut(&id)
                    .unwrap()
                    .push(Event::new(id, timestamp, e));
            }
            QueryEngineEvent::Operator(e) => {
                if let std::collections::hash_map::Entry::Vacant(e) = self.operators.entry(id) {
                    e.insert(Operator::try_new(id)?);
                }
                self.operators
                    .get_mut(&id)
                    .unwrap()
                    .push(Event::new(id, timestamp, e));
            }
            QueryEngineEvent::Port(e) => {
                if let std::collections::hash_map::Entry::Vacant(e) = self.ports.entry(id) {
                    e.insert(Port::try_new(id)?);
                }
                self.ports
                    .get_mut(&id)
                    .unwrap()
                    .push(Event::new(id, timestamp, e));
            }
        }
        Ok(())
    }

    pub fn try_extend(
        &mut self,
        iterator: impl Iterator<Item = Event<QueryEngineEvent>>,
    ) -> AnalyzerResult<()> {
        for event in iterator {
            self.try_push(event)?;
        }
        Ok(())
    }

    pub fn try_build(self) -> AnalyzerResult<InMemoryQueryEngineModel> {
        Ok(InMemoryQueryEngineModel {
            engine: self.engine,
            workers: self.workers,
            query_groups: self.query_groups,
            queries: self
                .queries
                .into_iter()
                .map(|(k, v)| v.try_build().map(|v| (k, v)))
                .collect::<AnalyzerResult<_>>()?,
            plans: self.plans,
            operators: self.operators,
            ports: self.ports,
        })
    }
}
