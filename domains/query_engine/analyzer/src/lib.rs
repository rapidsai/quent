//! Analyzes raw events to produce useful performance insights.
//!
//! General analyzer TODOs for post-PoC:
//!
//! - Arrow-fication of the data. Right now, everything is deserialized into
//!   Rust native types. It's subjectively easier for now to capture modeling
//!   rules but when queries become more complicated, more run-time defined and
//!   interactive, it's most likely best to move this to a query engine in order
//!   to get better performance and scalability without too much engineering
//!   investment. Prior art used DataFusion.
//!
//! - Timeseries databases like InfluxDB have the ability to do various things
//!   like time binned aggregations etc. as well. How modeling rules and
//!   validation can be expressed in such frameworks is to be investigated.

use std::collections::HashSet;

use rustc_hash::FxHashMap as HashMap;

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::State,
    resource::{Resource, ResourceGroup, ResourceTypeDecl, collection::ResourceCollection},
};
use quent_events::Event;
use quent_query_engine_events::QueryEngineEvent;
use quent_time::TimeUnixNanoSec;
use uuid::Uuid;

use crate::{
    engine::Engine,
    operator::Operator,
    plan::{Plan, tree::PlanTree},
    port::Port,
    query::{Query, QueryBuilder},
    query_group::QueryGroup,
    worker::Worker,
};

pub mod engine;
pub mod operator;
pub mod plan;
pub mod port;
pub mod query;
pub mod query_group;
pub mod worker;

pub struct InMemoryEngineModelBuilder {
    engine: Engine,
    workers: HashMap<Uuid, Worker>,
    query_groups: HashMap<Uuid, QueryGroup>,
    queries: HashMap<Uuid, QueryBuilder>,
    plans: HashMap<Uuid, Plan>,
    operators: HashMap<Uuid, Operator>,
    ports: HashMap<Uuid, Port>,
}

impl InMemoryEngineModelBuilder {
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

