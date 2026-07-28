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

use std::collections::BTreeMap;

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::NvtxEvent;
use quent_events::Event;
use quent_time::{TimeOrderedCollector, TimeUnixNanoSec};

use crate::error::NvtxModelResult;
use crate::ranges::{PushPopRanges, StartEndRanges};
use crate::resource::Resources;
use crate::span::{NvtxCategory, NvtxDomain, NvtxMark, NvtxSpan, NvtxThread, SpanId, SpanKind};
use crate::stats::{self, RangeStats, StatsKey};
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
    /// A span's index here *is* its [`SpanId`], which is what makes
    /// [`NvtxSpan::parent`] resolvable. Ordering is by *opening*: push/pop ranges
    /// take their slot when pushed, and start/end ranges when they close (they do
    /// not nest, so nothing refers to them). Both are deterministic functions of
    /// the timestamp-ordered stream.
    pub fn spans(&self) -> &[NvtxSpan] {
        &self.spans
    }

    /// Every reconstructed mark, in timestamp order.
    pub fn marks(&self) -> &[NvtxMark] {
        &self.marks
    }

    /// Every reconstructed resource lifespan.
    ///
    /// A resource is structurally just an [`NvtxSpan`] with
    /// [`SpanKind::Resource`](crate::SpanKind::Resource) — the interval between
    /// its create and its destroy — carrying an
    /// [`identifier_type_label`](NvtxSpan::identifier_type_label). NVTX says only
    /// that the handle existed and what it was called, so nothing about its size
    /// or how much of it was in use is inferred.
    pub fn resources(&self) -> impl Iterator<Item = &NvtxSpan> {
        self.spans
            .iter()
            .filter(|span| span.kind == SpanKind::Resource)
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

    /// Aggregated durations per `(name, domain, category)` (ANA-06).
    ///
    /// Covers range spans only — push/pop and start/end. Marks have no duration
    /// and resource lifespans measure existence rather than work, so neither
    /// participates. Computed on demand rather than cached, because it is a pure
    /// fold over [`Self::spans`].
    pub fn range_statistics(&self) -> BTreeMap<StatsKey, RangeStats> {
        stats::range_statistics(&self.spans)
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

/// Write a closed span into the slot reserved for it when it was pushed.
///
/// Bounds-checked rather than indexed: the id always addresses a reserved slot,
/// but reconstruction of untrusted telemetry does not get to rely on that — a
/// stray id is dropped, not a panic.
fn fill(slots: &mut [Option<NvtxSpan>], id: SpanId, span: NvtxSpan) {
    if let Some(slot) = slots.get_mut(id.0) {
        *slot = Some(span);
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
        let mut pushes = PushPopRanges::default();
        let mut resources = Resources::default();
        let mut marks = Vec::new();
        let mut trace_end: TimeUnixNanoSec = 0;

        // Slots, not a plain span list: a `SpanId` is an index here, and a
        // nested push needs its *parent's* id at pop time — while the parent is
        // still open. Reserving the slot at push time is what makes that id
        // exist before the span does. Every reserved slot is filled, either by
        // its pop or by the trace-end drain below.
        let mut slots: Vec<Option<NvtxSpan>> = Vec::new();

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
                        slots.push(Some(span));
                    }
                }
                NvtxEvent::RangePush {
                    domain,
                    thread_id,
                    attributes,
                } => {
                    let name = tables.resolve_message(domain, &attributes.message);
                    let id = SpanId(slots.len());
                    slots.push(None);
                    pushes.push(id, thread_id, domain, name, attributes, event.timestamp);
                }
                NvtxEvent::RangePop { domain, thread_id } => {
                    if let Some((id, span)) = pushes.pop(thread_id, domain, event.timestamp) {
                        fill(&mut slots, id, span);
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
                NvtxEvent::ResourceCreate {
                    domain,
                    handle,
                    identifier_type,
                    // The raw identifier bits are captured verbatim but carry no
                    // reconstruction meaning: their interpretation depends on
                    // `identifier_type`, and decoding extension classes is
                    // deliberately out of scope for this core-only slice.
                    identifier: _,
                    message,
                } => {
                    let name = tables.resolve_message(domain, &message);
                    resources.create(handle, domain, name, identifier_type, event.timestamp);
                }
                // Matched on `handle` alone — the event carries no domain, so
                // there is nothing else to key on. The domain comes back from
                // the create.
                NvtxEvent::ResourceDestroy { handle } => {
                    if let Some(span) = resources.destroy(handle, event.timestamp) {
                        slots.push(Some(span));
                    }
                }
                // Registration events were already consumed by pass 1, but they
                // still advance the trace end.
                _ => {}
            }
        }

        // Anything still open never had its close observed.
        slots.extend(ranges.close_at_trace_end(trace_end).into_iter().map(Some));
        slots.extend(
            resources
                .close_at_trace_end(trace_end)
                .into_iter()
                .map(Some),
        );
        for (id, span) in pushes.close_at_trace_end(trace_end) {
            fill(&mut slots, id, span);
        }

        // Every reserved slot is filled by construction — a push is closed
        // either by its pop or by the drain above — so flattening preserves the
        // indices the `SpanId`s were handed out against.
        let spans: Vec<NvtxSpan> = slots.into_iter().flatten().collect();

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
