// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Reconstruction of process-wide `RangeStart`/`RangeEnd` pairs.
//!
//! The match key is `range_id` **alone**. NVTX assigns range ids from a single
//! process-global counter, so a start and its end correlate across threads and
//! even across domains without any further keying — the domain on a `RangeEnd`
//! is redundant and deliberately ignored.
//!
//! Every anomaly here is tolerated: an end with no open start is logged and
//! skipped, and a start that is never ended is closed at trace end and flagged
//! synthetic. Neither aborts reconstruction.

use std::collections::HashMap;

use nvtx_events::NvtxEventAttributes;
use quent_time::TimeUnixNanoSec;
use tracing::warn;

use crate::span::{NvtxSpan, SpanKind};

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
