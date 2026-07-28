// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The two-pass reconstruction entry point.
//!
//! Pass 1 materializes the stream in timestamp order; pass 2 replays it,
//! dispatching each event to the reconstruction that owns it. Splitting the two
//! is what makes the core tolerant of arrival order: nothing in pass 2 has to
//! cope with an end that arrives before its start, because by then it cannot.
//!
//! Handle-resolution tables (registered strings, domain and category names) also
//! belong to pass 1 and land in a later slice; today pass 1 only orders.

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::NvtxEvent;
use quent_events::Event;
use quent_time::{TimeOrderedCollector, TimeUnixNanoSec};

use crate::error::NvtxModelResult;
use crate::ranges::StartEndRanges;
use crate::span::{NvtxCategory, NvtxDomain, NvtxMark, NvtxSpan, NvtxThread};

/// An in-memory model reconstructed from a captured NVTX event stream.
#[derive(Debug, Default)]
pub struct NvtxModel {
    spans: Vec<NvtxSpan>,
    marks: Vec<NvtxMark>,
    domains: Vec<NvtxDomain>,
    threads: Vec<NvtxThread>,
    categories: Vec<NvtxCategory>,
}

impl NvtxModel {
    /// Every reconstructed span.
    ///
    /// Ordered by completion: spans whose close was observed come first, in the
    /// order they closed, followed by any synthetically closed at trace end.
    pub fn spans(&self) -> &[NvtxSpan] {
        &self.spans
    }

    /// Every reconstructed mark, in timestamp order.
    pub fn marks(&self) -> &[NvtxMark] {
        &self.marks
    }

    /// Every domain the stream mentioned, ordered by raw handle.
    pub fn domains(&self) -> &[NvtxDomain] {
        &self.domains
    }

    /// Every OS thread the stream mentioned, ordered by id.
    pub fn threads(&self) -> &[NvtxThread] {
        &self.threads
    }

    /// Every non-zero category the stream mentioned, ordered by `(domain, id)`.
    pub fn categories(&self) -> &[NvtxCategory] {
        &self.categories
    }

    /// The resolved name of a category *within its domain*.
    ///
    /// Returns `None` for category `0`, NVTX's "no category" sentinel.
    pub fn category_name(&self, _domain: u64, _category: u32) -> Option<String> {
        // Resolution lands with pass 1; today this is unwired.
        None
    }

    /// The resolved name of an OS thread, placeholder included.
    pub fn thread_name(&self, _thread_id: u32) -> String {
        // Resolution lands with pass 1; today this is unwired.
        String::new()
    }
}

/// Builds an [`NvtxModel`] from a captured event stream.
#[derive(Debug, Default)]
pub struct NvtxModelBuilder;

impl NvtxModelBuilder {
    /// Reconstruct a model from a captured NVTX event stream.
    ///
    /// Tolerant by construction: out-of-order events are reordered, duplicate
    /// timestamps reconstruct deterministically, ranges left open at the end of
    /// the stream are closed and flagged, and closes with no matching open are
    /// logged and skipped. No malformed stream aborts the build or panics.
    ///
    /// # Errors
    ///
    /// Returns [`NvtxModelError`](crate::NvtxModelError) only when the stream
    /// itself cannot be obtained. Stream *anomalies* are tolerated, never
    /// returned as errors.
    pub fn build(
        events: impl IntoIterator<Item = Event<NvtxEventEntity>>,
    ) -> NvtxModelResult<NvtxModel> {
        // Pass 1 — materialize in timestamp order. `TimeOrderedCollector` is
        // O(1) for the in-order common case and binary-inserts late arrivals;
        // equal timestamps keep arrival order, so replay is deterministic.
        let mut collector = TimeOrderedCollector::default();
        collector.extend(events);
        let ordered = collector.into_inner();

        // Pass 2 — replay.
        let mut ranges = StartEndRanges::default();
        let mut spans = Vec::new();
        let mut trace_end: TimeUnixNanoSec = 0;

        for event in ordered {
            trace_end = trace_end.max(event.timestamp);

            match event.data.0 {
                NvtxEvent::RangeStart {
                    domain,
                    range_id,
                    attributes,
                } => ranges.start(range_id, domain, attributes, event.timestamp),
                // The domain on a `RangeEnd` is redundant: `range_id` is
                // process-globally unique, so it alone identifies the range.
                NvtxEvent::RangeEnd { range_id, .. } => {
                    if let Some(span) = ranges.end(range_id, event.timestamp) {
                        spans.push(span);
                    }
                }
                // Push/pop ranges, marks, and resources land in later slices.
                // Unhandled events still advance the trace end.
                _ => {}
            }
        }

        // Anything still open never had its close observed.
        spans.extend(ranges.close_at_trace_end(trace_end));

        // Marks, domains, threads, and categories are populated once pass 1
        // exists to resolve them.
        Ok(NvtxModel {
            spans,
            ..Default::default()
        })
    }
}
