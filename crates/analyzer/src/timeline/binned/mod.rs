// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Functionality to analyze telemetry to construct timelines with time bins.

use rustc_hash::FxHashMap as HashMap;

use quent_time::{SpanNanoSec, bin::BinnedSpan};

use crate::AnalyzerResult;

pub mod categorical;
pub mod resource;

/// A trait for types that can aggregate items into a sequence of time bins.
pub(crate) trait BinnedTimelineAggregator {
    type Item;
    type Output;

    /// Return the configuration of the binned timeline.
    fn config(&self) -> BinnedSpan;

    /// Attempt to push an item into all bins that intersect with the given time
    /// span.
    ///
    /// # Arguments
    ///
    /// * `span` - The time span that determines which bins should receive the
    ///   item.
    /// * `item` - The item to be pushed into all intersecting bins.
    fn try_push(&mut self, span: SpanNanoSec, item: Self::Item) -> AnalyzerResult<()>;

    /// Attempt to return the finished output of this aggregator.
    fn finish(self) -> Self::Output;
}

/// Preserve a value's unit through weighting, accumulation, and export.
pub(crate) trait BinValue: Copy {
    fn zero(self) -> Self;
    fn add_weighted(&mut self, value: Self, weight: f64);
    fn finish(self) -> f64;
}

impl BinValue for f64 {
    fn zero(self) -> Self {
        0.0
    }

    fn add_weighted(&mut self, value: Self, weight: f64) {
        *self += weight * value;
    }

    fn finish(self) -> f64 {
        self
    }
}

/// A binned timeline that preserves each value's unit during aggregation.
pub(crate) struct UnitAggregator<Value = f64> {
    config: BinnedSpan,
    bins: Vec<Value>,
}

impl<Value: BinValue> UnitAggregator<Value> {
    pub(crate) fn new(config: BinnedSpan, value: Value) -> Self {
        let capacity = config.num_bins().get() as usize;
        Self {
            config,
            bins: vec![value.zero(); capacity],
        }
    }
}

impl<Value: BinValue> BinnedTimelineAggregator for UnitAggregator<Value> {
    type Item = Value;
    type Output = Vec<f64>;

    fn config(&self) -> BinnedSpan {
        self.config
    }

    fn try_push(&mut self, span: SpanNanoSec, item: Self::Item) -> AnalyzerResult<()> {
        // Quickly return if the span duration is zero.
        let span_duration = span.duration();
        if span_duration == 0 {
            return Ok(());
        }
        for (index, intersect_duration) in self.config().iter_indices_intersect_durations(&span) {
            let overlap_fraction =
                intersect_duration as f64 / self.config().bin_duration().get() as f64;
            assert!(overlap_fraction >= 0.0);
            assert!(overlap_fraction <= 1.0);
            self.bins[index as usize].add_weighted(item, overlap_fraction);
        }

        Ok(())
    }

    fn finish(self) -> Self::Output {
        self.bins.into_iter().map(BinValue::finish).collect()
    }
}

/// A binned timeline built from numeric primitive values associated with a
/// span and a name.
pub(crate) struct KeyedAggregator<Key> {
    config: BinnedSpan,
    bins: HashMap<Key, UnitAggregator>,
}

impl<Key> KeyedAggregator<Key> {
    pub(crate) fn new(config: BinnedSpan) -> Self {
        Self {
            config,
            bins: HashMap::default(),
        }
    }
}

impl<Key> BinnedTimelineAggregator for KeyedAggregator<Key>
where
    Key: Eq + std::hash::Hash,
{
    type Item = (Key, f64);
    type Output = HashMap<Key, Vec<f64>>;

    fn config(&self) -> BinnedSpan {
        self.config
    }

    fn try_push(&mut self, span: SpanNanoSec, item: Self::Item) -> AnalyzerResult<()> {
        self.bins
            .entry(item.0)
            .or_insert_with(|| UnitAggregator::new(self.config, item.1))
            .try_push(span, item.1)
    }

    fn finish(self) -> Self::Output {
        self.bins
            .into_iter()
            .map(|(k, v)| (k, v.finish()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZero;

    #[test]
    fn unit_aggregator() -> AnalyzerResult<()> {
        let config = BinnedSpan::try_new(
            SpanNanoSec::try_new(0, 80).unwrap(),
            NonZero::new(4).unwrap(),
        )
        .unwrap();

        let mut aggregator: UnitAggregator = UnitAggregator::new(config, 0.0);

        aggregator.try_push(SpanNanoSec::try_new(0, 30).unwrap(), 10.0)?;
        aggregator.try_push(SpanNanoSec::try_new(20, 60).unwrap(), 10.0)?;

        assert_eq!(aggregator.finish(), [10.0, 15.0, 10.0, 0.0]);

        Ok(())
    }

    fn ten_bin_config() -> BinnedSpan {
        BinnedSpan::try_new(
            SpanNanoSec::try_new(0, 1000).unwrap(),
            NonZero::new(10).unwrap(),
        )
        .unwrap()
    }

    /// Overlapping spans accumulate span-weighted fractions per bin.
    #[test]
    fn keyed_aggregator_span_weighting_across_bin_boundaries() -> AnalyzerResult<()> {
        let mut aggregator: KeyedAggregator<&str> = KeyedAggregator::new(ten_bin_config());

        aggregator.try_push(SpanNanoSec::try_new(0, 300).unwrap(), ("k", 1.0))?;
        aggregator.try_push(SpanNanoSec::try_new(250, 450).unwrap(), ("k", 1.0))?;

        let bins = aggregator.finish();
        assert_eq!(
            bins.get("k").unwrap()[..],
            [1.0, 1.0, 1.5, 1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
        Ok(())
    }

    /// Zero-duration spans create the key but contribute nothing.
    #[test]
    fn keyed_aggregator_zero_duration_span_is_noop() -> AnalyzerResult<()> {
        let mut aggregator: KeyedAggregator<&str> = KeyedAggregator::new(ten_bin_config());
        aggregator.try_push(SpanNanoSec::try_new(500, 500).unwrap(), ("k", 1.0))?;

        let bins = aggregator.finish();
        assert_eq!(bins.get("k").unwrap()[..], [0.0; 10]);
        Ok(())
    }

    /// Spans entirely outside the window contribute nothing.
    #[test]
    fn keyed_aggregator_out_of_window_span_contributes_nothing() -> AnalyzerResult<()> {
        let mut aggregator: KeyedAggregator<&str> = KeyedAggregator::new(ten_bin_config());
        aggregator.try_push(SpanNanoSec::try_new(2000, 3000).unwrap(), ("k", 1.0))?;

        let bins = aggregator.finish();
        assert_eq!(bins.get("k").unwrap()[..], [0.0; 10]);
        Ok(())
    }
}
