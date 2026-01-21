use std::collections::HashMap;

use quent_time::{Span, bin::BinnedSpan};

use crate::Result;

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
    fn try_push(&mut self, span: Span, item: Self::Item) -> Result<()>;

    /// Attempt to return the finished output of this aggregator.
    fn try_finish(self) -> Result<Self::Output>;
}

/// A binned timeline built from numeric primitive values associated with a
/// span.
pub(crate) struct UnitAggregator {
    config: BinnedSpan,
    bins: Vec<f64>,
}

impl UnitAggregator {
    pub(crate) fn new(config: BinnedSpan) -> Self {
        let capacity = config.num_bins().get() as usize;
        Self {
            config,
            bins: std::iter::repeat_with(Default::default)
                .take(capacity)
                .collect(),
        }
    }
}

impl BinnedTimelineAggregator for UnitAggregator {
    type Item = f64;
    type Output = Vec<f64>;

    fn config(&self) -> BinnedSpan {
        self.config
    }

    fn try_push(&mut self, span: Span, item: Self::Item) -> Result<()> {
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
            self.bins[index as usize] += overlap_fraction * item
        }

        Ok(())
    }

    fn try_finish(self) -> Result<Self::Output> {
        Ok(self.bins)
    }
}

/// A binned timeline built from numeric primitive values associated with a
/// span and a name.
pub(crate) struct NamedAggregator<'a> {
    config: BinnedSpan,
    named_bins: HashMap<&'a str, UnitAggregator>,
}

impl NamedAggregator<'_> {
    pub(crate) fn new(config: BinnedSpan) -> Self {
        Self {
            config,
            named_bins: HashMap::new(),
        }
    }
}

impl<'a> BinnedTimelineAggregator for NamedAggregator<'a> {
    type Item = (f64, &'a str);
    type Output = HashMap<&'a str, Vec<f64>>;

    fn config(&self) -> BinnedSpan {
        self.config
    }

    fn try_push(&mut self, span: Span, item: Self::Item) -> Result<()> {
        self.named_bins
            .entry(item.1)
            .or_insert_with(|| UnitAggregator::new(self.config))
            .try_push(span, item.0)
    }

    fn try_finish(self) -> Result<Self::Output> {
        self.named_bins
            .into_iter()
            .map(|(k, v)| v.try_finish().map(|values| (k, values)))
            .collect::<Result<_>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZero;

    #[test]
    fn numeric_primitive_occupancy() -> Result<()> {
        let config =
            BinnedSpan::try_new(Span::try_new(0, 80).unwrap(), NonZero::new(4).unwrap()).unwrap();

        let mut aggregator: UnitAggregator = UnitAggregator::new(config);

        aggregator.try_push(Span::try_new(0, 30).unwrap(), 10.0)?;
        aggregator.try_push(Span::try_new(20, 60).unwrap(), 10.0)?;

        assert_eq!(aggregator.try_finish().unwrap(), [10.0, 15.0, 10.0, 0.0]);

        Ok(())
    }
}
