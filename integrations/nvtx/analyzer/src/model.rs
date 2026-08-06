// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The two-pass reconstruction entry point.
//!
//! Pass 1 materializes the stream in timestamp order and builds the
//! handle-resolution tables; pass 2 replays it against them. Splitting the two
//! is what makes replay tolerant of arrival order in both dimensions — by then
//! neither an end before its start nor a `RegisterString` after the range using
//! it is still possible.

use std::collections::BTreeMap;

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::NvtxEvent;
use quent_events::Event;
use quent_time::{TimeOrderedCollector, TimeUnixNanoSec};

use crate::anomalies::ReconstructionAnomalies;
use crate::ranges::{PushPopRanges, StartEndRanges};
use crate::resource::Resources;
use crate::span::{
    NvtxCategory, NvtxDomain, NvtxMark, NvtxSpan, NvtxThread, SpanId, SpanKind, category_id,
};
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
    anomalies: ReconstructionAnomalies,
    /// Event timestamp bounds. Not recoverable from the spans — see the
    /// accessors.
    trace_start: TimeUnixNanoSec,
    trace_end: TimeUnixNanoSec,
}

impl NvtxModel {
    /// Every reconstructed span.
    ///
    /// A span's index here *is* its [`SpanId`], which is what makes
    /// [`SpanKind::PushPop::parent`](SpanKind::PushPop) resolvable. Ordering is
    /// by opening: push/pop ranges take their slot when pushed, start/end ranges
    /// when they close.
    pub fn spans(&self) -> &[NvtxSpan] {
        &self.spans
    }

    /// The span a [`SpanId`] refers to, or `None` if it came from another model.
    pub fn span(&self, id: SpanId) -> Option<&NvtxSpan> {
        self.spans.get(id.0)
    }

    /// What the stream did that the span list does not show — dropped closes,
    /// and keys reused while still open.
    ///
    /// Check [`is_faithful`](ReconstructionAnomalies::is_faithful) before
    /// treating [`spans`](Self::spans) as the whole population: attaching to a
    /// process with work already running truncates the head of the trace, and a
    /// dropped close leaves nothing behind to notice it by.
    pub fn anomalies(&self) -> ReconstructionAnomalies {
        self.anomalies
    }

    /// The earliest event timestamp, or `0` for an empty stream.
    ///
    /// Not derivable from [`Self::spans`]: the first event may be a mark, or a
    /// close whose open was never captured.
    pub fn trace_start(&self) -> TimeUnixNanoSec {
        self.trace_start
    }

    /// The latest event timestamp, or `0` for an empty stream.
    ///
    /// Where observation stopped — the bound to reach for when an analysis needs
    /// to give a span with no [`end`](NvtxSpan::end) a right edge. Reconstruction
    /// does not apply it for you, because whether that bound belongs in a figure
    /// depends on the figure. The largest span `end` is not a substitute: the
    /// last event may carry no span at all.
    pub fn trace_end(&self) -> TimeUnixNanoSec {
        self.trace_end
    }

    /// Every reconstructed mark, in timestamp order.
    pub fn marks(&self) -> &[NvtxMark] {
        &self.marks
    }

