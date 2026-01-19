use quent_attributes::Value;
use quent_entities::resource::CapacityKind;
use quent_time::{Span, bin::BinnedSpan};

use crate::{Result, error::Error};

/// Trait to describe a bin of time into which any item can be pushed.
pub(crate) trait TimeBin {
    /// The type of item that can be pushed into this time bin.
    type Item;

    /// Attempt to push an item into this time bin.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to be added to this time bin.
    ///
    /// * `overlap_fraction` - A value between 0.0 and 1.0 indicating what
    ///   fraction of the item's time span overlaps with this bin's time range. A
    ///   value of 1.0 means the item is fully contained within the bin,
    ///   potentially exceeding it, while smaller values indicate partial overlap.
    ///
    /// * `span` - The total span of time related to the item being pushed.
    fn try_push(&mut self, item: Self::Item, overlap_fraction: f64, span: &Span) -> Result<()>;
}

impl TimeBin for f64 {
    type Item = (Value, CapacityKind);

    /// Pushes a numeric [`Value`] associated with a [`Span`] into this time
    /// bin, weighted by overlap fraction.
    ///
    /// If the [`CapacityKind`] is [`CapacityKind::Rate`], the [`Value`] is
    /// divided by the span duration.
    ///
    /// # Errors Returns an error if the [`Value`] cannot be converted to f64.
    fn try_push(&mut self, item: Self::Item, overlap_fraction: f64, span: &Span) -> Result<()> {
        *self += match item.1 {
            CapacityKind::Occupancy => {
                let occupancy = f64::try_from(item.0)
                    .map_err(|e| Error::ValueType(format!("cannot convert value to f64: {e}")))?;
                overlap_fraction * occupancy
            }
            CapacityKind::Rate => {
                let work = f64::try_from(item.0)
                    .map_err(|e| Error::ValueType(format!("cannot convert value to f64: {e}")))?;
                let rate = work / span.duration() as f64;
                overlap_fraction * rate
            }
        };
        Ok(())
    }
}

/// A trait for types that can aggregate items into [`TimeBin`]s.
pub(crate) trait BinnedTimelineAggregator {
    type Bin;
    type Output;

    /// Return the configuration of the binned timeline.
    fn config(&self) -> BinnedSpan;
    /// Return a mutable slice of all bins.
    fn bins_mut(&mut self) -> &mut [Self::Bin];

    /// Attempt to push an item into all bins that intersect with the given time
    /// span.
    ///
    /// # Arguments
    ///
    /// * `span` - The time span that determines which bins should receive the
    ///   item.
    /// * `item` - The item to be pushed into all intersecting bins.
    ///
    /// # Default Implementation
    ///
    /// A blanket implementation is provided when the `BinType` implements
    /// [`TimeBin`] and the item can be [`Clone`]d.
    fn try_push(&mut self, span: Span, item: <Self::Bin as TimeBin>::Item) -> Result<()>
    where
        Self::Bin: TimeBin,
        <Self::Bin as TimeBin>::Item: Clone,
    {
        for (index, intersect_duration) in self.config().iter_indices_intersect_durations(&span) {
            let overlap_fraction =
                intersect_duration as f64 / self.config().bin_duration().get() as f64;
            self.bins_mut()[index as usize].try_push(item.clone(), overlap_fraction, &span)?;
        }

        Ok(())
    }

    /// Attempt to return the finished output of this aggregator.
    fn try_finish(self) -> Result<Self::Output>;
}

/// A binned timeline built from numeric primitive values associated with a
/// span.
pub(crate) struct NumericPrimitiveBinnedTimeline {
    config: BinnedSpan,
    bins: Vec<f64>,
}

impl NumericPrimitiveBinnedTimeline {
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

impl BinnedTimelineAggregator for NumericPrimitiveBinnedTimeline {
    type Bin = f64;
    type Output = Vec<f64>;

    fn config(&self) -> BinnedSpan {
        self.config
    }
    fn bins_mut(&mut self) -> &mut [f64] {
        &mut self.bins
    }
    fn try_finish(self) -> Result<Self::Output> {
        Ok(self.bins)
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

        let mut aggregator: NumericPrimitiveBinnedTimeline =
            NumericPrimitiveBinnedTimeline::new(config);

        aggregator.try_push(
            Span::try_new(0, 30).unwrap(),
            (Value::U64(10), CapacityKind::Occupancy),
        )?;
        aggregator.try_push(
            Span::try_new(20, 60).unwrap(),
            (Value::U64(10), CapacityKind::Occupancy),
        )?;

        assert_eq!(aggregator.try_finish().unwrap(), [10.0, 15.0, 10.0, 0.0]);

        Ok(())
    }

    #[test]
    fn numeric_primitive_rate() -> Result<()> {
        let config =
            BinnedSpan::try_new(Span::try_new(0, 80).unwrap(), NonZero::new(4).unwrap()).unwrap();

        let mut aggregator: NumericPrimitiveBinnedTimeline =
            NumericPrimitiveBinnedTimeline::new(config);

        aggregator.try_push(
            Span::try_new(0, 30).unwrap(),
            (Value::U64(30), CapacityKind::Rate), // rate=1/sec
        )?;
        aggregator.try_push(
            Span::try_new(20, 60).unwrap(),
            (Value::U64(40), CapacityKind::Rate), // rate=1/sec
        )?;

        assert_eq!(aggregator.try_finish().unwrap(), [1.0, 1.5, 1.0, 0.0]);

        Ok(())
    }
}
