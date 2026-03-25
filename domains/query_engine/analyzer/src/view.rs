// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Model,
    resource::{Resource, ResourceGroup, ResourceTypeDecl, collection::ResourceCollection},
};
use rustc_hash::FxHashMap as HashMap;
use uuid::Uuid;

use crate::{
    QueryEngineModel,
    engine::Engine,
    model::{InMemoryQueryEngineModel, QueryEngineEntityId},
    operator::Operator,
    plan::{Plan, tree::PlanTree},
    port::Port,
    query::Query,
    query_group::QueryGroup,
    worker::Worker,
};

/// A view of a query engine model scoped by one single query.
pub struct InMemoryQueryEngineModelView<'a> {
    pub engine: &'a Engine,
    pub query_group: &'a QueryGroup,
    pub query: &'a Query,
    pub workers: HashMap<Uuid, &'a Worker>,
    pub plans: HashMap<Uuid, &'a Plan>,
    pub operators: HashMap<Uuid, &'a Operator>,
    pub ports: HashMap<Uuid, &'a Port>,
}

impl<'a> InMemoryQueryEngineModelView<'a> {
    pub fn try_new(
        model: &'a InMemoryQueryEngineModel,
        query_id: Uuid,
    ) -> AnalyzerResult<InMemoryQueryEngineModelView<'a>> {
        let engine = &model.engine;
        let query = model.query(query_id)?;
        let query_group = model.query_group(query.query_group_id)?;
        let workers = model.query_workers(query_id)?.collect::<Vec<_>>();
        let plans = model.query_plans(query_id)?.collect::<Vec<_>>();
        let operators = model
            .plans_operators(plans.iter().copied())?
            .collect::<Vec<_>>();
        let ports = model
            .operators_ports(operators.iter().copied())?
            .collect::<Vec<_>>();

        Ok(Self {
            engine,
            query_group,
            query,
            workers: workers.into_iter().map(|w| (w.id, w)).collect(),
            plans: plans.into_iter().map(|w| (w.id, w)).collect(),
            operators: operators.into_iter().map(|w| (w.id, w)).collect(),
            ports: ports.into_iter().map(|w| (w.id, w)).collect(),
        })
    }
}

impl<'a> Model for InMemoryQueryEngineModelView<'a> {
    type EntityIdType = QueryEngineEntityId;

    fn try_entity_ref(&self, entity_id: Uuid) -> AnalyzerResult<Self::EntityIdType> {
        if self.engine.id == entity_id {
            Ok(QueryEngineEntityId::Engine(entity_id))
        } else if self.workers.contains_key(&entity_id) {
            Ok(QueryEngineEntityId::Worker(entity_id))
        } else if self.query_group.id == entity_id {
            Ok(QueryEngineEntityId::QueryGroup(entity_id))
        } else if self.query.id == entity_id {
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
        Ok(self.engine)
    }
}

impl<'a> QueryEngineModel for InMemoryQueryEngineModelView<'a> {
    fn engine(&self) -> AnalyzerResult<&Engine> {
        Ok(self.engine)
    }
    fn query(&self, query_id: Uuid) -> AnalyzerResult<&Query> {
        (self.query.id == query_id)
            .then_some(self.query)
            .ok_or(AnalyzerError::InvalidId(query_id))
    }
    fn query_group(&self, query_group_id: Uuid) -> AnalyzerResult<&QueryGroup> {
        (self.query_group.id == query_group_id)
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

impl<'a> ResourceCollection for InMemoryQueryEngineModelView<'a> {
    fn resources(&self) -> impl Iterator<Item = &dyn Resource> {
        std::iter::empty()
    }

    fn resource_groups(&self) -> impl Iterator<Item = &dyn ResourceGroup> {
        std::iter::once(self.engine as &dyn ResourceGroup)
            .chain(std::iter::once(self.query_group as &dyn ResourceGroup))
            .chain(std::iter::once(self.query as &dyn ResourceGroup))
            .chain(self.workers.values().map(|&w| w as &dyn ResourceGroup))
            .chain(self.plans.values().map(|&p| p as &dyn ResourceGroup))
            .chain(self.operators.values().map(|&o| o as &dyn ResourceGroup))
            .chain(self.ports.values().map(|&p| p as &dyn ResourceGroup))
    }

    fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource> {
        Err(AnalyzerError::InvalidId(resource_id))
    }

    fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl> {
        Err(AnalyzerError::InvalidTypeName(
            resource_type_name.to_owned(),
        ))
    }

    fn resource_group(&self, resource_group_id: Uuid) -> AnalyzerResult<&dyn ResourceGroup> {
        match self.try_entity_ref(resource_group_id)? {
            QueryEngineEntityId::Engine(_) => Ok(self.engine),
            QueryEngineEntityId::Worker(_) => Ok(*self.workers.get(&resource_group_id).unwrap()),
            QueryEngineEntityId::QueryGroup(_) => Ok(self.query_group),
            QueryEngineEntityId::Query(_) => Ok(self.query),
            QueryEngineEntityId::Plan(_) => Ok(*self.plans.get(&resource_group_id).unwrap()),
            QueryEngineEntityId::Operator(_) => {
                Ok(*self.operators.get(&resource_group_id).unwrap())
            }
            QueryEngineEntityId::Port(_) => Ok(*self.ports.get(&resource_group_id).unwrap()),
        }
    }

    fn resource_group_child_groups(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        self.resource_group(resource_group_id)?;

        let workers = self.workers.values().filter_map(move |w| {
            (w.parent_group_id() == Some(resource_group_id)).then_some(w.id())
        });
        let query_group = (self.query_group.parent_group_id() == Some(resource_group_id))
            .then_some(self.query_group.id());
        let query =
            (self.query.parent_group_id() == Some(resource_group_id)).then_some(self.query.id());
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
            .chain(query_group)
            .chain(query)
            .chain(plans)
            .chain(operators)
            .chain(ports))
    }

    fn resource_group_child_resources(
        &self,
        _resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        Ok(std::iter::empty())
    }
}
