// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Functional test of `UiAnalyzer::data_flow_timeline` over the fixed scenario.
//!
//! Ground truth (times relative to the query epoch at 1s absolute): each task
//! queues + allocates at its slot start, computes from +250ms, and exits at
//! the slot end. `PHYS_SCAN_FILTER_W0` runs TASK_0 and TASK_1 in its 1–2s
//! slot; `PHYS_PARTIAL_AGG_W1` runs TASK_6 and TASK_7 in its 2–3s slot with a
//! `sending` state from +500ms. Computing holds 256 bytes of the worker's
//! "memory" resource; allocating and sending hold no memory.

use quent_io::{EventCallback, ExporterOptions};
use quent_query_engine_analyzer::ui::UiAnalyzer;
use quent_query_engine_fixed as fixed;
use quent_query_engine_ui::DataFlowTimelineBinned;
use quent_query_engine_ui::QueryFilter;
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use quent_simulator_instrumentation::{SimulatorContext, test_utils::events_from_recorded};
use quent_ui::timeline::{categorical::CategoricalTimelineRequest, request::TimelineConfig};
use std::sync::{Arc, Mutex};

/// Emit the fixed scenario into memory via a callback exporter and build an
/// analyzer from the captured events.
fn fixed_analyzer() -> SimulatorUiAnalyzer {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    {
        let captured = Arc::clone(&recorded);
        let ctx = SimulatorContext::try_new(Some(ExporterOptions::Callback(EventCallback::new(
            move |event| captured.lock().unwrap().push(event),
        ))))
        .unwrap();
        fixed::emit(&ctx);
        // ctx dropped here, flushing all events to the callback.
    }

    let events = events_from_recorded(std::mem::take(&mut *recorded.lock().unwrap()));
    SimulatorUiAnalyzer::try_new(fixed::ENGINE, events.into_iter()).unwrap()
}

/// A whole-query request: 7 one-second bins over the 0–7s window.
fn request(measures: &[&str]) -> CategoricalTimelineRequest<QueryFilter> {
    CategoricalTimelineRequest {
        measures: measures.iter().map(|m| m.to_string()).collect(),
        config: TimelineConfig {
            num_bins: 7,
            start: 0.0,
            end: 7.0,
        },
        app_params: QueryFilter {
            query_id: fixed::QUERY,
        },
    }
}

fn bins<'a>(
    binned: &'a DataFlowTimelineBinned,
    operator: uuid::Uuid,
    measure: &str,
    state: &str,
    dimension: &str,
) -> Option<&'a Vec<f64>> {
    binned
        .operators
        .get(&operator)?
        .values
        .get(measure)?
        .get(state)?
        .get(dimension)
}

#[test]
fn declares_states_dimensions_and_measures() {
    let analyzer = fixed_analyzer();
    let result = analyzer.data_flow_timeline(request(&[])).unwrap();

    assert_eq!(result.decl.entity_type_name, "task");
    assert_eq!(result.decl.dimension_name, "Data location");
    // Both workers' memory resources share the instance name "memory"; the
    // no-memory key comes last.
    assert_eq!(
        result
            .decl
            .dimension_keys
            .iter()
            .map(|k| k.key.as_str())
            .collect::<Vec<_>>(),
        ["memory", "none"]
    );
    assert_eq!(
        result
            .decl
            .measures
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        ["tasks", "bytes"]
    );
    assert_eq!(result.config.num_bins, 7);
}

#[test]
fn distributes_scan_filter_tasks_over_states_and_locations() {
    let analyzer = fixed_analyzer();
    let result = analyzer.data_flow_timeline(request(&[])).unwrap();

    // Two tasks allocating (no memory) for 0.25s each within bin 1.
    assert_eq!(
        bins(
            &result,
            fixed::PHYS_SCAN_FILTER_W0,
            "tasks",
            "allocating",
            "none"
        )
        .unwrap()[..],
        [0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0]
    );
    // Two tasks computing in memory for 0.75s each within bin 1.
    assert_eq!(
        bins(
            &result,
            fixed::PHYS_SCAN_FILTER_W0,
            "tasks",
            "computing",
            "memory"
        )
        .unwrap()[..],
        [0.0, 1.5, 0.0, 0.0, 0.0, 0.0, 0.0]
    );
    // 2 tasks x 256 bytes x 0.75 bin fraction.
    assert_eq!(
        bins(
            &result,
            fixed::PHYS_SCAN_FILTER_W0,
            "bytes",
            "computing",
            "memory"
        )
        .unwrap()[..],
        [0.0, 384.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    );
    // Queueing is zero-duration in this scenario: filtered out as all-zero.
    assert!(
        bins(
            &result,
            fixed::PHYS_SCAN_FILTER_W0,
            "tasks",
            "queueing",
            "none"
        )
        .is_none()
    );
}

#[test]
fn sending_state_counts_without_memory_location() {
    let analyzer = fixed_analyzer();
    let result = analyzer.data_flow_timeline(request(&[])).unwrap();

    // TASK_6/TASK_7: allocating 2.0-2.25, computing 2.25-2.5, sending 2.5-3.0.
    assert_eq!(
        bins(
            &result,
            fixed::PHYS_PARTIAL_AGG_W1,
            "tasks",
            "allocating",
            "none"
        )
        .unwrap()[..],
        [0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0]
    );
    assert_eq!(
        bins(
            &result,
            fixed::PHYS_PARTIAL_AGG_W1,
            "tasks",
            "computing",
            "memory"
        )
        .unwrap()[..],
        [0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0]
    );
    // The channel usage during sending is not a memory resource: location "none".
    assert_eq!(
        bins(
            &result,
            fixed::PHYS_PARTIAL_AGG_W1,
            "tasks",
            "sending",
            "none"
        )
        .unwrap()[..],
        [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]
    );
    assert_eq!(
        bins(
            &result,
            fixed::PHYS_PARTIAL_AGG_W1,
            "bytes",
            "computing",
            "memory"
        )
        .unwrap()[..],
        [0.0, 0.0, 128.0, 0.0, 0.0, 0.0, 0.0]
    );
}

#[test]
fn measures_filter_restricts_response_and_decl() {
    let analyzer = fixed_analyzer();
    let result = analyzer.data_flow_timeline(request(&["tasks"])).unwrap();

    assert_eq!(
        result
            .decl
            .measures
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        ["tasks"]
    );
    assert!(
        bins(
            &result,
            fixed::PHYS_SCAN_FILTER_W0,
            "tasks",
            "computing",
            "memory"
        )
        .is_some()
    );
    assert!(
        bins(
            &result,
            fixed::PHYS_SCAN_FILTER_W0,
            "bytes",
            "computing",
            "memory"
        )
        .is_none()
    );
}

#[test]
fn unknown_measures_are_an_error() {
    let analyzer = fixed_analyzer();
    assert!(analyzer.data_flow_timeline(request(&["bogus"])).is_err());
    // A typo alongside valid measures must not be silently ignored.
    assert!(
        analyzer
            .data_flow_timeline(request(&["tasks", "bogus"]))
            .is_err()
    );
}
