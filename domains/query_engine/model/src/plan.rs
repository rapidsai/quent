// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Plan entity: a DAG of operators representing a query execution plan.

use quent_model::{Attributes, Ref, entity};
use serde::{Deserialize, Serialize};

/// A directed edge of a Plan DAG.
#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct Edge {
    /// The ID of the port sourcing data.
    pub source: Ref<super::port::Port>,
    /// The ID of the port sinking data.
    pub target: Ref<super::port::Port>,
}

/// A reference to the parent of a Plan. Exactly one field should be set.
/// If `query_id` is set, this is a root plan under a query.
/// If `plan_id` is set, this is a nested plan (e.g. logical → physical).
#[derive(Debug, Default, Attributes, Deserialize, Serialize)]
pub struct PlanParent {
    pub query_id: Option<Ref<super::query::Query>>,
    pub plan_id: Option<Ref<super::plan::Plan>>,
}

#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct Declaration {
    pub parent: PlanParent,
    pub instance_name: String,
    pub edges: Vec<Edge>,
    pub worker_id: Option<Ref<super::worker::Worker>>,
}

entity! {
    Plan: ResourceGroup {
        declaration: declaration,
        events: {
            declaration: Declaration,
        },
    }
}
