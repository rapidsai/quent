// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Reconstruction of NVTX ranges — the two kinds, with two different match keys.
//!
//! **`RangeStart`/`RangeEnd`** correlate by `range_id` **alone**. NVTX assigns
//! range ids from a single process-global counter, so a start and its end
//! correlate across threads and even across domains without any further keying —
//! the domain on a `RangeEnd` is redundant and deliberately ignored.
//!
//! **`RangePush`/`RangePop`** carry no id at all; they correlate by *position on
//! a stack*, and the stack is per `(thread_id, domain)`. That grain is not a
//! guess: the injection layer's own `RANGE_DEPTH` is a thread-local keyed by
//! domain (`nvtx-injection`'s `range_push_level`/`range_pop_level`), which is the
//! same `(thread, domain)` pair viewed from the capture side. Keying any coarser
//! — one global stack, or one stack per domain — does not fail loudly on a
//! multi-threaded stream; it silently closes another thread's push and
//! reconstructs plausible-but-wrong nesting.
//!
//! Every anomaly here is tolerated: a close with no matching open is logged and
//! skipped, and an open that is never closed is closed at trace end and flagged
//! synthetic. Neither aborts reconstruction.

use std::collections::HashMap;

use nvtx_events::NvtxEventAttributes;
use quent_time::TimeUnixNanoSec;
use tracing::warn;

use crate::span::{NvtxSpan, SpanId, SpanKind};

/// A `RangeStart` awaiting its matching `RangeEnd`.
struct OpenStartRange {
    domain: u64,
    /// Resolved at open time against the pass-1 tables, which are already
    /// complete by the time replay begins.
    name: String,
    attributes: NvtxEventAttributes,
    start: TimeUnixNanoSec,
}

impl OpenStartRange {
    /// Close this range, forming the reconstructed span.
    ///
    /// `end` is clamped up to `start`, so a pair that arrives inverted yields a
    /// zero-duration span rather than an underflowing one.
    fn close(self, end: TimeUnixNanoSec, synthetic_end: bool) -> NvtxSpan {
        NvtxSpan {
            domain: self.domain,
            // `RangeStart`/`RangeEnd` are process-wide by definition and carry no
            // thread id; per-thread attribution belongs to push/pop ranges.
            thread_id: None,
            name: self.name,
            // Category `0` is NVTX's "no category" sentinel.
            category: (self.attributes.category != 0).then_some(self.attributes.category),
            color: self.attributes.color,
            payload: self.attributes.payload,
            start: self.start,
            end: end.max(self.start),
            kind: SpanKind::StartEnd,
            // Only resource spans identify a thing; a range identifies work.
            identifier_type_label: None,
            parent: None,
            synthetic_end,
        }
    }
}

/// The set of currently-open process-wide ranges.
#[derive(Default)]
pub(crate) struct StartEndRanges {
    open: HashMap<u64, OpenStartRange>,
}

impl StartEndRanges {
    /// Record a `RangeStart` under its already-resolved `name`.
    pub(crate) fn start(
        &mut self,
        range_id: u64,
        domain: u64,
        name: String,
        attributes: NvtxEventAttributes,
        start: TimeUnixNanoSec,
    ) {
        let open = OpenStartRange {
            domain,
            name,
            attributes,
            start,
        };
        if self.open.insert(range_id, open).is_some() {
            warn!(
                "nvtx range id 0x{range_id:X} was restarted before it ended; dropping the earlier start"
            );
        }
    }

    /// Close the range matching `range_id`, if one is open.
    ///
    /// Returns `None` for an orphan end — logged and skipped, never fatal.
    pub(crate) fn end(&mut self, range_id: u64, end: TimeUnixNanoSec) -> Option<NvtxSpan> {
        let Some(open) = self.open.remove(&range_id) else {
            warn!("orphan nvtx range end for id 0x{range_id:X} with no open start; skipping");
            return None;
        };
        Some(open.close(end, false))
    }

    /// Close every range still open at the end of the trace.
    ///
    /// Spans come back ordered by start timestamp (then by range id) so a stream
    /// with several leaked ranges still reconstructs deterministically.
    pub(crate) fn close_at_trace_end(&mut self, trace_end: TimeUnixNanoSec) -> Vec<NvtxSpan> {
        let mut leaked: Vec<(u64, OpenStartRange)> = self.open.drain().collect();
        leaked.sort_by_key(|(range_id, open)| (open.start, *range_id));

        leaked
            .into_iter()
            .map(|(range_id, open)| {
                warn!(
                    "nvtx range id 0x{range_id:X} was never ended; closing it at trace end ({trace_end})"
                );
                open.close(trace_end, true)
            })
            .collect()
    }
}

/// The stack a push/pop range nests on: one OS thread within one domain.
///
/// Mirrors the capture side's thread-local-keyed-by-domain `RANGE_DEPTH`, with
/// the thread made explicit because the analyzer replays every thread's events
/// on one thread of its own.
type StackKey = (u32, u64);

