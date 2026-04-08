// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Plan entity: a DAG of operators representing a query execution plan.

use quent_model::{EmitOnce, Entity, Event, Ref};
use serde::{Deserialize, Serialize};

/// A directed edge of a Plan DAG.
#[derive(Debug, Deserialize, Serialize)]
pub struct Edge {
    /// The ID of the port sourcing data.
    pub source: Ref<super::port::Port>,
    /// The ID of the port sinking data.
    pub target: Ref<super::port::Port>,
}

/// A reference to the parent of Plan.
#[derive(Debug, Deserialize, Serialize)]
pub enum PlanParent {
    /// The parent of this plan is a query, which means this is the source plan.
    Query(Ref<super::query::Query>),
    /// This is a nested plan.
    ///
    /// This is useful if an application constructs various types of plans
    /// before execution, sometimes referred to as "lowering". Examples include
    /// a logical and physical plan.
    Plan(Ref<super::plan::Plan>),
}

#[derive(Debug, Event, Deserialize, Serialize)]
pub struct Declaration {
    pub parent: PlanParent,
    pub instance_name: String,
    pub edges: Vec<Edge>,
    pub worker_id: Option<Ref<super::worker::Worker>>,
}

#[derive(Entity)]
#[resource_group]
pub struct Plan {
    pub declaration: EmitOnce<Declaration>,
}
