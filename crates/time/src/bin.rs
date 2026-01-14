//! Functionality useful to bin spans of time.

use std::num::NonZero;

use py_rs::PY;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Duration, Result, Span, TimeError, Timestamp};

/// A span of time separated into equally-sized bins of time.
#[derive(TS, PY, Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BinnedSpan {
    // The entire span of time this binned span represents.
    span: Span,
    // The duration of one bin.
    bin_duration: NonZero<Duration>,
    // The number of bins.
    //
    // This is a u64, because a u64::MAX duration span could be binned into
    // u64::MAX bins.
    num_bins: NonZero<u64>,
}

impl BinnedSpan {
    /// Construct a BinnedSpan.
    ///
    /// This function will fail if the duration of the `span` is zero.
    ///
    /// If the duration of the `span` is smaller than the number of bins, the bins will be of size 1.
    /// This function will modify the `span` in which the bins lie such that it always fits within the expressable `[Timestamp]` range.
    pub fn try_new(span: Span, num_bins: NonZero<u64>) -> Result<Self> {
        // Calculate a guaranteed non-zero positive bin size. This can fail if the span duration is zero.
        let bin_size =
            NonZero::try_from(span.duration().div_ceil(num_bins.get())).map_err(|_| {
                TimeError::InvalidArguments(format!(
                    "BinnedSpan cannot be constructed from zero-duration span: {span:?}"
                ))
            })?;
        // Recalculate the end such that the bin will not exceed u64::MAX.
        let end = span
            .start()
            .saturating_add(num_bins.get().saturating_mul(bin_size.get()));
        // Recalculate the start in case the end saturated.
        let start = end.saturating_sub(num_bins.get().saturating_mul(bin_size.get()));
        // Construct the actual span from the adjusted interval.
        let span = Span::try_new(start, end)?;
        // Sanity check.
        assert_eq!(
            span.duration(),
            num_bins.get().saturating_mul(bin_size.get())
        );

        Ok(Self {
            span,
            bin_duration: bin_size,
            num_bins,
        })
    }

    /// Return the index of the bin in which the provided timestamp lies, if at all.
    pub fn index_of(&self, timestamp: Timestamp) -> Option<u64> {
        if let Some(relative_timestamp) = timestamp.checked_sub(self.span.start) {
            let maybe_index = relative_timestamp / self.bin_duration.get();
            if maybe_index >= self.num_bins.get() {
                None
            } else {
                Some(maybe_index)
            }
        } else {
            None
        }
    }

    /// Return the Span of the bin with the provided index.
    pub fn bin(&self, index: u64) -> Option<Span> {
        if index < self.num_bins.get() {
            let start = self.span.start + index * self.bin_duration.get();
            // Unwrap here because if this would ever panic something is really wrong.
            Some(Span::try_new(start, start + self.bin_duration.get()).unwrap())
        } else {
            None
        }
    }

    /// Return an iterator over bin indices that the provided span overlaps.
    pub fn iter_indices(&self, span: &Span) -> impl Iterator<Item = u64> {
        let start_end = (
            self.index_of(span.start()),
            span.end().checked_sub(1).and_then(|t| self.index_of(t)),
        );
        match start_end {
            (Some(start_idx), Some(end_idx)) => start_idx..end_idx + 1,
            (Some(start_idx), None) => start_idx..self.num_bins.get(),
            (None, Some(end_idx)) => 0..end_idx + 1,
            (None, None) => {
                // The start and end timestamps are not within this span. This
                // could mean two things:
                if self.span.during(span) {
                    // 1. self's span is completely contained within the
                    // provided span
                    0..self.num_bins.get()
                } else {
                    // 2. self's span has no overlap with the provided span at
                    // all
                    0..0
                }
            }
        }
    }

