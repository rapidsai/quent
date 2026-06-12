// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use quent_attributes::Value;
use quent_query_engine_ui::{
    self as ui,
    diff::{
        Compatibility, DiffDelta, DiffOperatorDelta, DiffOperatorRef, DiffQuerySummary, QueryDiff,
        QueryStatDiffs,
    },
};
use quent_time::TimeSec;
use uuid::Uuid;

/// Per-query operator data needed to compute a diff.
///
/// Returned by [`UiAnalyzer::query_operator_stats`].
pub struct QueryOperatorStats {
    pub engine_id: Uuid,
    pub instance_name: Option<String>,
    pub query_group_id: Option<Uuid>,
    pub query_group_name: Option<String>,
    pub duration_s: TimeSec,
    /// All operators that worked on this query, keyed by operator ID.
    pub operators: HashMap<Uuid, ui::Operator>,
}

fn to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::U8(n) => Some(*n as f64),
        Value::U16(n) => Some(*n as f64),
        Value::U32(n) => Some(*n as f64),
        Value::U64(n) => Some(*n as f64),
        Value::I8(n) => Some(*n as f64),
        Value::I16(n) => Some(*n as f64),
        Value::I32(n) => Some(*n as f64),
        Value::I64(n) => Some(*n as f64),
        Value::F32(n) => Some(*n as f64),
        Value::F64(n) => Some(*n),
        _ => None,
    }
}

/// Groups operators by type name and sums all numeric stats within each group.
///
/// Returns a map of `operator_type_name → (count, summed_stats)`.
fn aggregate_by_type(
    operators: &HashMap<Uuid, ui::Operator>,
) -> HashMap<String, (usize, HashMap<String, f64>)> {
    let mut by_type: HashMap<String, (usize, HashMap<String, f64>)> = HashMap::new();
    for op in operators.values() {
        if let Some(type_name) = op.operator_type_name.as_deref() {
            let entry = by_type.entry(type_name.to_string()).or_default();
            entry.0 += 1;
            if let Some(stats) = &op.statistics {
                for (key, val) in &stats.custom_statistics {
                    if let Some(v) = val.as_ref().and_then(to_f64) {
                        *entry.1.entry(key.clone()).or_insert(0.0) += v;
                    }
                }
            }
        }
    }
    by_type
}

pub fn compute_diff(
    comparison_query_id: Uuid,
    baseline: &QueryOperatorStats,
    comparison: &QueryOperatorStats,
) -> QueryDiff {
    let summary = DiffQuerySummary {
        id: comparison_query_id,
        engine_id: comparison.engine_id,
        instance_name: comparison.instance_name.clone(),
        query_group_id: comparison.query_group_id,
        query_group_name: comparison.query_group_name.clone(),
    };

    let duration_delta = comparison.duration_s - baseline.duration_s;
    let duration_pct = if baseline.duration_s != 0.0 {
        Some(duration_delta / baseline.duration_s * 100.0)
    } else {
        None
    };
    let stat_diff = QueryStatDiffs {
        duration: DiffDelta {
            stats: (
                Some(Value::F64(baseline.duration_s)),
                Some(Value::F64(comparison.duration_s)),
            ),
            delta: Some(duration_delta),
            percent_delta: duration_pct,
        },
    };

    let baseline_by_type = aggregate_by_type(&baseline.operators);
    let comparison_by_type = aggregate_by_type(&comparison.operators);

    let all_type_names: HashSet<&str> = baseline_by_type
        .keys()
        .map(|k| k.as_str())
        .chain(comparison_by_type.keys().map(|k| k.as_str()))
        .collect();

    let empty: HashMap<String, f64> = HashMap::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut operator_diffs: Vec<DiffOperatorDelta> = Vec::new();

    for type_name in all_type_names {
        let b_entry = baseline_by_type.get(type_name);
        let c_entry = comparison_by_type.get(type_name);

        if b_entry.is_none() {
            warnings.push(format!("operator type '{type_name}' not found in baseline"));
        }
        if c_entry.is_none() {
            warnings.push(format!("operator type '{type_name}' not found in comparison"));
        }

        let (b_count, b_stats) = b_entry.map(|(c, s)| (*c, s)).unwrap_or((0, &empty));
        let (c_count, c_stats) = c_entry.map(|(c, s)| (*c, s)).unwrap_or((0, &empty));

        let all_stat_keys: HashSet<&str> = b_stats
            .keys()
            .map(|k| k.as_str())
            .chain(c_stats.keys().map(|k| k.as_str()))
            .collect();

        let stats: HashMap<String, DiffDelta> = all_stat_keys
            .into_iter()
            .map(|key| {
                let b_val = b_stats.get(key).copied();
                let c_val = c_stats.get(key).copied();

                let (delta, percent_delta) = match (b_val, c_val) {
                    (Some(b), Some(c)) => {
                        let d = c - b;
                        let pct = if b != 0.0 { Some(d / b * 100.0) } else { None };
                        (Some(d), pct)
                    }
                    _ => (None, None),
                };

                (
                    key.to_string(),
                    DiffDelta {
                        stats: (b_val.map(Value::F64), c_val.map(Value::F64)),
                        delta,
                        percent_delta,
                    },
                )
            })
            .collect();

        operator_diffs.push(DiffOperatorDelta {
            operators: (
                DiffOperatorRef {
                    label: type_name.to_string(),
                    operator_type_name: Some(type_name.to_string()),
                    count: b_count,
                },
                DiffOperatorRef {
                    label: type_name.to_string(),
                    operator_type_name: Some(type_name.to_string()),
                    count: c_count,
                },
            ),
            stats,
        });
    }

    QueryDiff {
        compatibility: Compatibility::Compatible,
        query: Some(summary),
        operator_diffs: Some(operator_diffs),
        stat_diffs: Some(stat_diff),
        warnings: if warnings.is_empty() { None } else { Some(warnings) },
    }
}
