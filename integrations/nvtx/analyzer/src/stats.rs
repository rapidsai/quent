// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Range statistics — a fold over the reconstructed span set (ANA-06).
//!
//! Groups completed *range* spans by `(name, domain, category)` and reports
//! count and total/average/minimum/maximum duration per group. All three key
//! components are needed: the same name means different work in different
//! domains, and the same name under different categories is deliberately
//! distinguished by the instrumented application.
//!
//! Two modelling choices are worth stating outright:
//!
//! - **Only ranges participate.** Marks are instants — they have no duration to
//!   average — and resource lifespans measure how long a handle existed, not how
//!   long any work took. Folding either into "range statistics" would produce a
//!   number that reads like a duration but answers a different question.
//! - **Synthetically-closed spans are counted, but counted separately too.** A
//!   range left open at trace end has a real start and an *inferred* end, so its
//!   duration is a lower bound rather than a measurement. Dropping it would
//!   understate the count; folding it in silently would overstate the confidence.
//!   It contributes to every figure and also to
//!   [`RangeStats::synthetic_count`], so a consumer can tell how much of a group
//!   is inferred.

use std::collections::BTreeMap;

use crate::span::{NvtxSpan, SpanKind};

/// What one [`RangeStats`] group is keyed by.
///
/// `Ord` rather than `Hash`: the statistics come back in a [`BTreeMap`], so
/// repeated builds of the same stream iterate in the same order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatsKey {
    /// The resolved span name (or its placeholder).
    pub name: String,
    /// Raw domain handle (`0` = default domain).
    pub domain: u64,
    /// Raw category id, or `None` for NVTX's "no category" sentinel.
    pub category: Option<u32>,
}

/// Aggregated durations for one `(name, domain, category)` group, in nanoseconds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RangeStats {
    /// How many spans fell into this group.
    pub count: u64,
    /// Sum of every span's duration.
    pub total_duration: u64,
    /// `total_duration / count`, or `0` for an empty group.
    pub avg_duration: u64,
    /// The shortest span's duration.
    pub min_duration: u64,
    /// The longest span's duration.
    pub max_duration: u64,
    /// How many of [`Self::count`] were closed at trace end rather than observed.
    ///
    /// Their durations are lower bounds, so this is what tells a consumer how
    /// much of the group is inferred rather than measured.
    pub synthetic_count: u64,
}

impl RangeStats {
    /// Fold one span into this group.
    fn accumulate(&mut self, span: &NvtxSpan) {
        // `start <= end` is a span invariant, so this is a real duration; zero
        // is legal and contributes zero.
        let duration = span.duration();

        if self.count == 0 {
            // A zeroed `min` would win every comparison, so the first span seeds
            // both bounds rather than being compared against them.
            self.min_duration = duration;
            self.max_duration = duration;
        } else {
            self.min_duration = self.min_duration.min(duration);
            self.max_duration = self.max_duration.max(duration);
        }

        self.count += 1;
        // Saturating: a stream of adversarial timestamps must not overflow the
        // accumulator, and a saturated total is a visibly wrong number rather
        // than a silently wrapped plausible one.
        self.total_duration = self.total_duration.saturating_add(duration);
        if span.synthetic_end {
            self.synthetic_count += 1;
        }
    }

    /// Compute the derived average, once the group is complete.
    ///
    /// `checked_div` rather than a bare `/`: an empty group is representable
    /// (nothing forbids constructing one), and a division by zero here would be
    /// the single arithmetic panic in an otherwise total reconstruction path.
    fn finish(&mut self) {
        self.avg_duration = self.total_duration.checked_div(self.count).unwrap_or(0);
    }
}

/// Aggregate every range span in `spans` by `(name, domain, category)`.
///
/// Push/pop and start/end spans participate; marks never reach here (they are
/// not spans at all) and resource lifespans are filtered out — see the module
/// docs for why.
pub(crate) fn range_statistics(spans: &[NvtxSpan]) -> BTreeMap<StatsKey, RangeStats> {
    let mut grouped: BTreeMap<StatsKey, RangeStats> = BTreeMap::new();

    for span in spans
        .iter()
        .filter(|span| matches!(span.kind, SpanKind::PushPop | SpanKind::StartEnd))
    {
        grouped
            .entry(StatsKey {
                name: span.name.clone(),
                domain: span.domain,
                category: span.category,
            })
            .or_default()
            .accumulate(span);
    }

    // `avg` is derived, so it is computed once per group after the fold rather
    // than recomputed on every span.
    for stats in grouped.values_mut() {
        stats.finish();
    }

    grouped
}
