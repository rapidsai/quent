//! Time-related types and utilities.

use py_rs::PY;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

/// The number of nanoseconds expired since the Unix epoch
pub type Timestamp = u64;

/// An amount of time in nanoseconds.
pub type Duration = u64;

/// Error type
#[derive(Error, Debug)]
pub enum TimeError {
    #[error("invalid arguments")]
    InvalidArguments,
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

/// A span of time
#[derive(TS, PY, Clone, Default, Debug, Deserialize, PartialEq, Serialize)]
pub struct Span {
    start: Timestamp,
    end: Timestamp,
}

impl Span {
    /// Construct a new Span. Checks whether end >= start.
    #[inline]
    pub fn try_new(start: Timestamp, end: Timestamp) -> Result<Self> {
        if end < start {
            Err(TimeError::InvalidArguments)
        } else {
            Ok(Self { start, end })
        }
    }

    /// Return the start Timestamp of this span.
    #[inline]
    pub fn start(&self) -> Timestamp {
        self.start
    }

    /// Return the end Timestamp of this span.
    #[inline]
    pub fn end(&self) -> Timestamp {
        self.end
    }

    /// Return true if the other Span intersects with this Span, false otherwise.
    #[inline]
    pub fn intersects(&self, other: &Span) -> bool {
        self.start < other.end && self.end > other.start
    }

    /// Return the intersection of this Span with the other Span as a Span, if the intersection exists.
    pub fn intersection(&self, other: &Span) -> Option<Span> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);

        if start >= end {
            None
        } else {
            Some(Span { start, end })
        }
    }

    /// Return true if this Span occurs within the duration of the other Span.
    #[inline]
    pub fn during(&self, other: &Span) -> bool {
        self.start >= other.start && self.end <= other.end
    }

    /// Return true if the other Span occurs within the duration of this Span.
    #[inline]
    pub fn contains(&self, other: &Span) -> bool {
        other.start >= self.start && other.end <= self.end
    }

    /// Return the duration of this Span.
    pub fn duration(&self) -> Duration {
        self.end - self.start
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

        assert_span_ok(0, 0);
        assert_span_ok(0, u64::MAX);
        assert_span_ok(u64::MAX, u64::MAX);
        assert_span_ok(u64::MAX - 1, u64::MAX);
        assert_span_ok(100, 200);
        assert_span_ok(timestamp(), timestamp());

        assert!(matches!(
            Span::try_new(1, 0).err().unwrap(),
            TimeError::InvalidArguments
        ));
        assert!(matches!(
            Span::try_new(u64::MAX, u64::MAX - 1).err().unwrap(),
            TimeError::InvalidArguments
        ));
        assert!(matches!(
            Span::try_new(u64::MAX, 0).err().unwrap(),
            TimeError::InvalidArguments
        ));
    }

    #[test]
    fn span_intersects() {
        // Partial
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(150, 250).unwrap();
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));

        // Contained
        let span1 = Span::try_new(100, 300).unwrap();
        let span2 = Span::try_new(150, 200).unwrap();
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));

        // No overlap
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(200, 300).unwrap();
        assert!(!span1.intersects(&span2));
        assert!(!span2.intersects(&span1));

        // Adjecent
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(201, 300).unwrap();
        assert!(!span1.intersects(&span2));
        assert!(!span2.intersects(&span1));

        // Touching boundaries
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(200, 300).unwrap();
        assert!(!span1.intersects(&span2));
        assert!(!span2.intersects(&span1));

        //  Overlaps by one
        let span1 = Span::try_new(100, 201).unwrap();
        let span2 = Span::try_new(200, 300).unwrap();
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));
    }

    #[test]
    fn span_during() {
        // Fully contained
        let span1 = Span::try_new(150, 200).unwrap();
        let span2 = Span::try_new(100, 300).unwrap();
        assert!(span1.during(&span2));
        assert!(!span2.during(&span1));

        // Equal spans
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(100, 200).unwrap();
        assert!(span1.during(&span2));
        assert!(span2.during(&span1));

        // Partial overlap
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(150, 250).unwrap();
        assert!(!span1.during(&span2));
        assert!(!span2.during(&span1));

        // No overlap
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
        // Fully containing
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
        let span = Span::try_new(100, 300).unwrap();
        assert_eq!(span.duration(), 200);

        // zero
        let span = Span::try_new(100, 100).unwrap();
        assert_eq!(span.duration(), 0);

        // huge duration
        let span = Span::try_new(0, u64::MAX).unwrap();
        assert_eq!(span.duration(), u64::MAX);
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

        // Touching boundaries
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(200, 300).unwrap();
        assert!(span1.intersection(&span2).is_none());
        assert!(span2.intersection(&span1).is_none());

        // One contained
        let span1 = Span::try_new(100, 300).unwrap();
        let span2 = Span::try_new(150, 200).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        assert_eq!(intersection.unwrap(), span2);

        // Exact match
        let span1 = Span::try_new(100, 200).unwrap();
        let span2 = Span::try_new(100, 200).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        assert_eq!(intersection.unwrap(), span1);

        // One point overlap
        let span1 = Span::try_new(100, 201).unwrap();
        let span2 = Span::try_new(200, 300).unwrap();
        let intersection = span1.intersection(&span2);
        assert!(intersection.is_some());
        let intersection = intersection.unwrap();
        assert_eq!(intersection.start(), 200);
        assert_eq!(intersection.end(), 201);
        assert_eq!(intersection.duration(), 1);
    }
}
