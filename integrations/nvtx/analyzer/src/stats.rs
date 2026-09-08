// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Range statistics — a fold over the reconstructed span set.
//!
//! Groups range spans by `(name, domain, category)` — all three, since a name
//! means different work in different domains and the application distinguishes
//! categories deliberately.
//!
//! Only ranges participate. A mark has no duration and a resource lifespan
//! measures existence rather than work, so folding either in yields a number
//! that reads like a duration but answers a different question.
//!
//! Duration figures cover **observed** closes only. A span whose close was never
//! captured has no duration to fold in — inventing one would report inference as
//! measurement — so it raises `count` without raising `observed_count`.

use std::collections::BTreeMap;

use rustc_hash::FxHashMap as HashMap;

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
///
/// Duration fields cover observed closes only — compare
/// [`observed_count`](Self::observed_count) against [`count`](Self::count) to
/// see how much of the group they speak for. The difference is the number of
/// spans whose close was never captured; [`NvtxModel::anomalies`] says whether
/// any of that was a key reused mid-flight rather than a capture that stopped.
///
/// [`NvtxModel::anomalies`]: crate::NvtxModel::anomalies
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RangeStats {
    /// How many spans fell into this group, measured or not.
    pub count: u64,
    /// How many contributed to the duration figures below.
    pub observed_count: u64,
    /// Sum of the observed durations.
    pub total_duration: u64,
    /// `total_duration / observed_count`, or `0` when nothing was observed.
    pub avg_duration: u64,
    /// The shortest observed duration.
    pub min_duration: u64,
    /// The longest observed duration.
    pub max_duration: u64,
    /// Whether [`Self::total_duration`] hit the `u64` ceiling and stopped being
    /// a sum. Only an adversarial or corrupt stream reaches this.
    pub saturated: bool,
}

impl RangeStats {
    /// Fold one span into this group.
    fn accumulate(&mut self, span: &NvtxSpan) {
        self.count += 1;

        // A span with no observed close has no duration to fold in. It stays in
        // `count`, so the gap between the two counts is what says the figures
        // below do not speak for the whole group.
        let Some(duration) = span.duration() else {
            return;
        };

        if self.observed_count == 0 {
            // A zeroed `min` would win every comparison, so the first span seeds
            // both bounds rather than being compared against them.
            self.min_duration = duration;
            self.max_duration = duration;
        } else {
            self.min_duration = self.min_duration.min(duration);
            self.max_duration = self.max_duration.max(duration);
        }
        self.observed_count += 1;

        match self.total_duration.checked_add(duration) {
            Some(total) => self.total_duration = total,
            // Saturate, but say so: a silently wrapped total would be a
            // plausible-looking wrong number.
            None => {
                self.total_duration = u64::MAX;
                self.saturated = true;
            }
        }
    }

    /// Compute the derived average, once the group is complete.
    ///
    /// `checked_div` because a group can be entirely unobserved closes, and this
    /// would otherwise be the one arithmetic panic in a total path.
    fn finish(&mut self) {
        self.avg_duration = self
            .total_duration
            .checked_div(self.observed_count)
            .unwrap_or(0);
    }
}

/// Aggregate every range span in `spans` by `(name, domain, category)`.
pub(crate) fn range_statistics(spans: &[NvtxSpan]) -> BTreeMap<StatsKey, RangeStats> {
    range_statistics_where(spans, |_| true)
}

/// Aggregate the range spans in `spans` that `keep` accepts.
///
/// Whole-capture figures beside a windowed chart invite the reader to relate
/// two numbers drawn from different populations.
pub(crate) fn range_statistics_where(
    spans: &[NvtxSpan],
    keep: impl Fn(&NvtxSpan) -> bool,
) -> BTreeMap<StatsKey, RangeStats> {
    // Folded against borrowed names, then keyed by owned ones once per group.
    // Grouping straight into the `BTreeMap` would clone every span's name only
    // to drop it again on the already-present path.
    let mut grouped: HashMap<(&str, u64, Option<u32>), RangeStats> = HashMap::default();

    for span in spans
        .iter()
        .filter(|span| matches!(span.kind, SpanKind::PushPop { .. } | SpanKind::StartEnd))
        .filter(|span| keep(span))
    {
        grouped
            .entry((span.name.as_str(), span.domain, span.category))
            .or_default()
            .accumulate(span);
    }

    grouped
        .into_iter()
        .map(|((name, domain, category), mut stats)| {
            // `avg` is derived, so it is computed once per group rather than
            // recomputed on every span.
            stats.finish();
            let key = StatsKey {
                name: name.to_owned(),
                domain,
                category,
            };
            (key, stats)
        })
        .collect()
}
