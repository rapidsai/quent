// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Time-related types and utilities.
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(not(feature = "clock-quanta"))]
use std::time::Instant;

#[cfg(feature = "clock-quanta")]
use quanta::Clock;

use thiserror::Error;

#[cfg(feature = "__test-clock-override")]
use std::cell::Cell;

pub mod bin;
pub mod span;

#[cfg(feature = "__test-clock-override")]
thread_local!(static TIMESTAMP_OVERRIDE: Cell<Option<TimeUnixNanoSec>> = const { Cell::new(None) });

/// Arm the next [`timestamp()`] read on this thread to return `ts`.
/// The override is consumed on read.
#[cfg(feature = "__test-clock-override")]
#[doc(hidden)]
pub fn set_timestamp(ts: TimeUnixNanoSec) {
    TIMESTAMP_OVERRIDE.with(|c| c.set(Some(ts)));
}

pub use span::{SpanNanoSec, SpanSec};

/// A number of nanoseconds expired since the Unix epoch.
// TODO(johanpel): u64::MAX should be excluded as a valid timestamp because it
// cannot fall into half-open span intervals. There is a possibility to make
// this a sentinel value for "potentially up to infinity" which may be useful
// when events are missing, e.g. state machine exit state timestamps.
pub type TimeUnixNanoSec = u64;

/// An amount of nanoseconds.
pub type TimeNanoSec = u64;

/// An amount of seconds.
pub type TimeSec = f64;

/// Error type
#[derive(Clone, Debug, Error)]
pub enum TimeError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

/// Result type
pub type Result<T> = std::result::Result<T, TimeError>;

#[cfg(feature = "clock-quanta")]
static EPOCH: OnceLock<(Clock, quanta::Instant, u64)> = OnceLock::new();
#[cfg(not(feature = "clock-quanta"))]
static EPOCH: OnceLock<(Instant, u64)> = OnceLock::new();

fn epoch_unix_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is set before Unix epoch")
        .as_nanos() as u64
}

#[cfg(feature = "clock-quanta")]
fn epoch() -> &'static (Clock, quanta::Instant, u64) {
    EPOCH.get_or_init(|| {
        let clock = Clock::new();
        let instant = clock.now();
        (clock, instant, epoch_unix_ns())
    })
}

#[cfg(not(feature = "clock-quanta"))]
fn epoch() -> &'static (Instant, u64) {
    EPOCH.get_or_init(|| (Instant::now(), epoch_unix_ns()))
}

/// Initialize the clock used by [`timestamp()`].
///
/// Calling this before emitting events keeps one-time clock setup outside the
/// first event call. Subsequent calls have no effect.
#[doc(hidden)]
pub fn initialize_clock() {
    // Using epoch() here so we don't potentially consume a value from
    // TIMESTAMP_OVERRIDE.
    let _ = epoch();
}

/// Return the number of nanoseconds passed since the Unix epoch.
///
/// # Clock implementation notes
///
/// Select the clock implementation with Cargo features. Timekeeping has subtle
/// hardware and operating system limitations. If strict timing guarantees
/// matter to your use case, review the selected clock's documentation carefully:
///
/// - **Standard clock (default):** [`std::time::Instant`].
/// - **`clock-quanta`:** [Quanta's `Clock`](https://docs.rs/quanta/0.12.6/quanta/struct.Clock.html).
///
/// On first use, this function records the current Unix time and then adds
/// elapsed time to it. If the system clock is later corrected, for example by
/// NTP, returned timestamps continue from the original time and may differ
/// from the current Unix time.
///
/// ## Test override (enabled with the `__test-clock-override` feature)
///
/// When enabling this feature, it is possible to make the next call into
/// [`timestamp()`] return a value set before that with [`set_timestamp()`].
/// This feature is intended for testing purposes only.
///
/// # Panics
///
/// Clock initialization panics if the system clock is before the Unix epoch.
#[inline(always)]
pub fn timestamp() -> TimeUnixNanoSec {
    #[cfg(feature = "__test-clock-override")]
    if let Some(ts) = TIMESTAMP_OVERRIDE.with(|c| c.take()) {
        return ts;
    }
    // Conversion to u64 limits this to Unix timestamp in seconds to
    // 18446744073709551617, which is in the 26th century.
    #[cfg(feature = "clock-quanta")]
    {
        let (clock, instant, epoch_unix_ns) = epoch();
        epoch_unix_ns.saturating_add(clock.now().duration_since(*instant).as_nanos() as u64)
    }
    #[cfg(not(feature = "clock-quanta"))]
    {
        let (instant, epoch_unix_ns) = epoch();
        epoch_unix_ns.saturating_add(instant.elapsed().as_nanos() as u64)
    }
}

/// Convert a nanosecond timestamp to seconds.
pub fn to_secs(time: TimeNanoSec) -> TimeSec {
    time as f64 * 1e-9
}

/// Convert a seconds timestamp to nanoseconda.
pub fn to_nanosecs(time: TimeSec) -> TimeNanoSec {
    (time * 1e9) as u64
}

/// Convert a nanosecond timestamp to seconds, relative to some epoch.
///
/// Does not allow the timestamp to fall before the epoch.
pub fn try_to_secs_relative(timestamp: TimeNanoSec, epoch: TimeNanoSec) -> Result<TimeSec> {
    timestamp.checked_sub(epoch)
        .ok_or_else(|| {
            TimeError::InvalidArgument(format!(
                "unable to convert to seconds relative to epoch - the epoch {epoch} occurs later than {timestamp}"
            ))
        })
        .map(to_secs)
}

