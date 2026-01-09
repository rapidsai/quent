use py_rs::PY;
use quent_events::{Duration, Timestamp};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A span of time
#[derive(TS, PY, Clone, Default, Debug, Deserialize, PartialEq, Serialize)]
pub struct Span {
    pub start: Timestamp,
    pub end: Timestamp,
}

impl Span {
    /// Return true if the other Span overlaps this Span, false otherwise.
    #[inline]
    pub fn overlaps(&self, other: &Span) -> bool {
        self.start < other.end && self.end > other.start
    }

    /// Return true if this span is fully contained within the other Span, false
    /// otherwise.
    #[inline]
    pub fn is_within(&self, other: &Span) -> bool {
        self.start >= other.start && self.end <= other.end
    }

    /// Return true if this Span fully contains the other Span, false otherwise.
    #[inline]
    pub fn contains(&self, other: &Span) -> bool {
        other.start >= self.start && other.end <= self.end
    }

    /// Return the duration of this Span.
    pub fn duration(&self) -> Duration {
        self.end - self.start
    }
}

// TODO(johanpel): code below is LLM generated, review it

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlaps_completely_separate() {
        let span1 = Span { start: 0, end: 10 };
        let span2 = Span { start: 20, end: 30 };
        assert!(!span1.overlaps(&span2));
        assert!(!span2.overlaps(&span1));
    }

    #[test]
    fn test_overlaps_touching_at_boundary() {
        let span1 = Span { start: 0, end: 10 };
        let span2 = Span { start: 10, end: 20 };
        // Spans that touch at a single point should not overlap
        assert!(!span1.overlaps(&span2));
        assert!(!span2.overlaps(&span1));
    }

    #[test]
    fn test_overlaps_partial_overlap() {
        let span1 = Span { start: 0, end: 15 };
        let span2 = Span { start: 10, end: 25 };
        assert!(span1.overlaps(&span2));
        assert!(span2.overlaps(&span1));
    }

    #[test]
    fn test_overlaps_one_contains_other() {
        let span1 = Span { start: 0, end: 100 };
        let span2 = Span { start: 25, end: 75 };
        assert!(span1.overlaps(&span2));
        assert!(span2.overlaps(&span1));
    }

    #[test]
    fn test_overlaps_identical_spans() {
        let span1 = Span { start: 10, end: 20 };
        let span2 = Span { start: 10, end: 20 };
        assert!(span1.overlaps(&span2));
        assert!(span2.overlaps(&span1));
    }

    #[test]
    fn test_is_within_completely_inside() {
        let inner = Span { start: 25, end: 75 };
        let outer = Span { start: 0, end: 100 };
        assert!(inner.is_within(&outer));
        assert!(!outer.is_within(&inner));
    }

    #[test]
    fn test_is_within_identical_spans() {
        let span1 = Span { start: 10, end: 20 };
        let span2 = Span { start: 10, end: 20 };
        // A span with the same boundaries should be considered within
        assert!(span1.is_within(&span2));
        assert!(span2.is_within(&span1));
    }

    #[test]
    fn test_is_within_start_aligned() {
        let span1 = Span { start: 10, end: 15 };
        let span2 = Span { start: 10, end: 20 };
        assert!(span1.is_within(&span2));
        assert!(!span2.is_within(&span1));
    }

    #[test]
    fn test_is_within_end_aligned() {
        let span1 = Span { start: 15, end: 20 };
        let span2 = Span { start: 10, end: 20 };
        assert!(span1.is_within(&span2));
        assert!(!span2.is_within(&span1));
    }

    #[test]
    fn test_is_within_partially_overlapping() {
        let span1 = Span { start: 0, end: 15 };
        let span2 = Span { start: 10, end: 25 };
        assert!(!span1.is_within(&span2));
        assert!(!span2.is_within(&span1));
    }

    #[test]
    fn test_is_within_completely_outside() {
        let span1 = Span { start: 0, end: 10 };
        let span2 = Span { start: 20, end: 30 };
        assert!(!span1.is_within(&span2));
        assert!(!span2.is_within(&span1));
    }

    #[test]
    fn test_contains_completely_inside() {
        let outer = Span { start: 0, end: 100 };
        let inner = Span { start: 25, end: 75 };
        assert!(outer.contains(&inner));
        assert!(!inner.contains(&outer));
    }

    #[test]
    fn test_contains_identical_spans() {
        let span1 = Span { start: 10, end: 20 };
        let span2 = Span { start: 10, end: 20 };
        // A span with the same boundaries should be considered contained
        assert!(span1.contains(&span2));
        assert!(span2.contains(&span1));
    }

    #[test]
    fn test_contains_start_aligned() {
        let span1 = Span { start: 10, end: 20 };
        let span2 = Span { start: 10, end: 15 };
        assert!(span1.contains(&span2));
        assert!(!span2.contains(&span1));
    }

    #[test]
    fn test_contains_end_aligned() {
        let span1 = Span { start: 10, end: 20 };
        let span2 = Span { start: 15, end: 20 };
        assert!(span1.contains(&span2));
        assert!(!span2.contains(&span1));
    }

    #[test]
    fn test_contains_partially_overlapping() {
        let span1 = Span { start: 0, end: 15 };
        let span2 = Span { start: 10, end: 25 };
        assert!(!span1.contains(&span2));
        assert!(!span2.contains(&span1));
    }

    #[test]
    fn test_contains_completely_outside() {
        let span1 = Span { start: 0, end: 10 };
        let span2 = Span { start: 20, end: 30 };
        assert!(!span1.contains(&span2));
        assert!(!span2.contains(&span1));
    }

    #[test]
    fn test_zero_length_spans() {
        let zero_span = Span { start: 10, end: 10 };
        let normal_span = Span { start: 5, end: 15 };

        // Zero-length span at a point overlaps with spans that contain that point
        assert!(zero_span.overlaps(&normal_span));
        assert!(normal_span.overlaps(&zero_span));

        // Zero-length span does not overlap with itself (start < end is false)
        assert!(!zero_span.overlaps(&zero_span));

        // Zero-length span should be within a span that covers its point
        assert!(zero_span.is_within(&normal_span));

        // A normal span should contain a zero-length span at its point
        assert!(normal_span.contains(&zero_span));
    }

    #[test]
    fn test_edge_case_large_timestamps() {
        let span1 = Span {
            start: u64::MAX - 100,
            end: u64::MAX,
        };
        let span2 = Span {
            start: u64::MAX - 50,
            end: u64::MAX - 25,
        };

        assert!(span1.overlaps(&span2));
        assert!(span1.contains(&span2));
        assert!(span2.is_within(&span1));
    }
}
