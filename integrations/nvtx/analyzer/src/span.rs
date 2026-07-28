// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The reconstruction core's own plain span types.
//!
//! These are hand-written structs, not framework types: no `RtFsm`, no
//! `quent-model` entity, no macro DSL. A reconstructed NVTX range *is* a span —
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
