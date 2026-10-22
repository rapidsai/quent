// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed access to fully materialized model events as Rust-native types.

use quent_events::{CombinedEventModel, EntityMarker, Event};
use quent_time::{OrderKey, TimeUnixNanoSec, Timestamp};

#[cfg(any(feature = "io-ndjson", feature = "io-msgpack", feature = "io-postcard"))]
pub mod filesystem;

/// An iterator yielding owned [`Event<T>`](Event) values or read failures.
pub type EventIterator<T, E> = Box<dyn Iterator<Item = Result<Event<T>, E>>>;

/// Loads owned events for entity marker `E`.
pub trait EventLoader<E: EntityMarker> {
    /// Error returned when events cannot be loaded.
    type Error;

    /// Loads `E::Payload` payloads without an ordering guarantee.
    fn events(&self) -> Result<EventIterator<E::Payload, Self::Error>, Self::Error>;
}

/// Loads owned model-wide events with combined event payloads.
///
/// # Code generation
///
/// Generated models support this trait only when
/// `quent_store_build::EventOptions::combined_event` is enabled.
pub trait CombinedEventLoader<M: CombinedEventModel> {
    /// Error returned when events cannot be loaded.
    type Error;

    /// Loads `Event<M::CombinedEvent>` values without an ordering guarantee.
    fn combined_events(&self) -> Result<EventIterator<M::CombinedEvent, Self::Error>, Self::Error>;
}

/// Marks an entity marker as belonging to model `M`.
///
/// This prevents users from using entity markers with the wrong model.
///
/// # Code generation
///
/// For entity marker `Task`, whose event payload type is `TaskEvent`,
/// `quent-store-build` emits `impl EntityMarkerInModel<Demo> for Task {}`.
#[doc(hidden)]
pub trait EntityMarkerInModel<M>: EntityMarker {}

/// Data with a timestamp.
#[derive(Clone, Debug)]
pub struct Timestamped<P> {
    pub timestamp: TimeUnixNanoSec,
    pub data: P,
}

impl<P> Timestamped<P> {
    pub fn new(timestamp: TimeUnixNanoSec, data: P) -> Self {
        Self { timestamp, data }
    }
}

impl<P> Timestamp for Timestamped<P> {
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
}

impl<P> OrderKey for Timestamped<P> {
    type Key = TimeUnixNanoSec;

    fn order_key(&self) -> Self::Key {
        self.timestamp
    }
}
