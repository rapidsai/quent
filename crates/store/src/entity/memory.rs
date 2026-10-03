// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! In-memory entity storage.

use std::marker::PhantomData;
use std::num::NonZeroUsize;

use quent_events::{EntityMarker, Event, EventPayload};
use quent_time::{OrderedCollector, TimeUnixNanoSec};
use rustc_hash::FxHashMap as HashMap;
use uuid::Uuid;

use super::sequence::EventSequence;
use super::{BorrowedEventSequenceStore, EntityHandle, EntityStore, OwnedEventSequenceStore};

/// Error returned when a handle's UUID is absent from an in-memory store.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("entity {0} is not in this store")]
pub struct MissingEntity(pub Uuid);

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

    fn type_name(&self) -> &str {
        E::Payload::NAME
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
    /// Practically speaking, when leveraging Quent's current builtin exporters
    /// / importers, out-of-order events will only occur when entities emit
    /// events from multiple contexts.
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
    type Error = MissingEntity;
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
            .ok_or(MissingEntity(handle.id))
    }

    fn latest_timestamp(&self, handle: &Self::Handle) -> Result<TimeUnixNanoSec, Self::Error> {
        self.entities
            .get(&handle.id)
            .map(EventSequence::latest_timestamp)
            .ok_or(MissingEntity(handle.id))
    }

    fn num_events(&self, handle: &Self::Handle) -> Result<NonZeroUsize, Self::Error> {
        self.entities
            .get(&handle.id)
            .map(EventSequence::event_count)
            .ok_or(MissingEntity(handle.id))
    }
}

impl<E: EntityMarker> BorrowedEventSequenceStore<E> for Store<E> {
    fn event_sequence(&self, handle: &Self::Handle) -> Result<&EventSequence<E>, Self::Error> {
        self.entities
            .get(&handle.id)
            .ok_or(MissingEntity(handle.id))
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
            .ok_or(MissingEntity(handle.id))
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
        assert_eq!(handle.type_name(), "Task");
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
        assert_eq!(
            store.earliest_timestamp(&foreign_handle),
            Err(MissingEntity(foreign_handle.id()))
        );
        assert_eq!(
            store.latest_timestamp(&foreign_handle),
            Err(MissingEntity(foreign_handle.id()))
        );
        assert_eq!(
            store.num_events(&foreign_handle),
            Err(MissingEntity(foreign_handle.id()))
        );
        assert!(
            matches!(store.event_sequence(&foreign_handle), Err(MissingEntity(id)) if id == foreign_handle.id())
        );
        assert!(
            matches!(store.event_sequence_owned(&foreign_handle), Err(MissingEntity(id)) if id == foreign_handle.id())
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
