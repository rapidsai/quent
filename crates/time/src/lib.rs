//! Time-related types and utilities.

use py_rs::PY;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

pub mod bin;

/// The number of nanoseconds expired since the Unix epoch.
// TODO(johanpel): u64::MAX should be excluded as a valid timestamp because it
// cannot fall into half-open span intervals. There is a possibility to make
// this a sentinel value for "potentially up to infinity" which may be useful
// when events are missing, e.g. state machine exit state timestamps.
pub type Timestamp = u64;

/// An amount of time in nanoseconds.
pub type Duration = u64;

/// Error type
#[derive(Error, Debug)]
pub enum TimeError {
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
}

/// Result type
pub type Result<T> = std::result::Result<T, TimeError>;

/// Return the current Timestamp.
#[inline]
pub fn timestamp() -> Timestamp {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Narrowing conversion to u64 limits this to Unix timestamp in seconds: 18446744073709551617
    // Which is in the 26th century
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_nanos() as u64)
        .unwrap_or_default()
    // TODO(johanpel): consider to do something else instead of unwrap_or_default, perhaps using Instant as described in the duration_since docs.
}

/// A span of time represented as a half-open interval [start, end) over discrete timestamps.
#[derive(TS, PY, Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Span {
    /// The start timestamp, inclusive.
    start: Timestamp,
    /// The end timestamp, exclusive.
    end: Timestamp,
}

impl Span {
    /// Construct a new Span.
    ///
    /// The start timestamp must precede the end timestamp, otherwise this
    /// function will return an error.
    ///
    /// If the start and end timestamps are equal, the Duration of this span is
    /// zero.
    pub fn try_new(start: Timestamp, end: Timestamp) -> Result<Self> {
        if end < start {
            Err(TimeError::InvalidArguments(format!(
                "Span cannot be constructed with end ({end}) preceding start ({start})"
            )))
        } else {
            Ok(Self { start, end })
        }
    }

    /// Return the start Timestamp of this Span.
    #[inline]
    pub fn start(&self) -> Timestamp {
        self.start
    }

    /// Return the end Timestamp of this Span.
    #[inline]
    pub fn end(&self) -> Timestamp {
        self.end
    }

    /// Return true if the other Span intersects with this Span, false
    /// otherwise.
    ///
    /// Spans of which the end and start values are equal are considered not to
    /// intersect, because intervals are half-open.
    #[inline]
    pub fn intersects(&self, other: &Span) -> bool {
        self.start < other.end && self.end > other.start
    }