/// A `RangePush` awaiting its matching `RangePop`.
struct OpenPushSpan {
    /// The slot already reserved for this span in the model's span list.
    ///
    /// Reserved at *push* time rather than assigned at pop time, because a
    /// child pops before its parent does and needs the parent's id to record
    /// `parent`. Reserving up front is what makes that id knowable early.
    id: SpanId,
    domain: u64,
    thread_id: u32,
    /// Resolved at open time against the pass-1 tables, which are already
    /// complete by the time replay begins.
    name: String,
    attributes: NvtxEventAttributes,
    start: TimeUnixNanoSec,
}

impl OpenPushSpan {
    /// Close this push, forming the reconstructed span.
    ///
    /// `end` is clamped up to `start`, so a pair that arrives inverted yields a
    /// zero-duration span rather than an underflowing one.
    fn close(
        self,
        end: TimeUnixNanoSec,
        parent: Option<SpanId>,
        synthetic_end: bool,
    ) -> (SpanId, NvtxSpan) {
        let span = NvtxSpan {
            domain: self.domain,
            thread_id: Some(self.thread_id),
            name: self.name,
            // Category `0` is NVTX's "no category" sentinel.
            category: (self.attributes.category != 0).then_some(self.attributes.category),
            color: self.attributes.color,
            payload: self.attributes.payload,
            start: self.start,
            end: end.max(self.start),
            kind: SpanKind::PushPop,
            // Only resource spans identify a thing; a range identifies work.
            identifier_type_label: None,
            parent,
            synthetic_end,
        };
        (self.id, span)
    }
}

/// The currently-open push/pop ranges, one stack per `(thread_id, domain)`.
#[derive(Default)]
pub(crate) struct PushPopRanges {
    stacks: HashMap<StackKey, Vec<OpenPushSpan>>,
}

impl PushPopRanges {
    /// Record a `RangePush` under its already-resolved `name`, in the slot `id`.
    pub(crate) fn push(
        &mut self,
        id: SpanId,
        thread_id: u32,
        domain: u64,
        name: String,
        attributes: NvtxEventAttributes,
        start: TimeUnixNanoSec,
    ) {
        self.stacks
            .entry((thread_id, domain))
            .or_default()
            .push(OpenPushSpan {
                id,
                domain,
                thread_id,
                name,
                attributes,
                start,
            });
    }

    /// Close the innermost open push on `(thread_id, domain)`, if there is one.
    ///
    /// `parent` is read from the stack *after* the pop, so it is the range that
    /// encloses the one just closed — `None` when the closed range was
    /// outermost on its thread.
    ///
    /// Returns `None` for an orphan pop — logged and skipped, never fatal. The
    /// pop is a `Vec::pop`, so an empty stack yields `None` rather than panicking.
    pub(crate) fn pop(
        &mut self,
        thread_id: u32,
        domain: u64,
        end: TimeUnixNanoSec,
    ) -> Option<(SpanId, NvtxSpan)> {
        let key = (thread_id, domain);
        // `Vec::pop` on an empty stack yields `None`, never a panic, so an
        // unbalanced stream degrades to a skipped pop.
        let Some(open) = self.stacks.get_mut(&key).and_then(Vec::pop) else {
            warn!(
                "orphan nvtx range pop on thread {thread_id} in domain 0x{domain:X} \
                 with no open push; skipping"
            );
            return None;
        };
        // The enclosing range is whatever the pop uncovered.
        let parent = self
            .stacks
            .get(&key)
            .and_then(|stack| stack.last())
            .map(|enclosing| enclosing.id);
        // Retain only keys with currently-open ranges, mirroring the capture
        // side's `range_pop_level`, so the map does not grow with every thread
        // the process ever ran.
        if self.stacks.get(&key).is_some_and(Vec::is_empty) {
            self.stacks.remove(&key);
        }
        Some(open.close(end, parent, false))
    }

    /// Close every push still open at the end of the trace.
    ///
    /// Nesting is still known here — it is exactly the stack's own shape — so a
    /// leaked inner push keeps its parent rather than being flattened. Stacks are
    /// visited in key order and drained innermost-first, so a stream with several
    /// leaked pushes still reconstructs deterministically.
    pub(crate) fn close_at_trace_end(
        &mut self,
        trace_end: TimeUnixNanoSec,
    ) -> Vec<(SpanId, NvtxSpan)> {
        let mut leaked: Vec<(StackKey, Vec<OpenPushSpan>)> = self.stacks.drain().collect();
        leaked.sort_by_key(|(key, _)| *key);

        leaked
            .into_iter()
            .flat_map(|((thread_id, domain), stack)| {
                // Each open push's parent is the one immediately below it.
                let parents: Vec<Option<SpanId>> = (0..stack.len())
                    .map(|depth| depth.checked_sub(1).map(|below| stack[below].id))
                    .collect();

                stack
                    .into_iter()
                    .zip(parents)
                    .rev()
                    .map(move |(open, parent)| {
                        warn!(
                            "nvtx range \"{}\" on thread {thread_id} in domain 0x{domain:X} was \
                             never popped; closing it at trace end ({trace_end})",
                            open.name
                        );
                        open.close(trace_end, parent, true)
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}
