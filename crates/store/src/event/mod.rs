// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed access to fully materialized model events as Rust-native types.

use quent_events::{CombinedEventModel, EntityMarker, Event};

#[cfg(any(feature = "io-ndjson", feature = "io-msgpack", feature = "io-postcard"))]
pub mod filesystem;

/// An iterator yielding owned [`Event<T>`](Event) values or read failures.
pub type EventIterator<T, E> = Box<dyn Iterator<Item = Result<Event<T>, E>>>;

/// The result of creating an [`EventIterator`].
pub type EventIteratorResult<T, E> = Result<EventIterator<T, E>, E>;

/// Loads owned events for entity marker `E` from selected contexts.
pub trait EventLoader<E: EntityMarker> {
    /// Error returned when events cannot be loaded.
    type Error;

    /// Loads `E::Payload` payloads without an ordering guarantee.
    fn events(&self) -> EventIteratorResult<E::Payload, Self::Error>;
}

/// Loads owned model-wide events with combined event payloads.
///
/// # Code generation
///
/// Generated models support this trait only when
/// `quent_store_build::Options::combined_event` is enabled.
pub trait CombinedEventLoader<M: CombinedEventModel> {
    /// Error returned when events cannot be loaded.
    type Error;

    /// Loads `Event<M::CombinedEvent>` values without an ordering guarantee.
    fn combined_events(&self) -> EventIteratorResult<M::CombinedEvent, Self::Error>;
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
