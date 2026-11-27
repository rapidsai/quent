// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed access to stored entities.

use std::num::NonZeroUsize;

use quent_events::{EntityMarker, EventPayload, Id};
use quent_time::TimeUnixNanoSec;

use crate::entity::sequence::EventSequence;

pub mod native;
pub mod sequence;

// TODO(johanpel): Deduplicate entity identity and timestamp access with quent-analyzer
// as part of rapidsai/quent#516.
/// Provides identity and event properties for a stored entity.
pub trait EntityHandle {
    /// Entity marker addressed by this handle.
    type Entity: EntityMarker;

    /// Returns the typed entity ID.
    fn id(&self) -> Id<Self::Entity>;

    /// Returns the earliest recorded timestamp.
    fn earliest_timestamp(&self) -> TimeUnixNanoSec;

    /// Returns the latest recorded timestamp.
    fn latest_timestamp(&self) -> TimeUnixNanoSec;

    /// Returns the event count.
    fn num_events(&self) -> NonZeroUsize;

    /// Returns the entity type name.
    fn type_name() -> &'static str {
        <<Self::Entity as EntityMarker>::Payload as EventPayload>::NAME
    }
}

/// Lists typed entity IDs and provides borrowed entity handles.
pub trait EntityStore<E: EntityMarker> {
    /// Error returned when entity access fails.
    type Error;
    /// Handle type returned by this store.
    type Handle<'a>: EntityHandle<Entity = E>
    where
        Self: 'a;

    /// Iterates over typed entity IDs.
    fn entities(&self) -> Result<impl Iterator<Item = Id<E>>, Self::Error>;

    /// Returns a borrowed handle, or `None` when the entity is absent.
    fn entity(&self, entity_id: impl Into<Id<E>>) -> Result<Option<Self::Handle<'_>>, Self::Error>;
}

/// Provides borrowed, timestamp-ordered raw event sequences for an entity marker.
pub trait BorrowedEventSequenceStore<E: EntityMarker>: EntityStore<E> {
    /// Borrows the selected sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity ID is absent from this store.
    fn event_sequence(&self, entity_id: Id<E>) -> Result<&EventSequence<E>, Self::Error>;
}

/// Provides owned, timestamp-ordered raw event sequences for an entity marker.
pub trait OwnedEventSequenceStore<E: EntityMarker>: EntityStore<E> {
    /// Returns an owned sequence without changing the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity ID is absent from this store.
    fn event_sequence_owned(&self, entity_id: Id<E>) -> Result<EventSequence<E>, Self::Error>;
}

impl<T: EntityHandle + ?Sized> EntityHandle for &T {
    type Entity = T::Entity;

    fn id(&self) -> Id<Self::Entity> {
        T::id(self)
    }

    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        T::earliest_timestamp(self)
    }

    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        T::latest_timestamp(self)
    }

    fn num_events(&self) -> NonZeroUsize {
        T::num_events(self)
    }

    fn type_name() -> &'static str {
        T::type_name()
    }
}
