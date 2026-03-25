// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
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

use quent_analyzer::{AnalyzerError, AnalyzerResult, Model};
use quent_time::{TimeUnixNanoSec, Timestamp};
use uuid::Uuid;

use crate::{
    engine::Engine,
    operator::Operator,
    plan::{Plan, tree::PlanTree},
    port::Port,
    query::Query,
    query_group::QueryGroup,
    worker::Worker,
};

// Entity mods
pub mod engine;
pub mod operator;
pub mod plan;
pub mod port;
pub mod query;
pub mod query_group;
pub mod worker;

// Full model mods
pub mod model;
pub mod view;

// UI related mods
pub mod ui;

pub trait QueryEngineModel: Model {
    // Lookup functions.

    fn engine(&self) -> AnalyzerResult<&Engine>;
    fn query(&self, query_id: Uuid) -> AnalyzerResult<&Query>;
    fn query_group(&self, query_group_id: Uuid) -> AnalyzerResult<&QueryGroup>;
    fn worker(&self, worker_id: Uuid) -> AnalyzerResult<&Worker>;
    fn plan(&self, plan_id: Uuid) -> AnalyzerResult<&Plan>;
    fn operator(&self, operator_id: Uuid) -> AnalyzerResult<&Operator>;
    fn port(&self, port_id: Uuid) -> AnalyzerResult<&Port>;

    // Entity iterators

    fn queries(&self) -> impl Iterator<Item = &Query>;
    fn query_groups(&self) -> impl Iterator<Item = &QueryGroup>;
    fn workers(&self) -> impl Iterator<Item = &Worker>;
    fn plans(&self) -> impl Iterator<Item = &Plan>;
    fn operators(&self) -> impl Iterator<Item = &Operator>;
    fn ports(&self) -> impl Iterator<Item = &Port>;

    // Query-related functions.

    /// Return an iterator over all plans of a query.
    fn query_plans(&self, query_id: Uuid) -> AnalyzerResult<impl Iterator<Item = &Plan>> {
        Ok(self
            .plan_tree(query_id)?
            .iter()
            .map(|p| self.plan(p.id))
            .collect::<AnalyzerResult<Vec<_>>>()?
            .into_iter())
    }

    /// Return an iterator over all workers that contributed to a query.
    fn query_workers(&self, query_id: Uuid) -> AnalyzerResult<impl Iterator<Item = &Worker>> {
        Ok(self
            .query_plans(query_id)?
            .filter_map(|p| p.worker_id.and_then(|w| self.worker(w).ok())))
    }

    /// Return the time at which a query started.
    fn query_epoch(&self, query_id: Uuid) -> AnalyzerResult<TimeUnixNanoSec> {
        self.query(query_id).and_then(|q| {
            q.transitions
                .first()
                .map(|init| init.timestamp())
                .ok_or_else(|| {
                    AnalyzerError::Validation("query does not have any transitions".to_string())
                })
        })
    }

    // Plan-related functions.

    /// Return the tree of plans that processed a query.
    fn plan_tree(&self, query_id: Uuid) -> AnalyzerResult<PlanTree>;

    /// Return all operators that worked on any of the supplied plans.
    fn plans_operators<'a>(
        &'a self,
        plans: impl Iterator<Item = &'a Plan>,
    ) -> AnalyzerResult<impl Iterator<Item = &'a Operator>> {
        let plan_ids = plans.map(|plan| plan.id).collect::<HashSet<_>>();
        Ok(self.operators().filter(move |op| {
            op.plan_id
                .is_some_and(|plan_id| plan_ids.contains(&plan_id))
        }))
    }

    // Operator-related functions.

    /// Return all ports of the supplied operators.
    fn operators_ports<'a>(
        &'a self,
        operators: impl Iterator<Item = &'a Operator>,
    ) -> AnalyzerResult<impl Iterator<Item = &'a Port>> {
        let operator_ids = operators.map(|op| op.id).collect::<HashSet<_>>();
        Ok(self.ports().filter(move |port| {
            port.operator_id
                .is_some_and(|op_id| operator_ids.contains(&op_id))
        }))
    }
}
