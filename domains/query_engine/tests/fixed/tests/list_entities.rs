// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Functional test of `UiAnalyzer::list_entities` over the fixed scenario.
//!
//! Captures the fixed 7-second telemetry in memory via a callback exporter,
//! builds a `SimulatorUiAnalyzer` from it, and asserts the entity-list query
//! against the scenario's known tasks and their resource usage.
//!
//! Ground truth: every task holds its memory for the `computing→exit` span
//! (0.75s). `MEMORY_W0` is used by 8 tasks, `MEMORY_W1` by 4. Because the spans
//! are equal, results are ordered by the UUID tiebreaker.

use quent_model::EventCallback;
use quent_query_engine_analyzer::ui::UiAnalyzer;
use quent_query_engine_fixed as fixed;
use quent_query_engine_ui::{OperatorFilter, QueryFilter};
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use quent_simulator_instrumentation::SimulatorContext;
use quent_ui::entities::request::{
    EntityListEntry, EntityListFilter, EntityListRequest, EntityScope, EntitySortKey, Sort,
    SortDir, TimeWindow,
};
use quent_ui::entities::response::EntityListResponse;
use quent_ui::paginate::PageParams;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Tasks using MEMORY_W0, in ascending UUID order (the tiebreaker).
const MEMORY_W0_TASKS: [Uuid; 8] = [
    fixed::TASK_0,
    fixed::TASK_1,
    fixed::TASK_4,
    fixed::TASK_5,
    fixed::TASK_8,
    fixed::TASK_9,
    fixed::TASK_10,
    fixed::TASK_11,
];

// Tasks using MEMORY_W1, in ascending UUID order.
const MEMORY_W1_TASKS: [Uuid; 4] = [fixed::TASK_2, fixed::TASK_3, fixed::TASK_6, fixed::TASK_7];

// All 12 tasks ranked by longest usage, descending. TASK_6 and TASK_7 sort
// last: their `computing` is cut short by a `sending` transition, so their
// longest single usage span is 0.5s (the send) versus 0.75s for every other
// task. The remaining ten tie at 0.75s and fall back to ascending UUID order.
const ALL_TASKS_RANKED: [Uuid; 12] = [
    fixed::TASK_0,
    fixed::TASK_1,
    fixed::TASK_2,
    fixed::TASK_3,
    fixed::TASK_4,
    fixed::TASK_5,
    fixed::TASK_8,
    fixed::TASK_9,
    fixed::TASK_10,
    fixed::TASK_11,
    fixed::TASK_6,
    fixed::TASK_7,
];

/// Emit the fixed scenario into memory via a callback exporter and build an
/// analyzer from the captured events.
fn fixed_analyzer() -> SimulatorUiAnalyzer {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    {
        let captured = Arc::clone(&recorded);
        let ctx = SimulatorContext::try_new(EventCallback::new(move |event| {
            captured.lock().unwrap().push(event);
        }))
        .unwrap();
        fixed::emit(&ctx);
        // ctx dropped here, flushing all events to the callback.
    }

    let events = std::mem::take(&mut *recorded.lock().unwrap());
    SimulatorUiAnalyzer::try_new(fixed::ENGINE, events.into_iter()).unwrap()
}

/// An entity-list entry over the whole query window, ranked by usage duration.
fn entry(
    scope: Option<EntityScope>,
    min_usage_s: Option<f64>,
    page: Option<PageParams>,
    operator_ids: Vec<Uuid>,
) -> EntityListEntry<OperatorFilter> {
    EntityListEntry {
        window: TimeWindow {
            start: 0.0,
            end: 7.0,
        },
        filter: EntityListFilter {
            scope,
            entity_type_name: None,
            min_usage_s,
        },
        sort: Sort {
            key: EntitySortKey::UsageDuration,
            dir: SortDir::Desc,
        },
        page,
        application: OperatorFilter { operator_ids },
    }
}