    /// Every reconstructed resource lifespan: the interval between a create and
    /// its destroy.
    ///
    /// NVTX says only that the handle existed and what it was called, so nothing
    /// about its size or occupancy is inferred.
    pub fn resources(&self) -> impl Iterator<Item = &NvtxSpan> {
        self.spans
            .iter()
            .filter(|span| matches!(span.kind, SpanKind::Resource { .. }))
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

    /// Aggregated durations per `(name, domain, category)`.
    ///
    /// Range spans only. Marks have no duration and resource lifespans measure
    /// existence rather than work, so neither participates.
    pub fn range_statistics(&self) -> BTreeMap<StatsKey, RangeStats> {
        stats::range_statistics(&self.spans)
    }

    /// Aggregated durations over the range spans `keep` accepts.
    ///
    /// For a view showing part of a capture — whole-model statistics would not
    /// describe the spans on screen.
    pub fn range_statistics_where(
        &self,
        keep: impl Fn(&NvtxSpan) -> bool,
    ) -> BTreeMap<StatsKey, RangeStats> {
        stats::range_statistics_where(&self.spans, keep)
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
/// Bounds-checked rather than indexed: a stray id is dropped, not a panic.
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
    /// Events may arrive in any order and may reference handles registered
    /// anywhere in the stream. Incomplete pairs are represented rather than
    /// dropped or guessed — see the crate docs for what each case yields.
    pub fn build(events: impl IntoIterator<Item = Event<NvtxEventEntity>>) -> NvtxModel {
        // Pass 1a — materialize in timestamp order. Equal timestamps keep
        // arrival order, so replay is deterministic.
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
        let mut anomalies = ReconstructionAnomalies::default();
        let mut trace_start: Option<TimeUnixNanoSec> = None;
        let mut trace_end: TimeUnixNanoSec = 0;

        // Slots, not a plain span list: a `SpanId` is an index here, and a
        // nested push needs its parent's id at pop time — while the parent is
        // still open. Reserving the slot at push time makes that id exist
        // before the span does.
        let mut slots: Vec<Option<NvtxSpan>> = Vec::new();

        for event in ordered {
            trace_start = Some(trace_start.map_or(event.timestamp, |at| at.min(event.timestamp)));
            trace_end = trace_end.max(event.timestamp);

            match event.data.0 {
                NvtxEvent::RangeStart {
                    domain,
                    range_id,
                    attributes,
                } => {
                    let name = tables.resolve_message(domain, &attributes.message);
                    if let Some(span) =
                        ranges.start(range_id, domain, name, attributes, event.timestamp)
                    {
                        // A span came back only because this start displaced one
                        // still open under the same id.
                        anomalies.reused_range_ids += 1;
                        slots.push(Some(span));
                    }
                }
                // The domain on a `RangeEnd` is redundant: `range_id` is
                // process-globally unique, so it alone identifies the range.
                NvtxEvent::RangeEnd { range_id, .. } => {
                    match ranges.end(range_id, event.timestamp) {
                        Some(span) => slots.push(Some(span)),
                        None => anomalies.orphan_range_ends += 1,
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
                    match pushes.pop(thread_id, domain, event.timestamp) {
                        Some((id, span)) => fill(&mut slots, id, span),
                        None => anomalies.orphan_range_pops += 1,
                    }
                }
                NvtxEvent::Mark { domain, attributes } => marks.push(NvtxMark {
                    domain,
                    // `nvtxDomainMarkEx` carries no thread id.
                    thread_id: None,
                    name: tables.resolve_message(domain, &attributes.message),
                    category: category_id(attributes.category),
                    color: attributes.color,
                    payload: attributes.payload,
                    timestamp: event.timestamp,
                }),
                NvtxEvent::ResourceCreate {
                    domain,
                    handle,
                    identifier_type,
                    // Interpreting the raw identifier bits depends on
                    // `identifier_type`; decoding extension classes is out of
                    // scope for this crate.
                    identifier: _,
                    message,
                } => {
                    let name = tables.resolve_message(domain, &message);
                    if let Some(span) =
                        resources.create(handle, domain, name, identifier_type, event.timestamp)
                    {
                        // As above: only a displaced lifespan returns a span.
                        anomalies.reused_resource_handles += 1;
                        slots.push(Some(span));
                    }
                }
                // Matched on `handle` alone — the event carries no domain, so
                // there is nothing else to key on.
                NvtxEvent::ResourceDestroy { handle } => {
                    match resources.destroy(handle, event.timestamp) {
                        Some(span) => slots.push(Some(span)),
                        None => anomalies.orphan_resource_destroys += 1,
                    }
                }
                // Consumed by pass 1; they only advance the trace end here.
                // Listed explicitly rather than caught by `_`, so a new
                // `NvtxEvent` variant fails to compile instead of being
                // silently discarded.
                NvtxEvent::DomainCreate { .. }
                | NvtxEvent::DomainDestroy { .. }
                | NvtxEvent::RegisterString { .. }
                | NvtxEvent::NameCategory { .. }
                | NvtxEvent::NameThread { .. } => {}
            }
        }

        // Anything still open never had its close observed, so it ends `None`.
        slots.extend(ranges.drain_unclosed().into_iter().map(Some));
        slots.extend(resources.drain_unclosed().into_iter().map(Some));
        for (id, span) in pushes.drain_unclosed() {
            fill(&mut slots, id, span);
        }

        // Flattening preserves the indices the `SpanId`s were handed out
        // against only while every slot is filled; a hole would shift every
        // later index, so the invariant is asserted rather than assumed.
        debug_assert!(
            slots.iter().all(Option::is_some),
            "an unfilled slot would invalidate every SpanId after it"
        );
        // `Flatten` reports a lower size hint of `0`, so collecting alone would
        // grow the span list by repeated doubling.
        let mut spans: Vec<NvtxSpan> = Vec::with_capacity(slots.len());
        spans.extend(slots.into_iter().flatten());

        let domains = tables.domain_records();
        let threads = tables.thread_records();
        let categories = tables.category_records();

        NvtxModel {
            spans,
            marks,
            domains,
            threads,
            categories,
            tables,
            anomalies,
            trace_start: trace_start.unwrap_or(0),
            trace_end,
        }
    }
}
