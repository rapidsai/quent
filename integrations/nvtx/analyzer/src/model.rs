// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The two-pass reconstruction entry point.
//!
//! Pass 1 materializes the stream in timestamp order and builds the
//! handle-resolution tables; pass 2 replays it, dispatching each event to the
//! reconstruction that owns it and resolving its labels against those tables.
//! Splitting the two is what makes the core tolerant of arrival order in both
//! dimensions: nothing in pass 2 has to cope with an end that arrives before its
//! start, nor with a `RegisterString` that arrives after the range using its
//! handle, because by then neither is possible.

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::NvtxEvent;
use quent_events::Event;
use quent_time::{TimeOrderedCollector, TimeUnixNanoSec};

use crate::error::NvtxModelResult;
use crate::ranges::StartEndRanges;
use crate::span::{NvtxCategory, NvtxDomain, NvtxMark, NvtxSpan, NvtxThread};
use crate::tables::ResolutionTables;

/// An in-memory model reconstructed from a captured NVTX event stream.
#[derive(Debug, Default)]
pub struct NvtxModel {
    spans: Vec<NvtxSpan>,
    marks: Vec<NvtxMark>,
    domains: Vec<NvtxDomain>,
    threads: Vec<NvtxThread>,
    categories: Vec<NvtxCategory>,
    /// Retained so names not attached to a reconstructed entity — an unnamed
    /// thread, a category referenced by nothing yet — still resolve on demand.
    tables: ResolutionTables,
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
    /// The domain is required, not optional: NVTX category ids are unique only
    /// within a domain, so resolving one globally would return another domain's
    /// name. Returns `None` for category `0`, NVTX's "no category" sentinel.
    pub fn category_name(&self, domain: u64, category: u32) -> Option<String> {
        self.tables.resolve_category(domain, category)
    }

    /// The resolved name of an OS thread.
    ///
    /// Threads are usually unnamed, so this answers for *any* id — a thread the
    /// stream never named renders as `"thread {id}"` rather than nothing.
    pub fn thread_name(&self, thread_id: u32) -> String {
        self.tables.resolve_thread(thread_id)
    }
}

/// Builds an [`NvtxModel`] from a captured event stream.
#[derive(Debug, Default)]
pub struct NvtxModelBuilder;

impl NvtxModelBuilder {
    /// Reconstruct a model from a captured NVTX event stream.
    ///
    /// Tolerant by construction: out-of-order events are reordered, forward
    /// references to not-yet-registered handles resolve anyway, duplicate
    /// timestamps reconstruct deterministically, ranges left open at the end of
    /// the stream are closed and flagged, and closes with no matching open are
    /// logged and skipped. Handles that never resolve get a stable placeholder.
    /// No malformed stream aborts the build or panics.
    ///
    /// # Errors
    ///
    /// Returns [`NvtxModelError`](crate::NvtxModelError) only when the stream
    /// itself cannot be obtained. Stream *anomalies* are tolerated, never
    /// returned as errors.
    pub fn build(
        events: impl IntoIterator<Item = Event<NvtxEventEntity>>,
    ) -> NvtxModelResult<NvtxModel> {
        // Pass 1a — materialize in timestamp order. `TimeOrderedCollector` is
        // O(1) for the in-order common case and binary-inserts late arrivals;
        // equal timestamps keep arrival order, so replay is deterministic.
        let mut collector = TimeOrderedCollector::default();
        collector.extend(events);
        let ordered = collector.into_inner();

        // Pass 1b — learn every name in the stream before resolving any of them.
        let tables = ResolutionTables::build(&ordered);

        // Pass 2 — replay.
        let mut ranges = StartEndRanges::default();
        let mut spans = Vec::new();
        let mut marks = Vec::new();
        let mut trace_end: TimeUnixNanoSec = 0;

        for event in ordered {
            trace_end = trace_end.max(event.timestamp);

            match event.data.0 {
                NvtxEvent::RangeStart {
                    domain,
                    range_id,
                    attributes,
                } => {
                    let name = tables.resolve_message(domain, &attributes.message);
                    ranges.start(range_id, domain, name, attributes, event.timestamp);
                }
                // The domain on a `RangeEnd` is redundant: `range_id` is
                // process-globally unique, so it alone identifies the range.
                NvtxEvent::RangeEnd { range_id, .. } => {
                    if let Some(span) = ranges.end(range_id, event.timestamp) {
                        spans.push(span);
                    }
                }
                NvtxEvent::Mark { domain, attributes } => marks.push(NvtxMark {
                    domain,
                    // `nvtxDomainMarkEx` carries no thread id; the field is kept
                    // for the push/pop slice, which does.
                    thread_id: None,
                    name: tables.resolve_message(domain, &attributes.message),
                    // Category `0` is NVTX's "no category" sentinel.
                    category: (attributes.category != 0).then_some(attributes.category),
                    color: attributes.color,
                    payload: attributes.payload,
                    timestamp: event.timestamp,
                }),
                // Push/pop ranges and resources land in later slices; the
                // registration events were already consumed by pass 1.
                // Unhandled events still advance the trace end.
                _ => {}
            }
        }

        // Anything still open never had its close observed.
        spans.extend(ranges.close_at_trace_end(trace_end));

        let domains = tables.domain_records();
        let threads = tables.thread_records();
        let categories = tables.category_records();

        Ok(NvtxModel {
            spans,
            marks,
            domains,
            threads,
            categories,
            tables,
        })
    }
}
