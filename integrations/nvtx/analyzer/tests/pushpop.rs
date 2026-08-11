// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Per-`(thread_id, domain)` nested Push/Pop reconstruction.
//!
//! A global stack would reconstruct these streams into plausible-but-wrong
//! nesting rather than failing visibly, so `pushpop_nested_per_thread` is the
//! load-bearing test here.

mod fixtures;

use fixtures::{mark, range_end, range_pop, range_push, range_start, span, span_id};
use nvtx_analyzer::{NvtxModelBuilder, SpanKind};

#[test]
fn pushpop_nested_per_thread() {
    // Two threads interleaved in one timestamp-ordered stream. Thread 1 nests
    // `c` inside `a`; thread 2's `b` spans across both of thread 1's pops.
    // A global stack would pop `b` when thread 1 popped `c`.
    let events = vec![
        range_push(100, 1, 1, "a"),
        range_push(110, 1, 2, "b"),
        range_push(120, 1, 1, "c"),
        range_pop(130, 1, 1),
        range_pop(140, 1, 1),
        range_pop(150, 1, 2),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.spans().len(), 3, "one span per matched push/pop pair");

    let a = span(&model, "a");
    assert_eq!(
        a.kind,
        SpanKind::PushPop {
            thread_id: 1,
            parent: None
        },
        "outermost on its thread"
    );
    assert_eq!(a.start, 100);
    assert_eq!(
        a.end,
        Some(140),
        "thread 1's outer push closed on its second pop"
    );

    let c = span(&model, "c");
    assert_eq!(
        c.kind,
        SpanKind::PushPop {
            thread_id: 1,
            parent: Some(span_id(&model, "a"))
        },
        "nested inside the enclosing push on the same thread"
    );
    assert_eq!(c.start, 120);
    assert_eq!(c.end, Some(130), "the first pop closed the innermost push");

    let b = span(&model, "b");
    assert_eq!(
        b.kind,
        SpanKind::PushPop {
            thread_id: 2,
            parent: None
        },
        "a push on another thread is never a parent — the stacks are independent"
    );
    assert_eq!(b.start, 110);
    assert_eq!(
        b.end,
        Some(150),
        "thread 2's push is closed only by thread 2's pop"
    );
}

#[test]
fn pushpop_single_thread() {
    // The plain nesting case: one thread, one domain, balanced.
    let events = vec![
        range_push(100, 7, 42, "outer"),
        range_push(110, 7, 42, "middle"),
        range_push(120, 7, 42, "inner"),
        range_pop(130, 7, 42),
        range_pop(140, 7, 42),
        range_pop(150, 7, 42),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.spans().len(), 3);

    let outer = span(&model, "outer");
    let middle = span(&model, "middle");
    let inner = span(&model, "inner");

    assert_eq!((outer.start, outer.end), (100, Some(150)));
    assert_eq!((middle.start, middle.end), (110, Some(140)));
    assert_eq!((inner.start, inner.end), (120, Some(130)));

    assert_eq!(outer.kind.parent(), None);
    assert_eq!(middle.kind.parent(), Some(span_id(&model, "outer")));
    assert_eq!(inner.kind.parent(), Some(span_id(&model, "middle")));

    for span in model.spans() {
        assert_eq!(span.kind.thread_id(), Some(42));
        assert_eq!(span.domain, 7);
        assert!(span.end.is_some(), "every pop was captured");
    }
}

#[test]
fn unclosed_pushes_have_no_end() {
    // Both pushes are left open; the mark carries the trace to 500. That is
    // where observation stopped, not where the ranges did, so neither gets an
    // end written into it.
    let events = vec![
        range_push(100, 1, 5, "leaked"),
        range_push(150, 1, 5, "also leaked"),
        mark(500, 1, "tick"),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(
        model.spans().len(),
        2,
        "both leaked pushes still become spans"
    );

    let leaked = span(&model, "leaked");
    assert_eq!(leaked.start, 100);
    assert_eq!(leaked.end, None, "the pop was never observed");
    assert_eq!(leaked.kind.parent(), None);

    let inner = span(&model, "also leaked");
    assert_eq!(inner.end, None);
    assert_eq!(
        inner.kind.parent(),
        Some(span_id(&model, "leaked")),
        "nesting is still known without a pop — the stack records it"
    );
}

#[test]
fn orphan_pop_skipped() {
    // The first pop has no open push on `(thread 5, domain 1)`; the second pop
    // belongs to another thread entirely. Neither is fatal, and neither steals
    // the real pair's push.
    let events = vec![
        range_pop(100, 1, 5),
        range_push(200, 1, 5, "real"),
        range_pop(300, 1, 9),
        range_pop(400, 1, 5),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.spans().len(), 1, "orphan pops produced no spans");
    assert_eq!(
        model.anomalies().orphan_range_pops,
        2,
        "both orphan pops are reported, not silently lost"
    );
    let real = span(&model, "real");
    assert_eq!(real.start, 200);
    assert_eq!(
        real.end,
        Some(400),
        "the cross-thread pop at 300 did not close it"
    );
}

#[test]
fn malformed_stream_completes() {
    // Everything the core promises to tolerate, in one stream: out-of-order
    // arrival, duplicate timestamps, an unmatched `RangeStart`, an orphan
    // `RangeEnd`, unclosed pushes, orphan pops, and a cross-thread pop.
    let stream = || {
        vec![
            // Out of order: this end precedes its start in arrival order.
            range_end(320, 1, 77),
            range_start(300, 1, 77, "reordered"),
            // Orphan end — no start was ever seen for this id.
            range_end(310, 1, 4242),
            // Unmatched start — never ended.
            range_start(305, 1, 78, "never ended"),
            // Duplicate timestamps across two threads.
            range_push(400, 2, 1, "dup a"),
            range_push(400, 2, 2, "dup b"),
            range_pop(400, 2, 1),
            range_pop(400, 2, 2),
            // Orphan pop on a thread that never pushed.
            range_pop(500, 2, 3),
            // Unclosed push, left open at trace end.
            range_push(600, 2, 1, "left open"),
            // A pop from a *different* thread must not close it.
            range_pop(650, 2, 99),
            // A mark, and a push in a domain that is never popped.
            mark(700, 0, "tick"),
            range_push(800, 3, 1, "other domain"),
        ]
    };

    // A malformed stream still builds — `build` is infallible by signature.
    let model = NvtxModelBuilder::build(stream());

    // 2 start/end + 4 push/pop = 6 spans; the two orphan ends/pops contribute none.
    assert_eq!(model.spans().len(), 6, "every open range became a span");

    let reordered = span(&model, "reordered");
    assert_eq!((reordered.start, reordered.end), (300, Some(320)));

    assert_eq!(span(&model, "never ended").end, None);
    assert_eq!(span(&model, "left open").end, None);
    assert_eq!(span(&model, "other domain").end, None);

    let dup_a = span(&model, "dup a");
    assert_eq!(dup_a.duration(), Some(0), "zero-duration spans are legal");
    assert_eq!(dup_a.kind.thread_id(), Some(1));

    for span in model.spans() {
        assert!(
            span.end.is_none_or(|end| end >= span.start),
            "duration can never underflow"
        );
    }

    assert_eq!(model.marks().len(), 1, "the mark survived the malformation");

    // Determinism: the same malformed stream reconstructs identically.
    let again = NvtxModelBuilder::build(stream());
    assert_eq!(model.spans(), again.spans());
}
