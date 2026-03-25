// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Time-related types and utilities.
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use thiserror::Error;

pub mod bin;
pub mod span;

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

/// Return a monotonically increasing [`TimeUnixNanoSec`] timestamp.
///
/// This function guarantees that subsequent calls will never return a value
/// less than a previous call, even if the system clock is adjusted backwards
/// (e.g. by an NTP sync).
///
/// If the system's clock is somehow set to before the Unix epoch when this
/// function is first called, this function will panic. While this is pretty
/// aggressive, the system is most likely very misconfigured.
#[inline]
pub fn timestamp() -> TimeUnixNanoSec {
    static EPOCH: OnceLock<(Instant, u64)> = OnceLock::new();
    let (instant, epoch_unix_ns) = EPOCH.get_or_init(|| {
        (
            Instant::now(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is set before Unix epoch")
                .as_nanos() as u64,
        )
    });
    // Conversion to u64 limits this to Unix timestamp in seconds to
    // 18446744073709551617, which is in the 26th century.
    epoch_unix_ns.saturating_add(instant.elapsed().as_nanos() as u64)
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

/// Maintains a timestamp-ordered sequence of items.
///
/// Optimized for when the common case is that items arrive in timestamp order,
/// in which case [`Self::push`] is O(1). Out-of-order items are inserted via
/// binary search (O(log n) search + O(n) insertion).
pub struct TimeOrderedCollector<T>(Vec<T>);

impl<T> Default for TimeOrderedCollector<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> TimeOrderedCollector<T>
where
    T: Timestamp,
{
    pub fn push(&mut self, state: T) {
        if let Some(last) = self.0.last()
            && last.timestamp() <= state.timestamp()
        {
            self.0.push(state);
        } else {
            let pos = self
                .0
                .partition_point(|s| s.timestamp() < state.timestamp());
            self.0.insert(pos, state);
        }
    }

    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T> Extend<T> for TimeOrderedCollector<T>
where
    T: Timestamp,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for transition in iter {
            self.push(transition)
        }
    }
}
