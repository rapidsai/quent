// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The reconstruction core's own plain span types.
//!
//! These are hand-written structs, not framework types: no shared runtime
//! state machine, no shared-model entity, no macro DSL. A reconstructed NVTX
//! range *is* a span —
//! an interval on the time axis carrying the attributes captured verbatim with
//! it. Keeping our own type is what lets the core stay tolerant (zero-duration
//! and synthetically-closed spans are representable, where the shared framework
//! would reject or panic on them).

use nvtx_events::{NvtxColor, NvtxPayload};
use quent_time::TimeUnixNanoSec;

/// A stable handle to an [`NvtxSpan`] within one reconstructed model.
///
/// The index into the owning model's span list. Only meaningful against the
/// model it was produced from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SpanId(pub usize);

/// Which NVTX construct a span was reconstructed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// A per-thread nested range: `nvtxDomainRangePushEx` / `nvtxDomainRangePop`.
    PushPop,
    /// A process-wide range keyed by id: `nvtxDomainRangeStartEx` / `nvtxDomainRangeEnd`.
    StartEnd,
    /// A resource lifespan: `nvtxDomainResourceCreate` / `nvtxDomainResourceDestroy`.
    Resource,
}

/// A reconstructed NVTX interval.
///
/// `start <= end` is an invariant established at construction — out-of-order
/// pairs are clamped rather than rejected, so [`Self::duration`] can never
/// underflow.
#[derive(Debug, Clone, PartialEq)]
pub struct NvtxSpan {
    /// Raw domain handle (`0` = default domain).
    pub domain: u64,
    /// Raw OS thread id, when the originating events carried one.
    pub thread_id: Option<u32>,
    /// Resolved message, or a placeholder when the handle was never registered.
    pub name: String,
    /// Raw category id (`None` when the event carried category `0` = none).
    pub category: Option<u32>,
    /// Verbatim color attribute, undecoded.
    pub color: Option<NvtxColor>,
    /// Verbatim payload value, undecoded.
    pub payload: Option<NvtxPayload>,
    /// Interval start.
    pub start: TimeUnixNanoSec,
    /// Interval end. Always `>= start`.
    pub end: TimeUnixNanoSec,
    /// Which NVTX construct this span was reconstructed from.
    pub kind: SpanKind,
    /// The enclosing span, for nested push/pop ranges.
    pub parent: Option<SpanId>,
    /// `true` when the close was never observed and was synthesized at trace end.
    pub synthetic_end: bool,
}

impl NvtxSpan {
    /// The span's duration in nanoseconds.
    ///
    /// Saturating by construction: `start <= end` is an invariant, and the
    /// saturating subtraction keeps a malformed stream from ever underflowing.
    pub fn duration(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
}

/// A reconstructed `nvtxDomainMarkEx` instant.
///
/// The zero-width counterpart to [`NvtxSpan`]: a mark has a timestamp but no
/// duration, so it is deliberately *not* modelled as a zero-length span — a
/// consumer rendering a timeline needs to tell "happened at" from "lasted zero".
#[derive(Debug, Clone, PartialEq)]
pub struct NvtxMark {
    /// Raw domain handle (`0` = default domain).
    pub domain: u64,
    /// Raw OS thread id, when the originating event carried one.
    pub thread_id: Option<u32>,
    /// Resolved message, or a placeholder when the handle was never registered.
    pub name: String,
    /// Raw category id (`None` when the event carried category `0` = none).
    pub category: Option<u32>,
    /// Verbatim color attribute, undecoded.
    pub color: Option<NvtxColor>,
    /// Verbatim payload value, undecoded.
    pub payload: Option<NvtxPayload>,
    /// When the mark was emitted.
    pub timestamp: TimeUnixNanoSec,
}

/// A domain the stream referenced, with its resolved name and lifespan.
///
/// Present for every domain *mentioned* by the stream, not only those with a
/// captured `nvtxDomainCreate` — a domain created before capture began still
/// groups the events that name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NvtxDomain {
    /// Raw domain handle (`0` = default domain).
    pub domain: u64,
    /// Resolved name, or the placeholder for an uncreated handle.
    pub name: String,
    /// The `nvtxDomainCreate` timestamp, or the first time the domain was seen.
    pub created: TimeUnixNanoSec,
    /// The `nvtxDomainDestroy` timestamp, if one was captured.
    pub destroyed: Option<TimeUnixNanoSec>,
}

/// An OS thread the stream referenced, with its resolved name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NvtxThread {
    /// Raw OS thread id (the `nvtxNameOsThread` id space).
    pub thread_id: u32,
    /// Resolved name, or `"thread {id}"` when the thread was never named.
    pub name: String,
}

/// A category the stream referenced, namespaced by its owning domain.
///
/// The `(domain, category)` pair *is* the identity: NVTX category ids are only
/// unique within a domain, so a globally-keyed view would silently merge
/// unrelated categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NvtxCategory {
    /// Raw domain handle the category belongs to.
    pub domain: u64,
    /// Raw category id (never `0` — that is the "no category" sentinel).
    pub category: u32,
    /// Resolved name, or the placeholder for an unnamed category.
    pub name: String,
}
