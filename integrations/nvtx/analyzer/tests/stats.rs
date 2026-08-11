// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Range statistics — the aggregation, and what it refuses to aggregate.
//!
//! Each component of the `(name, domain, category)` key is pinned by a test
//! that would still pass if the key were only partly right, and marks and
//! resource lifespans are pinned as excluded.

mod fixtures;

use fixtures::{
    GENERIC_POINTER, mark, range_end, range_pop, range_push, range_push_in_category, range_start,
    resource_create, resource_destroy,
};
use nvtx_analyzer::{NvtxModelBuilder, RangeStats, StatsKey};

/// The key for an uncategorized range named `name` in `domain`.
fn key(name: &str, domain: u64) -> StatsKey {
    StatsKey {
        name: name.to_owned(),
        domain,
        category: None,
    }
}

#[test]
fn range_statistics() {
    // Three "work" ranges of 100 / 50 / 300 ns, plus one differently-named
    // range and one same-named range in another domain — neither may merge in.
    let events = vec![
        range_push(100, 1, 1, "work"),
        range_pop(200, 1, 1),
        range_push(300, 1, 1, "work"),
        range_pop(350, 1, 1),
        range_push(400, 1, 1, "work"),
        range_pop(700, 1, 1),
        range_push(800, 1, 1, "other"),
        range_pop(900, 1, 1),
        // Same name, different domain: the domain is part of the key.
        range_push(1000, 2, 1, "work"),
        range_pop(1100, 2, 1),
    ];

    let model = NvtxModelBuilder::build(events);
    let stats = model.range_statistics();

    let work = stats.get(&key("work", 1)).expect("no stats for work@1");
    assert_eq!(
        *work,
        RangeStats {
            count: 3,
            observed_count: 3,
            total_duration: 450,
            avg_duration: 150,
            min_duration: 50,
            max_duration: 300,
            saturated: false,
        }
    );

    assert_eq!(
        stats
            .get(&key("work", 2))
            .expect("no stats for work@2")
            .count,
        1,
        "a same-named range in another domain is its own group"
    );
    assert_eq!(
        stats
            .get(&key("other", 1))
            .expect("no stats for other@1")
            .count,
        1
    );
    assert_eq!(stats.len(), 3, "exactly three groups");
}

#[test]
fn range_statistics_namespaced_by_category() {
    // One name, one domain, two categories. A key that dropped the category
    // would report a single group of two.
    let events = vec![
        range_push_in_category(100, 1, 1, 7, "work"),
        range_pop(200, 1, 1),
        range_push_in_category(300, 1, 1, 9, "work"),
        range_pop(500, 1, 1),
    ];

    let model = NvtxModelBuilder::build(events);
    let stats = model.range_statistics();

    assert_eq!(stats.len(), 2, "category is part of the grouping key");
    for (category, expected) in [(7, 100), (9, 200)] {
        let stats = stats
            .get(&StatsKey {
                name: "work".to_owned(),
                domain: 1,
                category: Some(category),
            })
            .unwrap_or_else(|| panic!("no stats for category {category}"));
        assert_eq!(stats.count, 1);
        assert_eq!(stats.total_duration, expected);
    }
}

#[test]
fn range_statistics_excludes_marks_and_resources() {
    // A mark is an instant and a resource lifespan is not work; only the range
    // may be aggregated.
    let events = vec![
        range_push(100, 1, 1, "work"),
        range_pop(200, 1, 1),
        mark(150, 1, "checkpoint"),
        resource_create(120, 1, 0xAB, GENERIC_POINTER, 0, "buf"),
        resource_destroy(900, 0xAB),
    ];

    let model = NvtxModelBuilder::build(events);
    let stats = model.range_statistics();

    assert_eq!(
        stats.len(),
        1,
        "only the range is aggregated; got {:?}",
        stats.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        stats.get(&key("work", 1)).expect("work").total_duration,
        100
    );
}

#[test]
fn range_statistics_excludes_unobserved_from_durations() {
    // One observed close and one push left open when the trace ended at 900.
    let events = vec![
        range_push(100, 1, 1, "work"),
        range_pop(200, 1, 1),
        range_push(300, 1, 2, "work"),
        mark(900, 1, "trace end"),
    ];

    let model = NvtxModelBuilder::build(events);
    let stats = model.range_statistics();

    let work = stats.get(&key("work", 1)).expect("work");
    assert_eq!(work.count, 2, "both ranges are in the group");
    assert_eq!(work.observed_count, 1, "but only one close was captured");
    assert_eq!(
        work.count - work.observed_count,
        1,
        "the gap between the counts is what says so"
    );

    // The open range had run 600ns by the clock when capture stopped, but that
    // is a lower bound on work still in flight. Folding it in used to report
    // max=600 and avg=350 for a group whose only measurement is 100.
    assert_eq!(work.total_duration, 100, "observed durations only");
    assert_eq!(work.avg_duration, 100);
    assert_eq!(work.max_duration, 100);
    assert_eq!(work.min_duration, 100);
}

