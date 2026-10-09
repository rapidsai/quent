// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Rust-native in-memory entity storage.

use std::marker::PhantomData;
use std::num::NonZeroUsize;

use quent_events::{EntityMarker, Event};
use quent_time::{OrderedCollector, TimeUnixNanoSec};
use rustc_hash::FxHashMap as HashMap;
use uuid::Uuid;

use super::sequence::EventSequence;
use super::{
    BorrowedEventSequenceStore, EntityHandle, EntityStore, EventStorage, OwnedEventSequenceStore,
};
use crate::Error;

/// Properties common to all stored entity types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityProperties {
    pub id: Uuid,
    pub earliest_timestamp: TimeUnixNanoSec,
    pub latest_timestamp: TimeUnixNanoSec,
    /// Counts events across all event types.
    pub event_count: NonZeroUsize,
}

/// Storage for events of an entity E stored in S.
#[doc(hidden)]
pub struct NativeEntity<E: EntityMarker, S: EventStorage<E>> {
    /// Properties of this entity that apply to every entity type.
    properties: EntityProperties,
    /// Events grouped by their schema event type.
    event_storage: S,
    marker: PhantomData<fn() -> E>,
}

impl<E: EntityMarker, S: EventStorage<E>> NativeEntity<E, S> {
    /// Returns this entity's type-agnostic properties.
    pub fn properties(&self) -> &EntityProperties {
        &self.properties
    }

    /// Borrows the internal event storage.
    #[doc(hidden)]
    pub fn events(&self) -> &S {
        &self.event_storage
    }
}

impl<E: EntityMarker, S: EventStorage<E>> EntityHandle for NativeEntity<E, S> {
    type Entity = E;

    fn id(&self) -> Uuid {
        self.properties.id
    }
}

/// # Errors
///
/// Returns the first error from [`EventStorage::push`].
impl<E: EntityMarker, S: EventStorage<E>> TryFrom<EventSequence<E>> for NativeEntity<E, S> {
    type Error = Error;

    fn try_from(sequence: EventSequence<E>) -> Result<Self, Self::Error> {
        let mut entity = Self {
            properties: EntityProperties {
                id: sequence.id(),
                earliest_timestamp: sequence.earliest_timestamp(),
                latest_timestamp: sequence.latest_timestamp(),
                event_count: sequence.event_count(),
            },
            event_storage: S::default(),
            marker: PhantomData,
        };
        for event in sequence.into_events() {
            entity.event_storage.push(event)?;
        }
        Ok(entity)
    }
}

impl<E: EntityMarker, S: EventStorage<E>> std::fmt::Debug for NativeEntity<E, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeEntity")
            .field("properties", &self.properties)
            .finish_non_exhaustive()
    }
}

/// An owned handle for one entity marker.
pub struct Handle<E> {
    id: Uuid,
    marker: PhantomData<fn() -> E>,
}

impl<E: EntityMarker> EntityHandle for Handle<E> {
    type Entity = E;

    fn id(&self) -> Uuid {
        self.id
    }
}

/// Stores one entity marker's events in memory, grouped by UUID.
pub struct Store<E: EntityMarker> {
    entities: HashMap<Uuid, EventSequence<E>>,
}

impl<E: EntityMarker> Store<E> {
    /// Groups events by UUID and orders each sequence by timestamp.
    ///
    /// # Performance notes
    ///
    /// For n input events, construction is O(n) when each entity's events
    /// arrive in timestamp order. Inserting m out-of-order events for one
    /// entity can take O(m^2).
    ///
    /// Practically speaking, when leveraging Quent's default channels and
    /// exporters in instrumentation and importers, out-of-order events will
    /// only occur when entities emit events from multiple contexts.
    pub fn new(events: impl IntoIterator<Item = Event<E::Payload>>) -> Self {
        // OrderedCollector appends the common in-order case and inserts late arrivals in order.
        let mut entities: HashMap<Uuid, OrderedCollector<Event<E::Payload>>> = HashMap::default();
        for event in events {
            entities.entry(event.id).or_default().push(event);
        }
        let entities = entities
            .into_iter()
            .map(|(id, events)| {
                (
                    id,
                    EventSequence::from_ordered_events(id, events.into_inner()),
                )
            })
            .collect();
        Self { entities }
    }

    /// Consumes the store and yields sequences in unspecified order.
    pub fn into_sequences(self) -> impl Iterator<Item = EventSequence<E>> {
        self.entities.into_values()
    }
}

impl<E: EntityMarker> EntityStore<E> for Store<E> {
    type Error = Error;
    type Handle = Handle<E>;

    fn entities(&self) -> Result<impl Iterator<Item = Self::Handle>, Self::Error> {
        Ok(self.entities.keys().copied().map(|id| Handle {
            id,
            marker: PhantomData,
        }))
    }

    fn entity(&self, entity_id: Uuid) -> Result<Option<Self::Handle>, Self::Error> {
        Ok(self.entities.contains_key(&entity_id).then_some(Handle {
            id: entity_id,
            marker: PhantomData,
        }))
    }

    fn earliest_timestamp(&self, handle: &Self::Handle) -> Result<TimeUnixNanoSec, Self::Error> {
        self.entities
            .get(&handle.id)
            .map(EventSequence::earliest_timestamp)
            .ok_or(Error::MissingEntity(handle.id))
    }

