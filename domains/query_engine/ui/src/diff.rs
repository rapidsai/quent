// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use quent_attributes::Value;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// Identifies a single query within a specific engine.
#[derive(TS, Debug, Deserialize)]
pub struct DiffQueryRef {
    pub engine_id: Uuid,
    pub query_id: Uuid,
}

/// Request body for a workload diff.
#[derive(TS, Debug, Deserialize)]
pub struct DiffRequest {
    pub baseline_query: DiffQueryRef,
    pub comparison_queries: Vec<DiffQueryRef>,
}

/// Whether two queries are structurally comparable (i.e. same plan shape).
#[derive(TS, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum Compatibility {
    Compatible,
    Incompatible,
}

/// Summary metadata for a query included in a diff result.
#[derive(TS, Debug, Serialize)]
pub struct DiffQuerySummary {
    pub id: Uuid,
    pub engine_id: Uuid,
    pub instance_name: Option<String>,
    pub query_group_id: Option<Uuid>,
    pub query_group_name: Option<String>,
}

/// A reference to an operator type in a diff, aggregated across all operators of that type.
#[derive(TS, Debug, Serialize)]
pub struct DiffOperatorRef {
    pub label: String,
    pub operator_type_name: Option<String>,
    /// Number of operators of this type that were aggregated.
    pub count: usize,
}

/// The raw values and computed delta for a single stat across two queries.
///
/// `stats.0` is the baseline value, `stats.1` is the comparison value.
#[derive(TS, Debug, Serialize)]
pub struct DiffDelta {
    pub stats: (Option<Value>, Option<Value>),
    pub delta: Option<f64>,
    pub percent_delta: Option<f64>,
}

/// Stat deltas for a matched pair of operators (one from each query).
#[derive(TS, Debug, Serialize)]
pub struct DiffOperatorDelta {
    /// `operators.0` is from the baseline, `operators.1` is from the comparison query.
    pub operators: (DiffOperatorRef, DiffOperatorRef),
    /// Keyed by stat name.
    pub stats: HashMap<String, DiffDelta>,
}

/// Query-level stat deltas (derived from query timestamps).
#[derive(TS, Debug, Serialize)]
pub struct QueryStatDiffs {
    pub duration: DiffDelta,
}

/// The diff result for a single comparison query against the baseline.
#[derive(TS, Debug, Serialize)]
pub struct QueryDiff {
    pub compatibility: Compatibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<DiffQuerySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_diffs: Option<Vec<DiffOperatorDelta>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stat_diffs: Option<QueryStatDiffs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

/// Response body for a workload diff.
///
/// One [`QueryDiff`] per entry in `DiffRequest.comparison_queries`, in the same order.
#[derive(TS, Debug, Serialize)]
pub struct DiffResponse {
    pub comparison_queries: Vec<QueryDiff>,
}