    /// Return an iterator over a pair of (bin index, the intersection duration
    /// in this bin) of bins where `span` intersects.
    pub fn iter_indices_intersect_durations(
        &self,
        span: &Span,
    ) -> impl Iterator<Item = (u64, Duration)> {
        // TODO(johanpel): this could be optimized by only calling overlap for the first and last bin
        self.iter_indices(span).map(|index| {
            (
                index,
                self.bin(index)
                    .unwrap()
                    .intersection(span)
                    .unwrap()
                    .duration(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new() {
        // Trivial arguments
        let bins = BinnedSpan::try_new(
            Span::try_new(100, 200).unwrap(),
            NonZero::try_from(5).unwrap(),
        );
        assert!(bins.is_ok());
        let bins = bins.unwrap();
        assert_eq!(bins.num_bins, NonZero::try_from(5).unwrap());
        assert_eq!(bins.bin_duration, NonZero::try_from(20).unwrap());
        assert_eq!(bins.span, Span::try_new(100, 200).unwrap());

        // More bins than time steps
        let bins = BinnedSpan::try_new(
            Span::try_new(10, 20).unwrap(),
            NonZero::try_from(15).unwrap(),
        );
        assert!(bins.is_ok());
        let bins = bins.unwrap();
        assert_eq!(bins.num_bins, NonZero::try_from(15).unwrap());
        assert_eq!(bins.bin_duration, NonZero::try_from(1).unwrap());
        assert_eq!(bins.span, Span::try_new(10, 25).unwrap());

        // More bins than time steps near zero
        let bins = BinnedSpan::try_new(
            Span::try_new(0, 10).unwrap(),
            NonZero::try_from(15).unwrap(),
        );
        assert!(bins.is_ok());
        let bins = bins.unwrap();
        assert_eq!(bins.num_bins, NonZero::try_from(15).unwrap());
        assert_eq!(bins.bin_duration, NonZero::try_from(1).unwrap());
        assert_eq!(bins.span, Span::try_new(0, 15).unwrap());

        // More bins than time steps near max
        let bins = BinnedSpan::try_new(
            Span::try_new(u64::MAX - 10, u64::MAX).unwrap(),
            NonZero::try_from(15).unwrap(),
        );
        assert!(bins.is_ok());
        let bins = bins.unwrap();
        assert_eq!(bins.num_bins, NonZero::try_from(15).unwrap());
        assert_eq!(bins.bin_duration, NonZero::try_from(1).unwrap());
        assert_eq!(bins.span, Span::try_new(u64::MAX - 15, u64::MAX).unwrap());
    }

    #[test]
    pub fn bin() {
        let bins = BinnedSpan::try_new(
            Span::try_new(100, 200).unwrap(),
            NonZero::try_from(4).unwrap(),
        )
        .unwrap();
        assert_eq!(bins.bin(0), Some(Span::try_new(100, 125).unwrap()));
        assert_eq!(bins.bin(1), Some(Span::try_new(125, 150).unwrap()));
        assert_eq!(bins.bin(2), Some(Span::try_new(150, 175).unwrap()));
        assert_eq!(bins.bin(3), Some(Span::try_new(175, 200).unwrap()));
        assert_eq!(bins.bin(4), None);
        assert_eq!(bins.bin(u64::MAX), None);
    }

    #[test]
    pub fn index_of() {
        // Trivial
        let bins = BinnedSpan::try_new(
            Span::try_new(100, 200).unwrap(),
            NonZero::try_from(4).unwrap(),
        )
        .unwrap();
        assert_eq!(bins.index_of(0), None);
        assert_eq!(bins.index_of(99), None);
        assert_eq!(bins.index_of(100), Some(0));
        assert_eq!(bins.index_of(124), Some(0));
        assert_eq!(bins.index_of(125), Some(1));
        assert_eq!(bins.index_of(199), Some(3));
        assert_eq!(bins.index_of(200), None);

        // Extremes
        let bins = BinnedSpan::try_new(
            Span::try_new(0, u64::MAX).unwrap(),
            NonZero::try_from(1024).unwrap(),
        )
        .unwrap();
        assert_eq!(bins.index_of(0), Some(0));
        assert_eq!(bins.index_of(u64::MAX / 2 + 1), Some(512));
        // TODO(johanpel): passing u64::MAX reveals an issue. Since we're using
        // half open intervals, u64::MAX can't really be a valid time step in
        // this u64 nanosecond universe in which things that happen can be made
        // sense of, since u64 is pretty much like infinity. Possibly add a
        // newtype for timestamps excluding this time step.
        assert_eq!(bins.index_of(u64::MAX - 1), Some(1023));
    }

    #[test]
    pub fn iter_indices() {
        // Span is same as binned span
        let bins = BinnedSpan::try_new(
            Span::try_new(100, 200).unwrap(),
            NonZero::try_from(4).unwrap(),
        )
        .unwrap();
        assert_eq!(
            bins.iter_indices(&Span::try_new(100, 200).unwrap())
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );

        // Span is smaller than binned span
        let bins = BinnedSpan::try_new(
            Span::try_new(100, 200).unwrap(),
            NonZero::try_from(4).unwrap(),
        )
        .unwrap();
        assert_eq!(
            bins.iter_indices(&Span::try_new(125, 149).unwrap())
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            bins.iter_indices(&Span::try_new(125, 150).unwrap())
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            bins.iter_indices(&Span::try_new(124, 150).unwrap())
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            bins.iter_indices(&Span::try_new(124, 151).unwrap())
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        // Span is larger than binned span
        let bins = BinnedSpan::try_new(
            Span::try_new(100, 200).unwrap(),
            NonZero::try_from(4).unwrap(),
        )
        .unwrap();
        assert_eq!(
            bins.iter_indices(&Span::try_new(0, 300).unwrap())
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(
            bins.iter_indices(&Span::try_new(99, 150).unwrap())
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            bins.iter_indices(&Span::try_new(99, 151).unwrap())
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            bins.iter_indices(&Span::try_new(150, 200).unwrap())
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert_eq!(
            bins.iter_indices(&Span::try_new(150, 300).unwrap())
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    pub fn iter_indices_intersect_durations() {
        // Span is same as binned span
        let bins = BinnedSpan::try_new(
            Span::try_new(100, 200).unwrap(),
            NonZero::try_from(4).unwrap(),
        )
        .unwrap();
        assert_eq!(
            bins.iter_indices_intersect_durations(&Span::try_new(100, 200).unwrap())
                .collect::<Vec<_>>(),
            vec![(0, 25), (1, 25), (2, 25), (3, 25)]
        );

        // Span is smaller than binned span
        let bins = BinnedSpan::try_new(
            Span::try_new(100, 200).unwrap(),
            NonZero::try_from(4).unwrap(),
        )
        .unwrap();
        assert_eq!(
            bins.iter_indices_intersect_durations(&Span::try_new(125, 149).unwrap())
                .collect::<Vec<_>>(),
            vec![(1, 24)]
        );
        assert_eq!(
            bins.iter_indices_intersect_durations(&Span::try_new(125, 150).unwrap())
                .collect::<Vec<_>>(),
            vec![(1, 25)]
        );
        assert_eq!(
            bins.iter_indices_intersect_durations(&Span::try_new(124, 150).unwrap())
                .collect::<Vec<_>>(),
            vec![(0, 1), (1, 25)]
        );
        assert_eq!(
            bins.iter_indices_intersect_durations(&Span::try_new(124, 151).unwrap())
                .collect::<Vec<_>>(),
            vec![(0, 1), (1, 25), (2, 1)]
        );

        // Span is larger than binned span
        let bins = BinnedSpan::try_new(
            Span::try_new(100, 200).unwrap(),
            NonZero::try_from(4).unwrap(),
        )
        .unwrap();
        assert_eq!(
            bins.iter_indices_intersect_durations(&Span::try_new(0, 300).unwrap())
                .collect::<Vec<_>>(),
            vec![(0, 25), (1, 25), (2, 25), (3, 25)]
        );
        assert_eq!(
            bins.iter_indices_intersect_durations(&Span::try_new(99, 150).unwrap())
                .collect::<Vec<_>>(),
            vec![(0, 25), (1, 25)]
        );
        assert_eq!(
            bins.iter_indices_intersect_durations(&Span::try_new(99, 151).unwrap())
                .collect::<Vec<_>>(),
            vec![(0, 25), (1, 25), (2, 1)]
        );
        assert_eq!(
            bins.iter_indices_intersect_durations(&Span::try_new(150, 200).unwrap())
                .collect::<Vec<_>>(),
            vec![(2, 25), (3, 25)]
        );
        assert_eq!(
            bins.iter_indices_intersect_durations(&Span::try_new(150, 300).unwrap())
                .collect::<Vec<_>>(),
            vec![(2, 25), (3, 25)]
        );
    }
}
