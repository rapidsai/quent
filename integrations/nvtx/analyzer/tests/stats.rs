// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Range statistics (ANA-06) — the aggregation and, just as importantly, what it
//! refuses to aggregate.
//!
//! The grouping key is `(name, domain, category)`, and each component is pinned
//! by a test that would still pass if the key were only *partly* right: two
//! same-named ranges in different domains, and two same-named ranges under
//! different categories, must not merge. Marks and resource lifespans must not
//! appear at all — folding them in would yield a number that reads like a range
//! duration but answers a different question.

mod fixtures;

use fixtures::{
    mark, range_pop, range_push, range_push_in_category, resource_create, resource_destroy,
};
use nvtx_analyzer::{NvtxModelBuilder, RangeStats, StatsKey};

/// `NVTX_RESOURCE_TYPE_GENERIC_POINTER`, for the exclusion fixture.
const GENERIC_POINTER: i32 = 0x0001_0001;

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

    let model = NvtxModelBuilder::build(events).expect("build");
    let stats = model.range_statistics();

    let work = stats.get(&key("work", 1)).expect("no stats for work@1");
    assert_eq!(
        *work,
        RangeStats {
            count: 3,
            total_duration: 450,
            avg_duration: 150,
            min_duration: 50,
            max_duration: 300,
            synthetic_count: 0,
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

    let model = NvtxModelBuilder::build(events).expect("build");
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

    let model = NvtxModelBuilder::build(events).expect("build");
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
fn range_statistics_tracks_synthetic_separately() {
    // One observed close and one leaked push closed at trace end (900).
    let events = vec![
        range_push(100, 1, 1, "work"),
        range_pop(200, 1, 1),
        range_push(300, 1, 2, "work"),
        mark(900, 1, "trace end"),
    ];

    let model = NvtxModelBuilder::build(events).expect("build");
    let stats = model.range_statistics();

    let work = stats.get(&key("work", 1)).expect("work");
    assert_eq!(work.count, 2, "the synthetic close still counts");
    assert_eq!(work.synthetic_count, 1, "but is separately identifiable");
    assert_eq!(work.total_duration, 100 + 600);
    assert_eq!(work.max_duration, 600);
    assert_eq!(work.min_duration, 100);
}

#[test]
fn range_statistics_zero_duration_and_empty() {
    // A zero-length range contributes 0 rather than being dropped, and a stream
    // with no ranges at all yields no groups (never a division by zero).
    let model = NvtxModelBuilder::build(vec![mark(100, 1, "only a mark")]).expect("build");
    assert!(model.range_statistics().is_empty());

    let events = vec![range_push(100, 1, 1, "instant"), range_pop(100, 1, 1)];
    let model = NvtxModelBuilder::build(events).expect("build");
    let stats = model.range_statistics();

    let instant = stats.get(&key("instant", 1)).expect("instant");
    assert_eq!(
        *instant,
        RangeStats {
            count: 1,
            total_duration: 0,
            avg_duration: 0,
            min_duration: 0,
            max_duration: 0,
            synthetic_count: 0,
        }
    );
}