    /// Return the Span where this span intersects with the other Span, if the
    /// intersection exists.
    pub fn intersection(&self, other: &Span) -> Option<Span> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);

        if start >= end {
            None
        } else {
            Some(Span { start, end })
        }
    }

    /// Return true if this Span is completely contained withi the other Span.
    #[inline]
    pub fn during(&self, other: &Span) -> bool {
        self.start >= other.start && self.end <= other.end
    }

    /// Return true if the other Span is completely contained within this Span.
    #[inline]
    pub fn contains(&self, other: &Span) -> bool {
        other.start >= self.start && other.end <= self.end
    }

    /// Return the duration of this Span.
    ///
    /// The duration is equal to the count of discrete timestamps within the
    /// half-open interval.
    pub fn duration(&self) -> Duration {
        self.end.saturating_sub(self.start)
    }

    /// Return true if the timestamp lies within this span, false otherwise.
    ///
    /// Since a Span interval is half-open, a timestamp equal to the end timestamp
    /// is considered to not lie within this span.
    pub fn contains_timestamp(&self, timestamp: Timestamp) -> bool {
        self.start <= timestamp && timestamp < self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_try_new() {
        let assert_span_ok = |start: u64, end: u64| {
            let span = Span::try_new(start, end);
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
            Span::try_new(10, 9).err().unwrap(),
            TimeError::InvalidArguments(_)
        ));
        // Can't construct a reverse time span near the lowest timestamp range.
        assert!(matches!(
            Span::try_new(1, 0).err().unwrap(),
            TimeError::InvalidArguments(_)
        ));
        // Can't construct a reverse time span near the highest timestamp range.
        assert!(matches!(
            Span::try_new(u64::MAX, u64::MAX - 1).err().unwrap(),
            TimeError::InvalidArguments(_)
        ));
        // Can't construct a reverse time span at timestamp extremeties.
        assert!(matches!(
            Span::try_new(u64::MAX, 0).err().unwrap(),
            TimeError::InvalidArguments(_)
        ));
    }

    #[test]
    fn span_intersects() {
        // Partial intersection.
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(150, 250).unwrap();
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));

        // Fully contained is also intersection.
        let span1 = Span::try_new(100, 300).unwrap();
        let span2 = Span::try_new(150, 200).unwrap();
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));

        // Adjacent spans don't intersect since we're using half open intervals.
        let span1 = Span::try_new(100, 200).unwrap(); // [100, 200)
        let span2 = Span::try_new(200, 300).unwrap(); // [200, 300)
        assert!(!span1.intersects(&span2));
        assert!(!span2.intersects(&span1));

        // A one nanosecond gap doesn't intersect.
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(201, 300).unwrap();
        assert!(!span1.intersects(&span2));
        assert!(!span2.intersects(&span1));

        // One discrete time step overlaps, intersects.
        let span1 = Span::try_new(100, 201).unwrap(); // includes 200
        let span2 = Span::try_new(200, 300).unwrap(); // includes 200
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));
    }

    #[test]
    fn span_during() {
        // Span 1 is fully contained within span 2, so span 1 occurs during span 2.
        let span1 = Span::try_new(150, 200).unwrap();
        let span2 = Span::try_new(100, 300).unwrap();
        assert!(span1.during(&span2));
        assert!(!span2.during(&span1));

        // Equal spans contain each other
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(100, 200).unwrap();
        assert!(span1.during(&span2));
        assert!(span2.during(&span1));

        // Partial overlap is not fully contained
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(150, 250).unwrap();
        assert!(!span1.during(&span2));
        assert!(!span2.during(&span1));

        // No overlap is not contained at all
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(300, 400).unwrap();
        assert!(!span1.during(&span2));
        assert!(!span2.during(&span1));

        // Same start
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(100, 300).unwrap();
        assert!(span1.during(&span2));
        assert!(!span2.during(&span1));

        // Same end
        let span1 = Span::try_new(150, 300).unwrap();
        let span2 = Span::try_new(100, 300).unwrap();
        assert!(span1.during(&span2));
        assert!(!span2.during(&span1));
    }

    #[test]
    fn span_contains() {
        // Span 2 is fully contained in span 1, but not vice versa
        let span1 = Span::try_new(100, 300).unwrap();
        let span2 = Span::try_new(150, 200).unwrap();
        assert!(span1.contains(&span2));
        assert!(!span2.contains(&span1));

        // Same span
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(100, 200).unwrap();
        assert!(span1.contains(&span2));
        assert!(span2.contains(&span1));

        // Partial overlap
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(150, 250).unwrap();
        assert!(!span1.contains(&span2));
        assert!(!span2.contains(&span1));

        // No overlap
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(300, 400).unwrap();
        assert!(!span1.contains(&span2));
        assert!(!span2.contains(&span1));
    }

    #[test]
    fn test_duration() {
        // Duration is basically a count of discrete time steps: end - start
        let span = Span::try_new(100, 300).unwrap();
        assert_eq!(span.duration(), 200); // 100 .. 299 = 200 time steps

        // Empty span
        let span = Span::try_new(100, 100).unwrap();
        assert_eq!(span.duration(), 0);

        // Single time step
        let span = Span::try_new(100, 101).unwrap();
        assert_eq!(span.duration(), 1);

        // Full range
        let span = Span::try_new(0, u64::MAX).unwrap();
        assert_eq!(span.duration(), u64::MAX);

        // Arbitrary span
        let span = Span::try_new(100, 1000).unwrap();
        assert_eq!(span.duration(), 900);
    }

    #[test]
    fn span_intersection() {
        // Partial overlap
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(150, 250).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        let intersection = intersection.unwrap();
        assert_eq!(intersection.start(), 150);
        assert_eq!(intersection.end(), 200);
        assert_eq!(span2.intersection(&span1).unwrap(), intersection);

        // No overlap
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(300, 400).unwrap();
        assert!(span1.intersection(&span2).is_none());
        assert!(span2.intersection(&span1).is_none());

        // Adjacent spans don't intersect with the end being open
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(200, 300).unwrap();
        assert!(span1.intersection(&span2).is_none());
        assert!(span2.intersection(&span1).is_none());

        // Fully contained gives the same span
        let span1 = Span::try_new(100, 300).unwrap();
        let span2 = Span::try_new(150, 200).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        assert_eq!(intersection.unwrap(), span2);

        // Equal spans give the same span
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(100, 200).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        assert_eq!(intersection.unwrap(), span1);

        // One time step overlap
        let span1 = Span::try_new(100, 201).unwrap();
        let span2 = Span::try_new(200, 300).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        let intersection = intersection.unwrap();
        assert_eq!(intersection.start(), 200);
        assert_eq!(intersection.end(), 201);
        assert_eq!(intersection.duration(), 1);
    }

    #[test]
    fn span_contains_timestamp() {
        let span = Span::try_new(100, 200).unwrap();

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
        let span = Span::try_new(100, 100).unwrap();
        assert!(!span.contains_timestamp(100));
        assert!(!span.contains_timestamp(99));
        assert!(!span.contains_timestamp(101));

        // Single time step span
        let span = Span::try_new(100, 101).unwrap();
        assert!(span.contains_timestamp(100));
        assert!(!span.contains_timestamp(101));
    }
}