fn request_scoped(
    scope: Option<EntityScope>,
    min_usage_s: Option<f64>,
    page: Option<PageParams>,
) -> EntityListRequest<QueryFilter, OperatorFilter> {
    EntityListRequest {
        entry: entry(scope, min_usage_s, page, Vec::new()),
        app_params: QueryFilter {
            query_id: fixed::QUERY,
        },
    }
}

/// A request scoped to a single resource.
fn request(
    resource_id: Uuid,
    min_usage_s: Option<f64>,
    page: Option<PageParams>,
) -> EntityListRequest<QueryFilter, OperatorFilter> {
    request_scoped(
        Some(EntityScope::Resource { resource_id }),
        min_usage_s,
        page,
    )
}

fn ids(resp: &EntityListResponse) -> Vec<Uuid> {
    resp.items.iter().map(|item| item.entity.id).collect()
}

#[test]
fn lists_all_tasks_on_a_resource_ranked_by_uuid_tiebreak() {
    let analyzer = fixed_analyzer();
    let resp = analyzer
        .list_entities(request(fixed::MEMORY_W0, None, None))
        .unwrap();

    assert_eq!(resp.total, 8);
    assert_eq!(ids(&resp), MEMORY_W0_TASKS);
    assert!(resp.items.iter().all(|item| item.usage_duration_s == 0.75));
}

#[test]
fn no_scope_lists_every_entity() {
    let analyzer = fixed_analyzer();
    let resp = analyzer
        .list_entities(request_scoped(None, None, None))
        .unwrap();

    // Every task is ranked regardless of which resource it used.
    assert_eq!(resp.total, 12);
    assert_eq!(ids(&resp), ALL_TASKS_RANKED);
}

#[test]
fn scope_restricts_to_the_resources_tasks() {
    let analyzer = fixed_analyzer();
    let resp = analyzer
        .list_entities(request(fixed::MEMORY_W1, None, None))
        .unwrap();

    assert_eq!(resp.total, 4);
    assert_eq!(ids(&resp), MEMORY_W1_TASKS);
}

#[test]
fn min_usage_filter_includes_or_excludes_by_threshold() {
    let analyzer = fixed_analyzer();

    // Each task's memory usage is 0.75s; a lower threshold keeps all.
    let kept = analyzer
        .list_entities(request(fixed::MEMORY_W0, Some(0.5), None))
        .unwrap();
    assert_eq!(kept.total, 8);

    // A threshold above 0.75s drops every task.
    let dropped = analyzer
        .list_entities(request(fixed::MEMORY_W0, Some(1.0), None))
        .unwrap();
    assert_eq!(dropped.total, 0);
    assert!(dropped.items.is_empty());
}

#[test]
fn pagination_slices_the_ranked_set_with_stable_total() {
    let analyzer = fixed_analyzer();

    let page0 = analyzer
        .list_entities(request(
            fixed::MEMORY_W0,
            None,
            Some(PageParams { max: 3, page: 0 }),
        ))
        .unwrap();
    assert_eq!(page0.total, 8);
    assert_eq!(ids(&page0), MEMORY_W0_TASKS[0..3]);

    let page2 = analyzer
        .list_entities(request(
            fixed::MEMORY_W0,
            None,
            Some(PageParams { max: 3, page: 2 }),
        ))
        .unwrap();
    assert_eq!(page2.total, 8);
    assert_eq!(ids(&page2), MEMORY_W0_TASKS[6..8]);
}

#[test]
fn unscoped_window_excludes_entities_whose_span_never_overlaps() {
    let analyzer = fixed_analyzer();

    // The window is relative to the query's own epoch (1.0s absolute in this
    // fixture). No task's lifecycle (first event to last event) reaches
    // [6.5, 7.0) relative (7.5-8.0s absolute): the latest event of any task is
    // TASK_10/TASK_11's exit at 6.0s absolute (5.0s relative). With no scope
    // and no min-usage filter, the window alone must still exclude entities
    // whose lifecycle never overlaps it.
    let request = EntityListRequest {
        entry: EntityListEntry {
            window: TimeWindow {
                start: 6.5,
                end: 7.0,
            },
            ..entry(None, None, None, Vec::new())
        },
        app_params: QueryFilter {
            query_id: fixed::QUERY,
        },
    };
    let resp = analyzer.list_entities(request).unwrap();

    assert_eq!(resp.total, 0);
    assert!(resp.items.is_empty());
}