    pub fn try_build(self) -> AnalyzerResult<InMemoryEngineModel> {
        Ok(InMemoryEngineModel {
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

#[derive(Debug)]
pub struct InMemoryEngineModel {
    pub engine: Engine,
    pub workers: HashMap<Uuid, Worker>,
    pub query_groups: HashMap<Uuid, QueryGroup>,
    pub queries: HashMap<Uuid, Query>,
    pub plans: HashMap<Uuid, Plan>,
    pub operators: HashMap<Uuid, Operator>,
    pub ports: HashMap<Uuid, Port>,
}

pub enum EntityRef {
    Engine(Uuid),
    Worker(Uuid),
    QueryGroup(Uuid),
    Query(Uuid),
    Plan(Uuid),
    Operator(Uuid),
    Port(Uuid),
}

//TODO(johanpel): if this is ever backed by something else, this needs to become a trait
impl InMemoryEngineModel {
    /// Look up any entity by its ID and return the corresponding [`EntityRef`].
    ///
    /// Returns `None` if the ID is not found in any of the entity collections.
    pub fn entity_ref(&self, id: Uuid) -> Option<EntityRef> {
        if self.engine.id == id {
            Some(EntityRef::Engine(id))
        } else if self.workers.contains_key(&id) {
            Some(EntityRef::Worker(id))
        } else if self.query_groups.contains_key(&id) {
            Some(EntityRef::QueryGroup(id))
        } else if self.queries.contains_key(&id) {
            Some(EntityRef::Query(id))
        } else if self.plans.contains_key(&id) {
            Some(EntityRef::Plan(id))
        } else if self.operators.contains_key(&id) {
            Some(EntityRef::Operator(id))
        } else if self.ports.contains_key(&id) {
            Some(EntityRef::Port(id))
        } else {
            None
        }
    }

    // Look-up functions for entities:

    pub fn query(&self, query_id: Uuid) -> AnalyzerResult<&Query> {
        self.queries
            .get(&query_id)
            .ok_or(AnalyzerError::InvalidId(query_id))
    }

    pub fn query_group(&self, query_group: Uuid) -> AnalyzerResult<&QueryGroup> {
        self.query_groups
            .get(&query_group)
            .ok_or(AnalyzerError::InvalidId(query_group))
    }

    pub fn worker(&self, worker_id: Uuid) -> AnalyzerResult<&Worker> {
        self.workers
            .get(&worker_id)
            .ok_or(AnalyzerError::InvalidId(worker_id))
    }

    pub fn plan(&self, plan_id: Uuid) -> AnalyzerResult<&Plan> {
        self.plans
            .get(&plan_id)
            .ok_or(AnalyzerError::InvalidId(plan_id))
    }

    pub fn operator(&self, operator_id: Uuid) -> AnalyzerResult<&Operator> {
        self.operators
            .get(&operator_id)
            .ok_or(AnalyzerError::InvalidId(operator_id))
    }

    pub fn port(&self, port_id: Uuid) -> AnalyzerResult<&Port> {
        self.ports
            .get(&port_id)
            .ok_or(AnalyzerError::InvalidId(port_id))
    }

    // Engine-related functions.

    pub fn engine_epoch(&self) -> AnalyzerResult<TimeUnixNanoSec> {
        self.engine
            .start_time_unix_ns
            .ok_or_else(|| AnalyzerError::Validation("engine has no start timestamp".to_string()))
    }

    // Query-related functions.

    pub fn query_plans(&self, query_id: Uuid) -> AnalyzerResult<impl Iterator<Item = &Plan>> {
        Ok(self
            .plan_tree(query_id)?
            .iter()
            .map(|p| self.plan(p.id))
            .collect::<AnalyzerResult<Vec<_>>>()?
            .into_iter())
    }

    pub fn query_workers(&self, query_id: Uuid) -> AnalyzerResult<impl Iterator<Item = &Worker>> {
        Ok(self
            .query_plans(query_id)?
            .filter_map(|p| p.worker_id.and_then(|w| self.worker(w).ok())))
    }

    pub fn query_epoch(&self, query_id: Uuid) -> AnalyzerResult<TimeUnixNanoSec> {
        self.queries
            .get(&query_id)
            .and_then(|q| q.sequence.first().map(|init| init.span().start()))
            .ok_or(AnalyzerError::Validation(format!(
                "query {query_id} has no start time"
            )))
    }

    /// Construct a view of the engine model data scoped to a single query.
    pub fn query_view<'a>(
        &'a self,
        query_id: Uuid,
    ) -> AnalyzerResult<InMemoryEngineModelQueryView<'a>> {
        InMemoryEngineModelQueryView::try_new(self, query_id)
    }

    // Plan-related functions.

    pub fn plan_tree(&self, query_id: Uuid) -> AnalyzerResult<PlanTree> {
        PlanTree::try_new(&self.plans, query_id)
    }

    pub fn plans_operators<'a>(
        &'a self,
        plans: impl Iterator<Item = &'a Plan>,
    ) -> AnalyzerResult<impl Iterator<Item = &'a Operator>> {
        let plan_ids = plans.map(|plan| plan.id).collect::<HashSet<_>>();
        Ok(self.operators.values().filter(move |op| {
            op.plan_id
                .is_some_and(|plan_id| plan_ids.contains(&plan_id))
        }))
    }

    // Operator-related functions.

    pub fn operators_ports<'a>(
        &'a self,
        operators: impl Iterator<Item = &'a Operator>,
    ) -> AnalyzerResult<impl Iterator<Item = &'a Port>> {
        let operator_ids = operators.map(|op| op.id).collect::<HashSet<_>>();
        Ok(self.ports.values().filter(move |port| {
            port.operator_id
                .is_some_and(|op_id| operator_ids.contains(&op_id))
        }))
    }
}

