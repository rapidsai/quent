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
pub fn to_secs(timestamp: TimeNanoSec) -> f64 {
    timestamp as f64 * 1e-9
}

/// Convert a nanosecond timestamp to seconds, relative to some epoch.
pub fn try_to_secs_relative(timestamp: TimeNanoSec, epoch: TimeNanoSec) -> Result<f64> {
    timestamp.checked_sub(epoch)
        .ok_or_else(|| {
            TimeError::InvalidArgument(format!(
                "unable to convert to seconds relative to epoch - the epoch {epoch} occurs later than {timestamp}"
            ))
        })
        .map(to_secs)
}