#[test]
fn unscoped_window_includes_entities_whose_span_overlaps() {
    let analyzer = fixed_analyzer();

    // The window is relative to the query's own epoch (its `init` transition,
    // at 1.0s absolute in this fixture). TASK_0..3 run 2.0-3.0s absolute, i.e.
    // [1.0, 2.0) relative; every other task's lifecycle starts at 3.0s
    // absolute (2.0s relative) or later, so only TASK_0..3 overlap [1.0, 1.3).
    let request = EntityListRequest {
        entry: EntityListEntry {
            window: TimeWindow {
                start: 1.0,
                end: 1.3,
            },
            ..entry(None, None, None, Vec::new())
        },
        app_params: QueryFilter {
            query_id: fixed::QUERY,
        },
    };
    let resp = analyzer.list_entities(request).unwrap();

    let mut got = ids(&resp);
    got.sort();
    let mut want = vec![fixed::TASK_0, fixed::TASK_1, fixed::TASK_2, fixed::TASK_3];
    want.sort();
    assert_eq!(resp.total, 4);
    assert_eq!(got, want);
}

#[test]
fn unscoped_window_includes_entities_even_without_an_event_inside_it() {
    let analyzer = fixed_analyzer();

    // TASK_4..7 (PartialAggregate) run 3.0-4.0s absolute (2.0-3.0s relative).
    // A window strictly between their `computing`/`sending` events and their
    // `exit` contains none of their individual event timestamps, but their
    // overall lifecycle still overlaps it, so they must still be included —
    // this is the exact case the window filter previously got wrong.
    let request = EntityListRequest {
        entry: EntityListEntry {
            window: TimeWindow {
                start: 2.6,
                end: 2.7,
            },
            ..entry(None, None, None, Vec::new())
        },
        app_params: QueryFilter {
            query_id: fixed::QUERY,
        },
    };
    let resp = analyzer.list_entities(request).unwrap();

    let mut got = ids(&resp);
    got.sort();
    let mut want = vec![fixed::TASK_4, fixed::TASK_5, fixed::TASK_6, fixed::TASK_7];
    want.sort();
    assert_eq!(resp.total, 4);
    assert_eq!(got, want);
}

#[test]
fn operator_filter_restricts_to_one_operator_tasks() {
    let analyzer = fixed_analyzer();

    // ScanFilter_W0 runs exactly TASK_0 and TASK_1.
    let request = EntityListRequest {
        entry: entry(None, None, None, vec![fixed::PHYS_SCAN_FILTER_W0]),
        app_params: QueryFilter {
            query_id: fixed::QUERY,
        },
    };
    let resp = analyzer.list_entities(request).unwrap();

    assert_eq!(resp.total, 2);
    assert_eq!(ids(&resp), [fixed::TASK_0, fixed::TASK_1]);
}

#[test]
fn operator_filter_combines_multiple_operators_tasks() {
    let analyzer = fixed_analyzer();

    let request = EntityListRequest {
        entry: entry(
            None,
            None,
            None,
            vec![fixed::PHYS_SCAN_FILTER_W0, fixed::PHYS_SCAN_FILTER_W1],
        ),
        app_params: QueryFilter {
            query_id: fixed::QUERY,
        },
    };
    let resp = analyzer.list_entities(request).unwrap();

    assert_eq!(resp.total, 4);
    assert_eq!(
        ids(&resp),
        [fixed::TASK_0, fixed::TASK_1, fixed::TASK_2, fixed::TASK_3]
    );
}
