// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! What the stream did that reconstruction could not represent.
//!
//! Two shapes, both invisible in the span list. A close with no matching open
//! produces no span at all — and the drops are not a random sample, they are the
//! ranges straddling the moment capture attached, which skews long. A key reused
//! while still open produces a span, but one whose end can never be known, and
//! nothing on the span distinguishes that from an ordinary detach.

/// Counts of what the stream did that the span list does not show.
///
/// Non-zero does not mean the capture is broken. Attaching to a running process
/// routinely truncates the head of a trace; reused keys point at a malformed or
/// wrongly merged stream. Both mean [`spans`](crate::NvtxModel::spans) is not
/// the whole story, which is the only claim this type makes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReconstructionAnomalies {
    /// `RangeEnd`s with no open `RangeStart`.
    pub orphan_range_ends: u64,
    /// `RangePop`s with no open `RangePush` on their `(thread, domain)` stack.
    pub orphan_range_pops: u64,
    /// `ResourceDestroy`s with no open `ResourceCreate`.
    pub orphan_resource_destroys: u64,
    /// `RangeStart`s that reused an id already open, displacing it.
    ///
    /// NVTX draws range ids from one process-global counter, so a live id
    /// cannot legitimately be restarted. The displaced range still reconstructs,
    /// with no end.
    pub reused_range_ids: u64,
    /// `ResourceCreate`s that reused a handle already open, displacing it.
    pub reused_resource_handles: u64,
}

impl ReconstructionAnomalies {
    /// Whether the stream reconstructed without anomaly.
    ///
    /// `true` says every close matched an open and no key was reused. It says
    /// nothing about whether the spans were *measured* — a capture can stop
    /// cleanly with ranges still running, which leaves
    /// [`NvtxSpan::end`](crate::NvtxSpan::end) `None` and is not an anomaly.
    pub fn is_faithful(&self) -> bool {
        self.total() == 0
    }

    /// How many events reconstruction could not represent faithfully.
    pub fn total(&self) -> u64 {
        self.orphan_range_ends
            .saturating_add(self.orphan_range_pops)
            .saturating_add(self.orphan_resource_destroys)
            .saturating_add(self.reused_range_ids)
            .saturating_add(self.reused_resource_handles)
    }
}