    fn latest_timestamp(&self, handle: &Self::Handle) -> Result<TimeUnixNanoSec, Self::Error> {
        self.entities
            .get(&handle.id)
            .map(EventSequence::latest_timestamp)
            .ok_or(Error::MissingEntity(handle.id))
    }

    fn num_events(&self, handle: &Self::Handle) -> Result<NonZeroUsize, Self::Error> {
        self.entities
            .get(&handle.id)
            .map(EventSequence::event_count)
            .ok_or(Error::MissingEntity(handle.id))
    }
}

impl<E: EntityMarker> BorrowedEventSequenceStore<E> for Store<E> {
    fn event_sequence(&self, handle: &Self::Handle) -> Result<&EventSequence<E>, Self::Error> {
        self.entities
            .get(&handle.id)
            .ok_or(Error::MissingEntity(handle.id))
    }
}

impl<E: EntityMarker> OwnedEventSequenceStore<E> for Store<E>
where
    E::Payload: Clone,
{
    fn event_sequence_owned(&self, handle: &Self::Handle) -> Result<EventSequence<E>, Self::Error> {
        self.entities
            .get(&handle.id)
            .cloned()
            .ok_or(Error::MissingEntity(handle.id))
    }
}

#[cfg(test)]
mod tests {
    use quent_events::{EntityMarker, EventPayload};

    use super::*;

    struct Task;

    impl EntityMarker for Task {
        type Payload = TaskEvent;
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TaskEvent(&'static str);

    impl EventPayload for TaskEvent {
        const NAME: &'static str = "Task";
    }

    #[test]
    fn groups_entities_and_preserves_equal_timestamp_input_order() {
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let store = Store::<Task>::new([
            Event::new(second, 3, TaskEvent("other")),
            Event::new(first, 2, TaskEvent("late")),
            Event::new(first, 1, TaskEvent("early")),
            Event::new(first, 1, TaskEvent("equal")),
        ]);

        let mut ids = store
            .entities()
            .unwrap()
            .map(|handle| handle.id())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, [first, second]);
        let handle = store.entity(first).unwrap().unwrap();
        assert_eq!(Handle::<Task>::type_name(), "Task");
        assert_eq!(store.earliest_timestamp(&handle).unwrap(), 1);
        assert_eq!(store.latest_timestamp(&handle).unwrap(), 2);
        assert_eq!(
            store.num_events(&handle).unwrap(),
            NonZeroUsize::new(3).unwrap()
        );
        let sequence = store.event_sequence(&handle).unwrap();
        let owned = store.event_sequence_owned(&handle).unwrap();
        assert_eq!(owned.id(), first);
        assert_eq!(owned.event_count(), sequence.event_count());
        assert_ne!(owned.events().as_ptr(), sequence.events().as_ptr());
        let mut owned_events = owned.into_events();
        assert_eq!(owned_events[0].data, TaskEvent("early"));
        owned_events[0].data = TaskEvent("changed");
        assert_eq!(sequence.events()[0].data, TaskEvent("early"));
        assert_eq!(sequence.id(), first);
        assert_eq!(sequence.event_count(), NonZeroUsize::new(3).unwrap());
        assert_eq!(sequence.earliest_timestamp(), 1);
        assert_eq!(sequence.latest_timestamp(), 2);
        assert_eq!(
            sequence
                .events()
                .iter()
                .map(|event| &event.data)
                .collect::<Vec<_>>(),
            [&TaskEvent("early"), &TaskEvent("equal"), &TaskEvent("late")]
        );
        assert!(store.entity(Uuid::from_u128(3)).unwrap().is_none());
        let other_store =
            Store::<Task>::new([Event::new(Uuid::from_u128(3), 4, TaskEvent("foreign"))]);
        let foreign_handle = other_store.entity(Uuid::from_u128(3)).unwrap().unwrap();
        assert!(matches!(
            store.earliest_timestamp(&foreign_handle),
            Err(Error::MissingEntity(id)) if id == foreign_handle.id()
        ));
        assert!(matches!(
            store.latest_timestamp(&foreign_handle),
            Err(Error::MissingEntity(id)) if id == foreign_handle.id()
        ));
        assert!(matches!(
            store.num_events(&foreign_handle),
            Err(Error::MissingEntity(id)) if id == foreign_handle.id()
        ));
        assert!(
            matches!(store.event_sequence(&foreign_handle), Err(Error::MissingEntity(id)) if id == foreign_handle.id())
        );
        assert!(
            matches!(store.event_sequence_owned(&foreign_handle), Err(Error::MissingEntity(id)) if id == foreign_handle.id())
        );
        let shared_handle = Store::<Task>::new([Event::new(first, 9, TaskEvent("shared"))])
            .entity(first)
            .unwrap()
            .unwrap();
        assert_eq!(store.earliest_timestamp(&shared_handle).unwrap(), 1);
        assert_eq!(store.num_events(&shared_handle).unwrap().get(), 3);
        let mut lengths = store
            .into_sequences()
            .map(|sequence| sequence.into_events().len())
            .collect::<Vec<_>>();
        lengths.sort_unstable();
        assert_eq!(lengths, [1, 3]);
    }
}