#[test]
fn range_statistics_all_unobserved_reports_no_durations() {
    // Every range in the group was left open. There is no measurement to
    // report, and `avg` must not divide by zero.
    let events = vec![
        range_push(100, 1, 1, "work"),
        range_push(300, 1, 2, "work"),
        mark(900, 1, "trace end"),
    ];

    let model = NvtxModelBuilder::build(events);
    let stats = model.range_statistics();

    let work = stats.get(&key("work", 1)).expect("work");
    assert_eq!(work.count, 2);
    assert_eq!(work.observed_count, 0);
    assert_eq!(work.total_duration, 0, "nothing was measured");
    assert_eq!(work.avg_duration, 0, "no division by zero");
    assert!(!work.saturated);
}

#[test]
fn displacement_is_reported_on_the_model_not_the_group() {
    // A restarted range id leaves the displaced range with no end, which reads
    // in the group exactly like a capture that stopped. The two are different
    // facts, so the one the group cannot express is on the model.
    let events = vec![
        range_start(100, 1, 7, "work"),
        range_start(300, 1, 7, "work"),
        range_end(500, 1, 7),
    ];

    let model = NvtxModelBuilder::build(events);
    let stats = model.range_statistics();

    let work = stats.get(&key("work", 1)).expect("work");
    assert_eq!(work.count, 2);
    assert_eq!(work.observed_count, 1, "only the second range closed");
    assert_eq!(work.total_duration, 200, "500 - 300, the observed pair");

    assert_eq!(
        model.anomalies().reused_range_ids,
        1,
        "the id reuse is a property of the stream, not of this group"
    );
}

#[test]
fn range_statistics_flags_a_saturated_total() {
    // Two spans whose durations sum past `u64::MAX`. The total stops being a
    // sum, and `saturated` is what says so — otherwise `u64::MAX` reads as a
    // real measurement.
    let events = vec![
        range_push(0, 1, 1, "huge"),
        range_pop(u64::MAX, 1, 1),
        range_push(0, 1, 2, "huge"),
        range_pop(u64::MAX, 1, 2),
    ];

    let model = NvtxModelBuilder::build(events);
    let stats = model.range_statistics();

    let huge = stats.get(&key("huge", 1)).expect("huge");
    assert_eq!(huge.observed_count, 2);
    assert_eq!(huge.total_duration, u64::MAX);
    assert!(huge.saturated, "the total is capped, not summed");
    assert_eq!(huge.max_duration, u64::MAX, "each span on its own is exact");
}

#[test]
fn range_statistics_where_covers_only_the_kept_spans() {
    // A windowed view must report figures for the spans it is showing. The
    // whole-model fold would describe a different population than the chart.
    let events = vec![
        range_push(100, 1, 1, "work"),
        range_pop(200, 1, 1),
        range_push(1000, 1, 1, "work"),
        range_pop(1300, 1, 1),
    ];

    let model = NvtxModelBuilder::build(events);

    let whole = model.range_statistics();
    assert_eq!(whole.get(&key("work", 1)).expect("work").count, 2);

    // Keep only what overlaps [0, 500).
    let windowed = model.range_statistics_where(|span| span.start < 500);
    let work = windowed.get(&key("work", 1)).expect("work");
    assert_eq!(work.count, 1);
    assert_eq!(work.total_duration, 100);
    assert_eq!(work.max_duration, 100, "the 300ns range is out of window");
}

#[test]
fn trace_bounds_are_not_the_span_extremes() {
    // The last event is a mark, and the first range never ends at all because
    // its id was reused. Neither bound is recoverable from the spans.
    let events = vec![
        range_start(100, 1, 7, "first"),
        range_start(300, 1, 7, "second"),
        range_end(400, 1, 7),
        mark(900, 1, "after everything closed"),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.trace_start(), 100);
    assert_eq!(
        model.trace_end(),
        900,
        "observation stopped at the mark, past the last span end"
    );

    let last_span_end = model.spans().iter().filter_map(|span| span.end).max();
    assert_eq!(last_span_end, Some(400));
    assert_ne!(
        Some(model.trace_end()),
        last_span_end,
        "the largest span end is not a substitute for the trace end"
    );
}

#[test]
fn trace_bounds_of_an_empty_capture() {
    let model = NvtxModelBuilder::build(vec![]);
    assert_eq!(model.trace_start(), 0);
    assert_eq!(model.trace_end(), 0);
}

#[test]
fn range_statistics_zero_duration_and_empty() {
    // A zero-length range contributes 0 rather than being dropped, and a stream
    // with no ranges at all yields no groups (never a division by zero).
    let model = NvtxModelBuilder::build(vec![mark(100, 1, "only a mark")]);
    assert!(model.range_statistics().is_empty());

    let events = vec![range_push(100, 1, 1, "instant"), range_pop(100, 1, 1)];
    let model = NvtxModelBuilder::build(events);
    let stats = model.range_statistics();

    let instant = stats.get(&key("instant", 1)).expect("instant");
    assert_eq!(
        *instant,
        RangeStats {
            count: 1,
            observed_count: 1,
            total_duration: 0,
            avg_duration: 0,
            min_duration: 0,
            max_duration: 0,
            saturated: false,
        }
    );
}