/// Convert a nanosecond timestamp to seconds, relative to some epoch.
///
/// Allows the timestamp to fall before the epoch, in which case a negative
/// value is returned.
pub fn to_secs_relative(timestamp: TimeNanoSec, epoch: TimeNanoSec) -> TimeSec {
    if timestamp >= epoch {
        to_secs(timestamp - epoch)
    } else {
        -to_secs(epoch - timestamp)
    }
}

pub trait Timestamp {
    fn timestamp(&self) -> TimeUnixNanoSec;
}

/// Provides the key used to order an item in an [`OrderedCollector`].
pub trait OrderKey {
    /// The item's sortable key type.
    type Key: Ord;

    /// Return the key used to order this item.
    fn order_key(&self) -> Self::Key;
}

/// Maintains an ordered sequence of items.
///
/// Optimized for when the common case is that items arrive in key order,
/// in which case [`Self::push`] is O(1). Out-of-order items are inserted via
/// binary search (O(log n) search + O(n) insertion).
///
/// Stable: items with equal keys keep their arrival order.
pub struct OrderedCollector<T>(Vec<T>);

impl<T> Default for OrderedCollector<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> OrderedCollector<T>
where
    T: OrderKey,
{
    /// Inserts an item and reports whether its key was already present.
    pub fn push(&mut self, item: T) -> bool {
        let key = item.order_key();
        match self.0.last().map(|last| last.order_key().cmp(&key)) {
            Some(std::cmp::Ordering::Less) | None => {
                self.0.push(item);
                false
            }
            Some(std::cmp::Ordering::Equal) => {
                self.0.push(item);
                true
            }
            Some(std::cmp::Ordering::Greater) => {
                // Upper-bound insertion keeps equal keys in arrival order.
                let pos = self
                    .0
                    .partition_point(|existing| existing.order_key() <= key);
                let equal_key = pos > 0 && self.0[pos - 1].order_key() == key;
                self.0.insert(pos, item);
                equal_key
            }
        }
    }

    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T> Extend<T> for OrderedCollector<T>
where
    T: OrderKey,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for transition in iter {
            self.push(transition);
        }
    }
}

#[cfg(test)]
mod collector_tests {
    use super::*;

    /// Carries its order key plus a tag, so equal-key ordering is visible.
    #[derive(Debug, PartialEq, Eq)]
    struct Tagged(u8, &'static str);

    impl OrderKey for Tagged {
        type Key = u8;

        fn order_key(&self) -> Self::Key {
            self.0
        }
    }

    fn collect(items: impl IntoIterator<Item = Tagged>) -> Vec<(u8, &'static str)> {
        let mut collector = OrderedCollector::default();
        collector.extend(items);
        collector
            .into_inner()
            .into_iter()
            .map(|Tagged(ts, tag)| (ts, tag))
            .collect()
    }

    #[test]
    fn in_order_arrivals_are_sorted() {
        assert_eq!(
            collect([Tagged(10, "a"), Tagged(20, "b"), Tagged(30, "c")]),
            [(10, "a"), (20, "b"), (30, "c")]
        );
    }

    #[test]
    fn late_arrivals_are_sorted_into_place() {
        assert_eq!(
            collect([Tagged(30, "c"), Tagged(10, "a"), Tagged(20, "b")]),
            [(10, "a"), (20, "b"), (30, "c")]
        );
    }

    #[test]
    fn equal_keys_keep_arrival_order_on_the_fast_path() {
        let mut collector = OrderedCollector::default();
        assert!(!collector.push(Tagged(10, "first")));
        assert!(collector.push(Tagged(10, "second")));
        assert_eq!(
            collector
                .into_inner()
                .into_iter()
                .map(|Tagged(key, tag)| (key, tag))
                .collect::<Vec<_>>(),
            [(10, "first"), (10, "second")]
        );
    }

    /// Regression: `<` predicate (lower bound) reversed arrival order on the slow path.
    #[test]
    fn equal_keys_keep_arrival_order_on_the_slow_path() {
        // Key 20 triggers the slow path for the subsequent second item at key 10.
        let mut collector = OrderedCollector::default();
        assert!(!collector.push(Tagged(10, "first")));
        assert!(!collector.push(Tagged(20, "later")));
        assert!(collector.push(Tagged(10, "second")));
        assert_eq!(
            collector
                .into_inner()
                .into_iter()
                .map(|Tagged(key, tag)| (key, tag))
                .collect::<Vec<_>>(),
            [(10, "first"), (10, "second"), (20, "later")]
        );
    }

    #[test]
    fn equal_keys_keep_arrival_order_across_a_run() {
        assert_eq!(
            collect([
                Tagged(10, "a"),
                Tagged(10, "b"),
                Tagged(20, "later"),
                Tagged(10, "c"),
                Tagged(10, "d"),
            ]),
            [(10, "a"), (10, "b"), (10, "c"), (10, "d"), (20, "later")]
        );
    }
}

#[cfg(all(test, feature = "__test-clock-override"))]
mod tests {
    use super::*;

    #[test]
    fn override_consumed_on_read() {
        set_timestamp(42);
        assert_eq!(timestamp(), 42);
        // Override consumed by the read — next call returns wall-clock,
        // well above any post-1970 nanosecond count we'd collide with.
        let now = timestamp();
        assert!(
            now > 1_600_000_000_000_000_000,
            "expected wall-clock ns, got {now}"
        );
        // Re-arming works.
        set_timestamp(43);
        assert_eq!(timestamp(), 43);
    }
}
