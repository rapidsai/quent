// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A directed edge of a Plan DAG.
#[derive(Debug, Deserialize, Serialize)]
pub struct Edge {
    /// The ID of the port sourcing data.
    pub source: Uuid,
    /// The ID of the port sinking data.
    pub target: Uuid,
}

/// A reference to the parent of Plan.
#[derive(Debug, Deserialize, Serialize)]
pub enum PlanParent {
    /// The parent of this plan is a query, which means this is the source plan.
    Query(Uuid),
    /// This is a nested plan.
    ///
    /// This is useful if an application constructs various types of plans
    /// before execution, sometimes referred to as "lowering". Examples include
    /// a logical and physical plan.
    Plan(Uuid),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlanEvent {
    pub parent: PlanParent,
    pub instance_name: String,
    pub edges: Vec<Edge>,
    pub worker_id: Option<Uuid>,
}
