// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Plan entity: a DAG of operators representing a query execution plan.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct Edge {
    pub source: Uuid,
    pub target: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PlanParent {
    Query(Uuid),
    Plan(Uuid),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Declaration {
    pub parent: PlanParent,
    pub instance_name: String,
    pub edges: Vec<Edge>,
    pub worker_id: Option<Uuid>,
}

#[derive(quent_model::Entity)]
#[resource_group]
pub struct Plan {
    #[event]
    pub declaration: Declaration,
}
