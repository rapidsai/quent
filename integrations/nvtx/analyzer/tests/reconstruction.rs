// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Reconstruction of `RangeStart`/`RangeEnd` pairs, and what an incomplete
//! pair yields.

mod fixtures;

use fixtures::{mark, range_end, range_start, span};
use nvtx_analyzer::{NvtxModelBuilder, SpanKind};

#[test]
fn startend_match_by_handle() {
    // The final `RangeEnd` carries a *different* domain than its start: the
    // match key is `range_id` alone, which NVTX makes process-globally unique.
    let events = vec![
        range_start(100, 1, 7, "alpha"),
        range_start(150, 2, 9, "beta"),
        range_end(200, 1, 7),
        range_end(250, 999, 9),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.spans().len(), 2, "one span per matched range");

    let alpha = span(&model, "alpha");
    assert_eq!(alpha.kind, SpanKind::StartEnd);
    assert_eq!(alpha.domain, 1);
    assert_eq!(alpha.start, 100);
    assert_eq!(alpha.end, Some(200), "the end was observed");
    assert_eq!(alpha.duration(), Some(100));

    let beta = span(&model, "beta");
    assert_eq!(beta.kind, SpanKind::StartEnd);
    assert_eq!(beta.domain, 2, "domain comes from the start, not the end");
    assert_eq!(beta.start, 150);
    assert_eq!(beta.end, Some(250));
}

#[test]
fn out_of_order_sorted() {
    // The end arrives before its start.
    let events = vec![range_end(200, 1, 7), range_start(100, 1, 7, "late")];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.spans().len(), 1);
    let late = span(&model, "late");
    assert_eq!(late.start, 100);
    assert_eq!(late.end, Some(200), "the end was observed, just early");
    assert_eq!(late.duration(), Some(100), "replay is timestamp-ordered");
}

#[test]
fn duplicate_timestamps_no_panic() {
    let stream = || {
        vec![
            range_start(100, 1, 7, "a"),
            range_start(100, 1, 8, "b"),
            range_end(100, 1, 7),
            range_end(100, 1, 8),
        ]
    };

    let model = NvtxModelBuilder::build(stream());
    assert_eq!(model.spans().len(), 2);

    for span in model.spans() {
        assert_eq!(span.start, 100);
        assert_eq!(span.end, Some(100), "zero-duration spans are legal");
        assert_eq!(span.duration(), Some(0));
    }

    // Equal timestamps preserve arrival order, so a rebuild is identical.
    let again = NvtxModelBuilder::build(stream());
    assert_eq!(
        model.spans(),
        again.spans(),
        "duplicate timestamps reconstruct deterministically"
    );
}

#[test]
fn unclosed_start_has_no_end() {
    // `leaked` is never ended. The trace runs to 500, but that is where
    // *observation* stopped, not where the range did — so nothing is written
    // into its end. A consumer that wants a right edge takes `trace_end`
    // deliberately.
    let events = vec![range_start(100, 1, 7, "leaked"), mark(500, 1, "tick")];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.spans().len(), 1, "a mark is an instant, not a span");
    let leaked = span(&model, "leaked");
    assert_eq!(leaked.start, 100);
    assert_eq!(leaked.end, None, "no close was ever captured");
    assert_eq!(leaked.duration(), None, "so there is nothing to measure");

    assert_eq!(model.trace_end(), 500, "the bound is on the model instead");
    assert!(
        model.anomalies().is_faithful(),
        "a capture that stopped mid-flight is not malformed; the span says so itself"
    );
}

#[test]
fn restarted_id_keeps_the_displaced_range() {
    // `first` is restarted under the same id before it ever ended. NVTX ids are
    // process-globally unique, so this is malformed — but the first start was
    // observed, and dropping it would erase a range the stream really reported.
    let events = vec![
        range_start(100, 1, 7, "first"),
        range_start(300, 1, 7, "second"),
        range_end(500, 1, 7),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.spans().len(), 2, "both starts produce a span");

    let first = span(&model, "first");
    assert_eq!(first.start, 100);
    assert_eq!(
        first.end, None,
        "the displaced range is kept, but nothing ever closed it"
    );
    assert_eq!(first.duration(), None);

    // The restart is not visible on the span — it looks like any other
    // never-closed range — so the model carries the fact instead.
    assert_eq!(model.anomalies().reused_range_ids, 1);
    assert!(!model.anomalies().is_faithful());

    // The `RangeEnd` belongs to whichever start was open when it arrived.
    let second = span(&model, "second");
    assert_eq!(second.start, 300);
    assert_eq!(second.end, Some(500), "the end was observed");
}

#[test]
fn orphan_end_skipped_but_counted() {
    // A `RangeEnd` with no open start cannot become a span — it carries only a
    // correlation key, no name or attributes. It is dropped, but the drop is
    // reported, because otherwise this model is indistinguishable from one that
    // saw the whole stream.
    let events = vec![
        range_end(100, 1, 7),
        range_start(200, 1, 8, "real"),
        range_end(300, 1, 8),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.spans().len(), 1, "the orphan end produced no span");
    let real = span(&model, "real");
    assert_eq!(real.start, 200);
    assert_eq!(real.end, Some(300));

    let anomalies = model.anomalies();
    assert_eq!(anomalies.orphan_range_ends, 1);
    assert_eq!(anomalies.total(), 1);
    assert!(
        !anomalies.is_faithful(),
        "the span list is not the whole population"
    );
}

#[test]
fn clean_stream_is_faithful() {
    // Every close matched an open, so nothing was lost and the span list
    // accounts for the entire stream.
    let events = vec![range_start(100, 1, 7, "alpha"), range_end(200, 1, 7)];

    let model = NvtxModelBuilder::build(events);

    assert!(model.anomalies().is_faithful());
    assert_eq!(model.anomalies().total(), 0);
}
