// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed access to fully materialized model events.

use quent_events::{Entity, Event, ModelEvents};
use uuid::Uuid;

#[cfg(any(feature = "io-ndjson", feature = "io-msgpack", feature = "io-postcard"))]
pub mod filesystem;

/// An iterator yielding owned [`Event<T>`](Event) values or read failures.
pub type EventIterator<T, E> = Box<dyn Iterator<Item = Result<Event<T>, E>>>;

/// The result of creating an [`EventIterator`].
pub type EventIteratorResult<T, E> = Result<EventIterator<T, E>, E>;

/// Loads stored events as owned values with payloads typed for an entity in model `M`.
pub trait EntityEventStore<M> {
    /// Error returned when events cannot be loaded.
    type Error;

    /// Loads events for entity type `E` without an ordering guarantee.
    fn entity_events<E>(
        &self,
        context_id: Uuid,
    ) -> EventIteratorResult<E::Event, <Self as EntityEventStore<M>>::Error>
    where
        E: StoredEntity<M>,
        Self: EntityEventLoader<E, Error = <Self as EntityEventStore<M>>::Error>,
    {
        self.load_entity_events(context_id)
    }
}

/// Loads model-wide stored events as owned values with umbrella-event payloads.
///
/// Generated models support this trait only when
/// `quent_store_build::Options::umbrella_event` is enabled.
pub trait ModelEventStore<M: ModelEvents>: EntityEventStore<M> {
    /// Loads every event stored for `context_id` without an ordering guarantee.
    fn events(
        &self,
        context_id: Uuid,
    ) -> EventIteratorResult<M::UmbrellaEvent, <Self as EntityEventStore<M>>::Error>
    where
        Self: ModelEventLoader<M, Error = <Self as EntityEventStore<M>>::Error>,
    {
        self.load_model_events(context_id)
    }
}

/// Loads one concrete entity event type for an [`EntityEventStore`].
#[doc(hidden)]
pub trait EntityEventLoader<E: Entity> {
    /// Error returned when events cannot be loaded.
    type Error;

    /// Loads events for `E` without an ordering guarantee.
    fn load_entity_events(&self, context_id: Uuid) -> EventIteratorResult<E::Event, Self::Error>;
}

/// Loads umbrella events for a [`ModelEventStore`].
#[doc(hidden)]
pub trait ModelEventLoader<M: ModelEvents> {
    /// Error returned when events cannot be loaded.
    type Error;

    /// Loads model events without an ordering guarantee.
    fn load_model_events(
        &self,
        context_id: Uuid,
    ) -> EventIteratorResult<M::UmbrellaEvent, Self::Error>;
}

/// Marks an entity as belonging to analysis model `M`.
#[doc(hidden)]
pub trait StoredEntity<M>: Entity {}
