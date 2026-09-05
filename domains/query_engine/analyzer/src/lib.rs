// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

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

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Model, Span,
    fsm::Fsm,
    resource::{ResourceGroup, Using},
};
use quent_query_engine_model::plan::{Edge, PlanParent};
use quent_query_engine_ui as qe_ui;
use quent_time::{TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec};
use uuid::Uuid;

// Storage implementations
pub mod plain;
pub mod plan_tree;

// UI related mods
pub mod entities;
pub mod ui;

/// Read-only analyzer API for an engine entity.
pub trait EngineEntity: Entity + Span + ResourceGroup {
    fn to_ui(&self) -> AnalyzerResult<qe_ui::Engine>;
}

/// Read-only analyzer API for a worker entity.
pub trait WorkerEntity: Entity + Span + ResourceGroup {
    fn to_ui(&self, epoch: TimeUnixNanoSec) -> qe_ui::Worker;
}

/// Read-only analyzer API for a query-group entity.
pub trait QueryGroupEntity: Entity + ResourceGroup {
    fn to_ui(&self) -> qe_ui::QueryGroup;
}

/// Read-only analyzer API for a query entity.
pub trait QueryEntity: Fsm + Using + ResourceGroup {
    fn query_group_id(&self) -> Option<Uuid>;
    fn to_ui(&self) -> AnalyzerResult<qe_ui::Query>;
}

/// Read-only analyzer API for a plan entity.
pub trait PlanEntity: Entity + ResourceGroup {
    fn parent(&self) -> Option<&PlanParent>;
    fn worker_id(&self) -> Option<Uuid>;
    fn edges(&self) -> &[Edge];
    fn to_ui(&self) -> qe_ui::Plan;
}

/// Read-only analyzer API for an operator entity.
pub trait OperatorEntity: Entity + ResourceGroup {
    fn plan_id(&self) -> Option<Uuid>;
    fn parent_operator_ids(&self) -> impl ExactSizeIterator<Item = Uuid> + '_;
    fn active_span(&self) -> Option<SpanUnixNanoSec>;
    fn operator_type_name(&self) -> Option<&str>;
    fn to_ui(&self, epoch: TimeUnixNanoSec) -> qe_ui::Operator;
}

/// Mutable analyzer API for an operator entity.
pub trait OperatorEntityMut: OperatorEntity {
    /// Extends the active span to include `span`.
    fn extend_active_span(&mut self, span: SpanUnixNanoSec);
}

/// Read-only analyzer API for a port entity.
pub trait PortEntity: Entity + ResourceGroup {
    fn operator_id(&self) -> Option<Uuid>;
    fn to_ui(&self, epoch: TimeUnixNanoSec) -> qe_ui::Port;
}

pub trait QueryEngineModel: Model {
    type Engine: EngineEntity;
    type Query: QueryEntity;
    type QueryGroup: QueryGroupEntity;
    type Worker: WorkerEntity;
    type Plan: PlanEntity;
    type Operator: OperatorEntity;
    type Port: PortEntity;

    // Lookup functions.

    fn engine(&self) -> AnalyzerResult<&Self::Engine>;
    fn query(&self, query_id: Uuid) -> AnalyzerResult<&Self::Query>;
    fn query_group(&self, query_group_id: Uuid) -> AnalyzerResult<&Self::QueryGroup>;
    fn worker(&self, worker_id: Uuid) -> AnalyzerResult<&Self::Worker>;
    fn plan(&self, plan_id: Uuid) -> AnalyzerResult<&Self::Plan>;
    fn operator(&self, operator_id: Uuid) -> AnalyzerResult<&Self::Operator>;
    fn port(&self, port_id: Uuid) -> AnalyzerResult<&Self::Port>;

    // Entity iterators

    fn queries(&self) -> impl Iterator<Item = &Self::Query>;
    fn query_groups(&self) -> impl Iterator<Item = &Self::QueryGroup>;
    fn workers(&self) -> impl Iterator<Item = &Self::Worker>;
    fn plans(&self) -> impl Iterator<Item = &Self::Plan>;
    fn operators(&self) -> impl Iterator<Item = &Self::Operator>;
    fn ports(&self) -> impl Iterator<Item = &Self::Port>;

    // Query-related functions.

    /// Return an iterator over all plans of a query.
    fn query_plans(&self, query_id: Uuid) -> AnalyzerResult<impl Iterator<Item = &Self::Plan>> {
        Ok(self
            .plan_tree(query_id)?
            .iter()
            .map(|p| self.plan(p.id))
            .collect::<AnalyzerResult<Vec<_>>>()?
            .into_iter())
    }

    /// Return an iterator over all workers that contributed to a query.
    fn query_workers(&self, query_id: Uuid) -> AnalyzerResult<impl Iterator<Item = &Self::Worker>> {
        Ok(self
            .query_plans(query_id)?
            .filter_map(|p| p.worker_id().and_then(|w| self.worker(w).ok())))
    }

    /// Return the time at which a query started.
    fn query_epoch(&self, query_id: Uuid) -> AnalyzerResult<TimeUnixNanoSec> {
        self.query(query_id).and_then(|q| {
            q.transition(0).map(|init| init.timestamp()).ok_or_else(|| {
                AnalyzerError::Validation("query does not have any transitions".to_string())
            })
        })
    }

    // Plan-related functions.

    /// Return the tree of plans that processed a query.
    fn plan_tree(&self, query_id: Uuid) -> AnalyzerResult<plan_tree::PlanTree>;

    /// Return all operators that worked on any of the supplied plans.
    fn plans_operators<'a>(
        &'a self,
        plans: impl Iterator<Item = &'a Self::Plan>,
    ) -> AnalyzerResult<impl Iterator<Item = &'a Self::Operator>> {
        let plan_ids = plans.map(|plan| plan.id()).collect::<HashSet<_>>();
        Ok(self.operators().filter(move |op| {
            op.plan_id()
                .is_some_and(|plan_id| plan_ids.contains(&plan_id))
        }))
    }

    // Operator-related functions.

    /// Return all ports of the supplied operators.
    fn operators_ports<'a>(
        &'a self,
        operators: impl Iterator<Item = &'a Self::Operator>,
    ) -> AnalyzerResult<impl Iterator<Item = &'a Self::Port>> {
        let operator_ids = operators.map(|op| op.id()).collect::<HashSet<_>>();
        Ok(self.ports().filter(move |port| {
            port.operator_id()
                .is_some_and(|op_id| operator_ids.contains(&op_id))
        }))
    }
}

/// Mutable analyzer API for a query-engine model.
pub trait QueryEngineModelMut: QueryEngineModel
where
    Self::Operator: OperatorEntityMut,
{
    fn operator_mut(&mut self, operator_id: Uuid) -> AnalyzerResult<&mut Self::Operator>;
}