impl ResourceCollection for InMemoryEngineModel {
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
        // TODO(johanpel): this is expensive, consider multi-level indexing
        match self.entity_ref(resource_group_id) {
            Some(EntityRef::Engine(_)) => Ok(&self.engine),
            Some(EntityRef::Worker(_)) => Ok(self.workers.get(&resource_group_id).unwrap()),
            Some(EntityRef::QueryGroup(_)) => {
                Ok(self.query_groups.get(&resource_group_id).unwrap())
            }
            Some(EntityRef::Query(_)) => Ok(self.queries.get(&resource_group_id).unwrap()),
            Some(EntityRef::Plan(_)) => Ok(self.plans.get(&resource_group_id).unwrap()),
            Some(EntityRef::Operator(_)) => Ok(self.operators.get(&resource_group_id).unwrap()),
            Some(EntityRef::Port(_)) => Ok(self.ports.get(&resource_group_id).unwrap()),
            None => Err(AnalyzerError::InvalidId(resource_group_id)),
        }
    }

    fn resource_group_child_groups(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // Sanity check
        self.resource_group(resource_group_id)?;

        // TODO(johanpel): this is expensive. Consider caching this in the entity structs themselves.
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

pub struct InMemoryEngineModelQueryView<'a> {
    pub engine: &'a Engine,
    pub query_group: &'a QueryGroup,
    pub query: &'a Query,
    pub workers: HashMap<Uuid, &'a Worker>,
    pub plans: HashMap<Uuid, &'a Plan>,
    pub operators: HashMap<Uuid, &'a Operator>,
    pub ports: HashMap<Uuid, &'a Port>,
}

impl<'a> InMemoryEngineModelQueryView<'a> {
    pub fn entity_ref(&self, id: Uuid) -> Option<EntityRef> {
        if self.engine.id == id {
            Some(EntityRef::Engine(id))
        } else if self.workers.contains_key(&id) {
            Some(EntityRef::Worker(id))
        } else if self.query_group.id == id {
            Some(EntityRef::QueryGroup(id))
        } else if self.query.id == id {
            Some(EntityRef::Query(id))
        } else if self.plans.contains_key(&id) {
            Some(EntityRef::Plan(id))
        } else if self.operators.contains_key(&id) {
            Some(EntityRef::Operator(id))
        } else if self.ports.contains_key(&id) {
            Some(EntityRef::Port(id))
        } else {
            None
        }
    }

    pub fn try_new(
        model: &'a InMemoryEngineModel,
        query_id: Uuid,
    ) -> AnalyzerResult<InMemoryEngineModelQueryView<'a>> {
        let engine = &model.engine;
        let query = model.query(query_id)?;
        let query_group = model.query_group(query.query_group_id.ok_or_else(|| {
            AnalyzerError::Validation(format!("query {query_id} has no group"))
        })?)?;
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

impl<'a> ResourceCollection for InMemoryEngineModelQueryView<'a> {
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
        Err(AnalyzerError::InvalidArgument(format!(
            "resource type {resource_type_name} is unknown to the query engine model"
        )))
    }

    fn resource_group(&self, resource_group_id: Uuid) -> AnalyzerResult<&dyn ResourceGroup> {
        match self.entity_ref(resource_group_id) {
            Some(EntityRef::Engine(_)) => Ok(self.engine),
            Some(EntityRef::Worker(_)) => Ok(*self.workers.get(&resource_group_id).unwrap()),
            Some(EntityRef::QueryGroup(_)) => Ok(self.query_group),
            Some(EntityRef::Query(_)) => Ok(self.query),
            Some(EntityRef::Plan(_)) => Ok(*self.plans.get(&resource_group_id).unwrap()),
            Some(EntityRef::Operator(_)) => Ok(*self.operators.get(&resource_group_id).unwrap()),
            Some(EntityRef::Port(_)) => Ok(*self.ports.get(&resource_group_id).unwrap()),
            None => Err(AnalyzerError::InvalidId(resource_group_id)),
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
