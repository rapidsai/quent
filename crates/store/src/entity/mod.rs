// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed access to stored entities.

use std::num::NonZeroUsize;

use quent_events::{EntityMarker, Event, EventPayload};
use quent_time::TimeUnixNanoSec;
use uuid::Uuid;

use self::sequence::EventSequence;
use crate::Error;

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

/// Allows a generated native storage container to consume entity events.
///
/// [`Default`] must produce empty storage.
#[doc(hidden)]
pub trait EventStorage<E: EntityMarker>: Default {
    /// Moves an event into its matching group.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateOnceEvent`] if the corresponding once-event
    /// group is occupied, leaving the existing event unchanged.
    fn push(&mut self, event: Event<E::Payload>) -> Result<(), Error>;
}

/// Inserts an event into an empty once-event slot.
///
/// # Errors
///
/// Returns the incoming event when occupied, leaving the existing event unchanged.
#[doc(hidden)]
pub fn insert_once<P>(slot: &mut Option<Event<P>>, event: Event<P>) -> Result<(), Event<P>> {
    if slot.is_some() {
        return Err(event);
    }
    *slot = Some(event);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserting_once_returns_the_rejected_event_and_preserves_the_existing_event() {
        struct Payload(&'static str);

        let id = Uuid::from_u128(1);
        let mut slot = None;
        assert!(insert_once(&mut slot, Event::new(id, 10, Payload("first"))).is_ok());

        let rejected = insert_once(&mut slot, Event::new(id, 10, Payload("second")))
            .err()
            .unwrap();
        assert_eq!(rejected.id, id);
        assert_eq!(rejected.timestamp, 10);
        assert_eq!(rejected.data.0, "second");

        let event = slot.unwrap();
        assert_eq!(event.id, id);
        assert_eq!(event.timestamp, 10);
        assert_eq!(event.data.0, "first");
    }
}
