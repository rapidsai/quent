// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::{Result, TimeError, TimeNanoSec, TimeSec, TimeUnixNanoSec, try_to_secs_relative};

/// A span of time represented as a half-open interval [start, end) over
/// a discrete number of nanoseconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpanNanoSec {
    /// The start time, inclusive.
    start: TimeNanoSec,
    /// The end time, exclusive.
    end: TimeNanoSec,
}

/// Type alias for a slight increase in code clarity wherever it's used.
pub type SpanUnixNanoSec = SpanNanoSec;

impl SpanNanoSec {
    /// Construct a new Span.
    ///
    /// `start` must precede `end`, otherwise this function will return an
    /// error.
    ///
    /// If the start and end time is equal, the duration of this span is zero.
    pub fn try_new(start: TimeNanoSec, end: TimeNanoSec) -> Result<Self> {
        if end < start {
            Err(TimeError::InvalidArgument(format!(
                "Span cannot be constructed with end ({end}) preceding start ({start})"
            )))
        } else {
            Ok(Self { start, end })
        }
    }

    /// Return a span with the maximum expressable range.
    pub fn new_max() -> Self {
        Self {
            start: 0,
            end: TimeNanoSec::MAX,
        }
    }

    /// Return the start Timestamp of this Span.
    #[inline]
    pub fn start(&self) -> TimeNanoSec {
        self.start
    }

    /// Return the end Timestamp of this Span.
    #[inline]
    pub fn end(&self) -> TimeNanoSec {
        self.end
    }

    /// Return true if the other Span intersects with this Span, false
    /// otherwise.
    ///
    /// Spans of which the end and start values are equal are considered not to
    /// intersect, because intervals are half-open.
    #[inline]
    pub fn intersects(&self, other: &SpanNanoSec) -> bool {
        self.start < other.end && self.end > other.start
    }

