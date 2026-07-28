// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Per-`(thread_id, domain)` nested Push/Pop reconstruction, and the tolerance
//! guarantees that hold while doing it.
//!
//! The keying is the whole point: NVTX's push/pop stack is per-thread *and*
//! per-domain, so a `RangePop` must close the innermost `RangePush` on its own
//! thread and domain and nothing else. A single global stack would reconstruct
//! these same streams into plausible-but-wrong nesting rather than failing
//! visibly, so the interleaved test below is the load-bearing one.

mod fixtures;

use fixtures::{mark, range_end, range_pop, range_push, range_start};
use nvtx_analyzer::{NvtxModel, NvtxModelBuilder, NvtxSpan, SpanId, SpanKind};

/// The single span whose resolved name is `name`.
fn span<'a>(model: &'a NvtxModel, name: &str) -> &'a NvtxSpan {
    let mut matches = model.spans().iter().filter(|span| span.name == name);
    let found = matches
        .next()
        .unwrap_or_else(|| panic!("no span named {name:?}"));
    assert!(
        matches.next().is_none(),
        "more than one span named {name:?}"
    );
    found
}

/// The [`SpanId`] of the single span named `name`.
///
/// A `SpanId` *is* the index into [`NvtxModel::spans`], so this is the identity
/// a `parent` reference must equal.
fn span_id(model: &NvtxModel, name: &str) -> SpanId {
    let index = model
        .spans()
        .iter()
        .position(|span| span.name == name)
        .unwrap_or_else(|| panic!("no span named {name:?}"));
    SpanId(index)
}

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

    let model = NvtxModelBuilder::build(events).expect("build");

    assert_eq!(model.spans().len(), 3, "one span per matched push/pop pair");

    let a = span(&model, "a");
    assert_eq!(a.kind, SpanKind::PushPop);
    assert_eq!(a.thread_id, Some(1));
    assert_eq!(a.start, 100);
    assert_eq!(a.end, 140, "thread 1's outer push closed on its second pop");
    assert_eq!(a.parent, None, "outermost on its thread");
    assert!(!a.synthetic_end);

    let c = span(&model, "c");
    assert_eq!(c.thread_id, Some(1));
    assert_eq!(c.start, 120);
    assert_eq!(c.end, 130, "the first pop closed the innermost push");
    assert_eq!(
        c.parent,
        Some(span_id(&model, "a")),
        "nested inside the enclosing push on the same thread"
    );

    let b = span(&model, "b");
    assert_eq!(b.thread_id, Some(2));
    assert_eq!(b.start, 110);
    assert_eq!(
        b.end, 150,
        "thread 2's push is closed only by thread 2's pop"
    );
    assert_eq!(
        b.parent, None,
        "a push on another thread is never a parent — the stacks are independent"
    );
    assert!(!b.synthetic_end);
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

    let model = NvtxModelBuilder::build(events).expect("build");

    assert_eq!(model.spans().len(), 3);

    let outer = span(&model, "outer");
    let middle = span(&model, "middle");
    let inner = span(&model, "inner");

    assert_eq!((outer.start, outer.end), (100, 150));
    assert_eq!((middle.start, middle.end), (110, 140));
    assert_eq!((inner.start, inner.end), (120, 130));

    assert_eq!(outer.parent, None);
    assert_eq!(middle.parent, Some(span_id(&model, "outer")));
    assert_eq!(inner.parent, Some(span_id(&model, "middle")));

    for span in model.spans() {
        assert_eq!(span.kind, SpanKind::PushPop);
        assert_eq!(span.thread_id, Some(42));
        assert_eq!(span.domain, 7);
        assert!(!span.synthetic_end);
    }
}

#[test]
fn unclosed_closed_at_trace_end() {
    // `leaked` is pushed and never popped; the mark sets the trace end.
    let events = vec![
        range_push(100, 1, 5, "leaked"),
        range_push(150, 1, 5, "also leaked"),
        mark(500, 1, "tick"),
    ];

    let model = NvtxModelBuilder::build(events).expect("build");

    assert_eq!(
        model.spans().len(),
        2,
        "both leaked pushes still become spans"
    );

    let leaked = span(&model, "leaked");
    assert_eq!(leaked.start, 100);
    assert_eq!(leaked.end, 500, "closed at the max observed timestamp");
    assert!(leaked.synthetic_end, "the pop was never observed");
    assert_eq!(leaked.parent, None);

    let inner = span(&model, "also leaked");
    assert_eq!(inner.end, 500);
    assert!(inner.synthetic_end);
    assert_eq!(
        inner.parent,
        Some(span_id(&model, "leaked")),
        "nesting is still known at trace end — the stack records it"
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

    let model = NvtxModelBuilder::build(events).expect("build");

    assert_eq!(model.spans().len(), 1, "orphan pops produced no spans");
    let real = span(&model, "real");
    assert_eq!(real.start, 200);
    assert_eq!(
        real.end, 400,
        "the cross-thread pop at 300 did not close it"
    );
    assert!(!real.synthetic_end);
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

    let model = NvtxModelBuilder::build(stream()).expect("a malformed stream still builds");

    // 2 start/end + 4 push/pop = 6 spans; the two orphan ends/pops contribute none.
    assert_eq!(model.spans().len(), 6, "every open range became a span");

    let reordered = span(&model, "reordered");
    assert_eq!((reordered.start, reordered.end), (300, 320));

    assert!(span(&model, "never ended").synthetic_end);
    assert!(span(&model, "left open").synthetic_end);
    assert!(span(&model, "other domain").synthetic_end);

    let dup_a = span(&model, "dup a");
    assert_eq!(dup_a.duration(), 0, "zero-duration spans are legal");
    assert_eq!(dup_a.thread_id, Some(1));

    for span in model.spans() {
        assert!(span.start <= span.end, "duration can never underflow");
    }

    assert_eq!(model.marks().len(), 1, "the mark survived the malformation");

    // Determinism: the same malformed stream reconstructs identically.
    let again = NvtxModelBuilder::build(stream()).expect("rebuild");
    assert_eq!(model.spans(), again.spans());
}
