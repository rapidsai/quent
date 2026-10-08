// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed access to stored entities.

use std::num::NonZeroUsize;

use quent_events::{EntityMarker, EventPayload};
use quent_time::TimeUnixNanoSec;
use uuid::Uuid;

use self::sequence::EventSequence;

pub mod native;
pub mod sequence;

// TODO(johanpel): Deduplicate entity identity and timestamp access with quent-analyzer
// as part of rapidsai/quent#516.
/// Identifies an entity in a store.
///
/// # Note
///
/// This is a trait rather than a concrete type wrapping only a UUID to allow
/// (typically immutable) stores to add additional indexing information.
pub trait EntityHandle {
    /// Entity marker addressed by this handle.
    type Entity: EntityMarker;

    /// Returns the entity UUID.
    fn id(&self) -> Uuid;

    /// Returns the entity type name.
    fn type_name() -> &'static str {
        <<Self::Entity as EntityMarker>::Payload as EventPayload>::NAME
    }
}

/// Provides access to (entity-type agnostic properties of) stored entities with
/// marker type `E`.
pub trait EntityStore<E: EntityMarker> {
    /// Error returned when entity access fails.
    type Error;
    /// Handle type returned by this store.
    type Handle: EntityHandle<Entity = E>;

    /// Iterates over entity handles.
    fn entities(&self) -> Result<impl Iterator<Item = Self::Handle>, Self::Error>;

    /// Look up an entity by its UUID and return a handle.
    fn entity(&self, entity_id: Uuid) -> Result<Option<Self::Handle>, Self::Error>;

    /// Returns the earliest recorded timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if the handle does not identify an entity in this store.
    fn earliest_timestamp(&self, handle: &Self::Handle) -> Result<TimeUnixNanoSec, Self::Error>;

    /// Returns the latest recorded timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if the handle does not identify an entity in this store.
    fn latest_timestamp(&self, handle: &Self::Handle) -> Result<TimeUnixNanoSec, Self::Error>;

    /// Returns the event count.
    ///
    /// # Errors
    ///
    /// Returns an error if the handle does not identify an entity in this store.
    fn num_events(&self, handle: &Self::Handle) -> Result<NonZeroUsize, Self::Error>;
}

/// Provides borrowed, timestamp-ordered raw event sequences for an entity marker.
pub trait BorrowedEventSequenceStore<E: EntityMarker>: EntityStore<E> {
    /// Borrows the selected sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if the handle does not identify an entity in this store.
    fn event_sequence(&self, handle: &Self::Handle) -> Result<&EventSequence<E>, Self::Error>;
}

/// Provides owned, timestamp-ordered raw event sequences for an entity marker.
pub trait OwnedEventSequenceStore<E: EntityMarker>: EntityStore<E> {
    /// Returns an owned sequence without changing the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the handle does not identify an entity in this store.
    fn event_sequence_owned(&self, handle: &Self::Handle) -> Result<EventSequence<E>, Self::Error>;
}