    /// Return the Span where this span intersects with the other Span, if the
    /// intersection exists.
    pub fn intersection(&self, other: &SpanNanoSec) -> Option<SpanNanoSec> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);

        if start >= end {
            None
        } else {
            Some(SpanNanoSec { start, end })
        }
    }

    /// Return true if this Span is completely contained withi the other Span.
    #[inline]
    pub fn during(&self, other: &SpanNanoSec) -> bool {
        self.start >= other.start && self.end <= other.end
    }

    /// Return true if the other Span is completely contained within this Span.
    #[inline]
    pub fn contains(&self, other: &SpanNanoSec) -> bool {
        other.start >= self.start && other.end <= self.end
    }

    /// Return the duration of this Span.
    ///
    /// The duration is equal to the count of discrete timestamps within the
    /// half-open interval.
    #[inline]
    pub fn duration(&self) -> TimeNanoSec {
        self.end.saturating_sub(self.start)
    }

    /// Return true if the timestamp lies within this span, false otherwise.
    ///
    /// Since a Span interval is half-open, a timestamp equal to the end timestamp
    /// is considered to not lie within this span.
    #[inline]
    pub fn contains_timestamp(&self, timestamp: TimeUnixNanoSec) -> bool {
        self.start <= timestamp && timestamp < self.end
    }

    /// Convert self to a [`SpanSec`], relative to the provided epoch.
    pub fn try_to_secs_relative(&self, epoch: TimeNanoSec) -> Result<SpanSec> {
        Ok(SpanSec {
            start: try_to_secs_relative(self.start, epoch)?,
            end: try_to_secs_relative(self.end, epoch)?,
        })
    }

    /// Extend this span such that it includes all timestamps of both self and
    /// other.
    #[inline]
    pub fn extend(&self, other: &Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// A span of time in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS, serde::Serialize))]
pub struct SpanSec {
    /// The start time, inclusive.
    start: TimeSec,
    /// The end time, exclusive.
    end: TimeSec,
}

impl SpanSec {
    /// Construct a new SpanSec.
    ///
    /// `start` must precede `end`, otherwise this function will return an
    /// error.
    ///
    /// If the start and end time is equal, the duration of this span is zero.
    pub fn try_new(start: TimeSec, end: TimeSec) -> Result<SpanSec> {
        if end < start {
            Err(TimeError::InvalidArgument(format!(
                "Span cannot be constructed with end ({end}) preceding start ({start})"
            )))
        } else {
            Ok(Self { start, end })
        }
    }

    /// Return the start time of this Span.
    #[inline]
    pub fn start(&self) -> TimeSec {
        self.start
    }

    /// Return the end time of this Span.
    #[inline]
    pub fn end(&self) -> TimeSec {
        self.end
    }

    /// Return the duration of this Span.
    #[inline]
    pub fn duration(&self) -> TimeSec {
        self.end - self.start
    }
}

#[cfg(test)]
mod tests {
    use crate::timestamp;

    use super::*;

    #[test]
    fn span_try_new() {
        let assert_span_ok = |start: u64, end: u64| {
            let span = SpanNanoSec::try_new(start, end);
            assert!(span.is_ok());
            let span = span.unwrap();
            assert_eq!(span.start(), start);
            assert_eq!(span.end(), end);
        };

        // Zero-duration span.
        assert_span_ok(0, 0);
        // Maximum size span.
        assert_span_ok(0, u64::MAX);
        // Empty span at max timestamp.
        assert_span_ok(u64::MAX, u64::MAX);
        // Span of size 1 touching max timestamp.
        assert_span_ok(u64::MAX - 1, u64::MAX);
        // Span of timestamps 100 up to and including 199, but not 200.
        assert_span_ok(100, 200);
        // Sneaky test that would error out if subsequent timestamp() calls ever
        // return a timestamp that is not monotonically increasing.
        assert_span_ok(timestamp(), timestamp());

        // Can't construct a reverse time span
        assert!(matches!(
            SpanNanoSec::try_new(10, 9).err().unwrap(),
            TimeError::InvalidArgument(_)
        ));
        // Can't construct a reverse time span near the lowest timestamp range.
        assert!(matches!(
            SpanNanoSec::try_new(1, 0).err().unwrap(),
            TimeError::InvalidArgument(_)
        ));
        // Can't construct a reverse time span near the highest timestamp range.
        assert!(matches!(
            SpanNanoSec::try_new(u64::MAX, u64::MAX - 1).err().unwrap(),
            TimeError::InvalidArgument(_)
        ));
        // Can't construct a reverse time span at timestamp extremeties.
        assert!(matches!(
            SpanNanoSec::try_new(u64::MAX, 0).err().unwrap(),
            TimeError::InvalidArgument(_)
        ));
    }

    #[test]
    fn span_intersects() {
        // Partial intersection.
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(150, 250).unwrap();
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));

        // Fully contained is also intersection.
        let span1 = SpanNanoSec::try_new(100, 300).unwrap();
        let span2 = SpanNanoSec::try_new(150, 200).unwrap();
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));

        // Adjacent spans don't intersect since we're using half open intervals.
        let span1 = SpanNanoSec::try_new(100, 200).unwrap(); // [100, 200)
        let span2 = SpanNanoSec::try_new(200, 300).unwrap(); // [200, 300)
        assert!(!span1.intersects(&span2));
        assert!(!span2.intersects(&span1));

        // A one nanosecond gap doesn't intersect.
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(201, 300).unwrap();
        assert!(!span1.intersects(&span2));
        assert!(!span2.intersects(&span1));

        // One discrete time step overlaps, intersects.
        let span1 = SpanNanoSec::try_new(100, 201).unwrap(); // includes 200
        let span2 = SpanNanoSec::try_new(200, 300).unwrap(); // includes 200
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));
    }

    #[test]
    fn span_during() {
        // Span 1 is fully contained within span 2, so span 1 occurs during span 2.
        let span1 = SpanNanoSec::try_new(150, 200).unwrap();
        let span2 = SpanNanoSec::try_new(100, 300).unwrap();
        assert!(span1.during(&span2));
        assert!(!span2.during(&span1));

        // Equal spans contain each other
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(100, 200).unwrap();
        assert!(span1.during(&span2));
        assert!(span2.during(&span1));

        // Partial overlap is not fully contained
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(150, 250).unwrap();
        assert!(!span1.during(&span2));
        assert!(!span2.during(&span1));

        // No overlap is not contained at all
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(300, 400).unwrap();
        assert!(!span1.during(&span2));
        assert!(!span2.during(&span1));

        // Same start
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(100, 300).unwrap();
        assert!(span1.during(&span2));
        assert!(!span2.during(&span1));

        // Same end
        let span1 = SpanNanoSec::try_new(150, 300).unwrap();
        let span2 = SpanNanoSec::try_new(100, 300).unwrap();
        assert!(span1.during(&span2));
        assert!(!span2.during(&span1));
    }

    #[test]
    fn span_contains() {
        // Span 2 is fully contained in span 1, but not vice versa
        let span1 = SpanNanoSec::try_new(100, 300).unwrap();
        let span2 = SpanNanoSec::try_new(150, 200).unwrap();
        assert!(span1.contains(&span2));
        assert!(!span2.contains(&span1));

        // Same span
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(100, 200).unwrap();
        assert!(span1.contains(&span2));
        assert!(span2.contains(&span1));

        // Partial overlap
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(150, 250).unwrap();
        assert!(!span1.contains(&span2));
        assert!(!span2.contains(&span1));

        // No overlap
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(300, 400).unwrap();
        assert!(!span1.contains(&span2));
        assert!(!span2.contains(&span1));
    }

    #[test]
    fn test_duration() {
        // Duration is basically a count of discrete time steps: end - start
        let span = SpanNanoSec::try_new(100, 300).unwrap();
        assert_eq!(span.duration(), 200); // 100 .. 299 = 200 time steps

        // Empty span
        let span = SpanNanoSec::try_new(100, 100).unwrap();
        assert_eq!(span.duration(), 0);

        // Single time step
        let span = SpanNanoSec::try_new(100, 101).unwrap();
        assert_eq!(span.duration(), 1);

        // Full range
        let span = SpanNanoSec::try_new(0, u64::MAX).unwrap();
        assert_eq!(span.duration(), u64::MAX);

        // Arbitrary span
        let span = SpanNanoSec::try_new(100, 1000).unwrap();
        assert_eq!(span.duration(), 900);
    }

    #[test]
    fn span_intersection() {
        // Partial overlap
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(150, 250).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        let intersection = intersection.unwrap();
        assert_eq!(intersection.start(), 150);
        assert_eq!(intersection.end(), 200);
        assert_eq!(span2.intersection(&span1).unwrap(), intersection);

        // No overlap
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(300, 400).unwrap();
        assert!(span1.intersection(&span2).is_none());
        assert!(span2.intersection(&span1).is_none());

        // Adjacent spans don't intersect with the end being open
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(200, 300).unwrap();
        assert!(span1.intersection(&span2).is_none());
        assert!(span2.intersection(&span1).is_none());

        // Fully contained gives the same span
        let span1 = SpanNanoSec::try_new(100, 300).unwrap();
        let span2 = SpanNanoSec::try_new(150, 200).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        assert_eq!(intersection.unwrap(), span2);

        // Equal spans give the same span
        let span1 = SpanNanoSec::try_new(100, 200).unwrap();
        let span2 = SpanNanoSec::try_new(100, 200).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        assert_eq!(intersection.unwrap(), span1);

        // One time step overlap
        let span1 = SpanNanoSec::try_new(100, 201).unwrap();
        let span2 = SpanNanoSec::try_new(200, 300).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        let intersection = intersection.unwrap();
        assert_eq!(intersection.start(), 200);
        assert_eq!(intersection.end(), 201);
        assert_eq!(intersection.duration(), 1);
    }

    #[test]
    fn span_contains_timestamp() {
        let span = SpanNanoSec::try_new(100, 200).unwrap();

        // Before
        assert!(!span.contains_timestamp(99));
        // At start time step
        assert!(span.contains_timestamp(100));
        // Middle
        assert!(span.contains_timestamp(150));
        // End is exclusive, so doesn't contain this time step
        assert!(!span.contains_timestamp(200));
        // After
        assert!(!span.contains_timestamp(201));

        // Zero duration span contains nothing
        let span = SpanNanoSec::try_new(100, 100).unwrap();
        assert!(!span.contains_timestamp(100));
        assert!(!span.contains_timestamp(99));
        assert!(!span.contains_timestamp(101));

        // Single time step span
        let span = SpanNanoSec::try_new(100, 101).unwrap();
        assert!(span.contains_timestamp(100));
        assert!(!span.contains_timestamp(101));
    }
}
