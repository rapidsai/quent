// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Binned timelines of weighted distributions over (state, dimension) pairs.
//!
//! A distribution timeline describes, per opaque series (e.g. an operator in a
//! query engine), how some weighted quantity (a "measure", e.g. an entity
//! count) is distributed over the states of a finite state machine and an
//! application-defined dimension (e.g. which resource holds the entity's
//! data), for each time bin of a window.
//!
//! This module is application-agnostic: series, measures, states, and
//! dimension keys are all opaque to the aggregation. Downstream analyzers
//! decide what they mean and are expected to keep dimension keys a small
//! enumerable set.

use std::hash::Hash;

use rustc_hash::FxHashMap as HashMap;

use quent_time::{SpanNanoSec, bin::BinnedSpan};

use crate::{
    AnalyzerResult,
    timeline::binned::{BinnedTimelineAggregator, KeyedAggregator},
};

/// Identity of one aggregation cell of a distribution timeline.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DistributionKey<'a, S> {
    /// Opaque series the sample belongs to (e.g. an operator id downstream).
    pub series: S,
    /// The measure this weight contributes to (e.g. an entity count).
    pub measure: &'a str,
    /// The FSM state name during the span.
    pub state: &'a str,
    /// Application-defined dimension key (opaque to the aggregation).
    pub dimension: &'a str,
}

/// A binned timeline of weighted (state, dimension) distributions for
/// multiple series and measures.
#[derive(Clone, Debug)]
pub struct DistributionTimeline<'a, S> {
    pub config: BinnedSpan,
    pub data: HashMap<DistributionKey<'a, S>, Vec<f64>>,
}

/// Builds a [`DistributionTimeline`] from weighted samples.
///
/// Aggregation is span-weighted: each sample contributes
/// `weight * overlap_fraction` to every bin its span intersects, so bin values
/// are time-weighted averages over the bin, not instantaneous snapshots.
pub struct DistributionTimelineBuilder<'a, S> {
    aggregator: KeyedAggregator<DistributionKey<'a, S>>,
}

impl<'a, S> DistributionTimelineBuilder<'a, S>
where
    S: Eq + Hash + Clone,
{
    pub fn new(config: BinnedSpan) -> Self {
        Self {
            aggregator: KeyedAggregator::new(config),
        }
    }

    /// Return the configuration of the binned timeline.
    pub fn config(&self) -> BinnedSpan {
        self.aggregator.config()
    }

    /// Attempt to push one weighted sample spanning `span` into the timeline.
    pub fn try_push(
        &mut self,
        key: DistributionKey<'a, S>,
        span: SpanNanoSec,
        weight: f64,
    ) -> AnalyzerResult<()> {
        self.aggregator.try_push(span, (key, weight))
    }

    pub fn build(self) -> DistributionTimeline<'a, S> {
        DistributionTimeline {
            config: self.aggregator.config(),
            data: self.aggregator.finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use super::*;

    fn test_config() -> BinnedSpan {
        BinnedSpan::try_new(
            SpanNanoSec::try_new(0, 1000).unwrap(),
            NonZero::try_from(10).unwrap(),
        )
        .unwrap()
    }

    fn key<'a>(
        series: u32,
        measure: &'a str,
        state: &'a str,
        dimension: &'a str,
    ) -> DistributionKey<'a, u32> {
        DistributionKey {
            series,
            measure,
            state,
            dimension,
        }
    }

    #[test]
    fn span_weighting_across_bin_boundaries() -> AnalyzerResult<()> {
        let mut builder = DistributionTimelineBuilder::new(test_config());

        // Spans [0, 300) and [250, 450) of weight 1 each.
        builder.try_push(key(1, "count", "a", "x"), SpanNanoSec::try_new(0, 300).unwrap(), 1.0)?;
        builder.try_push(
            key(1, "count", "a", "x"),
            SpanNanoSec::try_new(250, 450).unwrap(),
            1.0,
        )?;

        let timeline = builder.build();
        let bins = timeline.data.get(&key(1, "count", "a", "x")).unwrap();
        assert_eq!(
            bins[..],
            [1.0, 1.0, 1.5, 1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
        Ok(())
    }

    #[test]
    fn distinct_series_measures_states_dimensions() -> AnalyzerResult<()> {
        let mut builder = DistributionTimelineBuilder::new(test_config());
        let span = SpanNanoSec::try_new(0, 1000).unwrap();

        builder.try_push(key(1, "count", "a", "x"), span, 1.0)?;
        builder.try_push(key(2, "count", "a", "x"), span, 1.0)?;
        builder.try_push(key(1, "bytes", "a", "x"), span, 100.0)?;
        builder.try_push(key(1, "count", "b", "x"), span, 1.0)?;
        builder.try_push(key(1, "count", "a", "y"), span, 1.0)?;

        let timeline = builder.build();
        assert_eq!(timeline.data.len(), 5);
        assert_eq!(
            timeline.data.get(&key(1, "bytes", "a", "x")).unwrap()[..],
            [100.0; 10]
        );
        assert_eq!(
            timeline.data.get(&key(2, "count", "a", "x")).unwrap()[..],
            [1.0; 10]
        );
        Ok(())
    }

    #[test]
    fn zero_duration_span_is_noop() -> AnalyzerResult<()> {
        let mut builder = DistributionTimelineBuilder::new(test_config());
        builder.try_push(
            key(1, "count", "a", "x"),
            SpanNanoSec::try_new(500, 500).unwrap(),
            1.0,
        )?;

        let timeline = builder.build();
        // The key exists (aggregator was created) but all bins remain zero.
        let bins = timeline.data.get(&key(1, "count", "a", "x")).unwrap();
        assert_eq!(bins[..], [0.0; 10]);
        Ok(())
    }

    #[test]
    fn out_of_window_span_contributes_nothing() -> AnalyzerResult<()> {
        let mut builder = DistributionTimelineBuilder::new(test_config());
        builder.try_push(
            key(1, "count", "a", "x"),
            SpanNanoSec::try_new(2000, 3000).unwrap(),
            1.0,
        )?;

        let timeline = builder.build();
        let bins = timeline.data.get(&key(1, "count", "a", "x")).unwrap();
        assert_eq!(bins[..], [0.0; 10]);
        Ok(())
    }
}
