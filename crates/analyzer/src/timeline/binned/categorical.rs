// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Binned timelines of weighted values keyed by categories.
//!
//! A categorical timeline describes, per opaque series (e.g. an operator in a
//! query engine), how some weighted quantity (a "measure", e.g. an entity
//! count) breaks down over the states of a finite state machine and an
//! application-defined dimension (e.g. which resource holds the entity's
//! data), for each time bin of a window. Bin values are absolute,
//! time-weighted quantities — not normalized shares.
//!
//! This module is application-agnostic: series, measures, states, and
//! dimension keys are opaque to the aggregation. Downstream analyzers decide
//! what they mean and are expected to keep dimension keys a small enumerable
//! set.

use std::hash::Hash;

use rustc_hash::FxHashMap as HashMap;

use quent_time::{SpanNanoSec, bin::BinnedSpan};

use crate::{
    AnalyzerResult,
    timeline::binned::{BinnedTimelineAggregator, KeyedAggregator},
};

/// Identity of one aggregation cell of a categorical timeline.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CategoricalKey<S, M, St, D> {
    /// Opaque series the sample belongs to (e.g. an operator id downstream).
    pub series: S,
    /// The measure this weight contributes to (e.g. an entity count).
    pub measure: M,
    /// The FSM state name during the span.
    pub state: St,
    /// Application-defined dimension key (opaque to the aggregation).
    pub dimension: D,
}

/// A binned timeline of weighted (state, dimension) values for multiple
/// series and measures.
#[derive(Clone, Debug)]
pub struct CategoricalTimeline<S, M, St, D> {
    pub config: BinnedSpan,
    pub data: HashMap<CategoricalKey<S, M, St, D>, Vec<f64>>,
}

/// Builds a [`CategoricalTimeline`] from weighted samples.
///
/// Aggregation is span-weighted: each sample contributes
/// `weight * overlap_fraction` to every bin its span intersects, so bin values
/// are time-weighted averages over the bin, not instantaneous snapshots.
pub struct CategoricalTimelineBuilder<S, M, St, D> {
    aggregator: KeyedAggregator<CategoricalKey<S, M, St, D>>,
}

impl<S, M, St, D> CategoricalTimelineBuilder<S, M, St, D>
where
    S: Eq + Hash,
    M: Eq + Hash,
    St: Eq + Hash,
    D: Eq + Hash,
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
        key: CategoricalKey<S, M, St, D>,
        span: SpanNanoSec,
        weight: f64,
    ) -> AnalyzerResult<()> {
        self.aggregator.try_push(span, (key, weight))
    }

    pub fn build(self) -> CategoricalTimeline<S, M, St, D> {
        CategoricalTimeline {
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
    ) -> CategoricalKey<u32, &'a str, &'a str, &'a str> {
        CategoricalKey {
            series,
            measure,
            state,
            dimension,
        }
    }

    /// Samples differing in any key component land in distinct cells; the
    /// binning math itself is covered by the aggregator's own tests.
    #[test]
    fn distinct_series_measures_states_dimensions() -> AnalyzerResult<()> {
        let mut builder = CategoricalTimelineBuilder::new(test_config());
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

    /// Non-string key components only need `Eq + Hash` — no stringification.
    #[test]
    fn non_string_key_components() -> AnalyzerResult<()> {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        enum Measure {
            Count,
        }

        let mut builder: CategoricalTimelineBuilder<u32, Measure, u8, u16> =
            CategoricalTimelineBuilder::new(test_config());
        let cell = CategoricalKey {
            series: 7u32,
            measure: Measure::Count,
            state: 3u8,
            dimension: 9u16,
        };
        builder.try_push(cell.clone(), SpanNanoSec::try_new(0, 1000).unwrap(), 2.0)?;

        let timeline = builder.build();
        assert_eq!(timeline.data.get(&cell).unwrap()[..], [2.0; 10]);
        Ok(())
    }
}
