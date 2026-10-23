// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Rust-native in-memory entity storage implementation support.

use std::{marker::PhantomData, num::NonZeroUsize};

use quent_events::{EntityMarker, Event};
use quent_time::{OrderKey, OrderedCollector, TimeUnixNanoSec};
use rustc_hash::FxHashMap as HashMap;
use uuid::Uuid;

use crate::{
    Error, Timestamped,
    entity::{
        BorrowedEventSequenceStore, EntityHandle, EntityStore, Id, OwnedEventSequenceStore,
        sequence::EventSequence,
    },
    event::EventLoader,
};

/// Maps entity IDs to some T.
pub struct EntityMap<E: EntityMarker, T> {
    entities: HashMap<Uuid, T>,
    marker: PhantomData<fn() -> E>,
}

/// Stores indexed entity event sequences in memory.
pub type NativeStore<E> = EntityMap<E, NativeEntity<E>>;

impl<E: NativeEntityMarker> NativeStore<E>
where
    E::Payload: OrderKey,
{
    /// Groups events by UUID, orders each sequence by timestamp, and indexes it.
    ///
    /// The payload ordering key determines the order of events with equal
    /// timestamps.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EntityIndex`] with [`Error::DuplicateOnceEvent`] as its
    /// source if an entity emits a once-event more than once.
    ///
    /// # Performance notes
    ///
    /// Collecting n events arriving in key order per entity is O(n).
    /// Inserting m out-of-order events for one entity can take O(m^2).
    /// Index construction may add further costs.
    pub fn try_from_events(
        events: impl IntoIterator<Item = Event<E::Payload>>,
    ) -> Result<Self, Error> {
        Self::from_event_results(events.into_iter().map(Ok))
    }

    /// Loads events, grouping by UUID and ordering each sequence before indexing.
    ///
    /// # Errors
    ///
    /// Returns the first loader error and stops reading events. Indexing rejects
    /// duplicate once-events after all events have been read successfully.
    pub fn try_from_loader<L>(loader: &L) -> Result<Self, L::Error>
    where
        L: EventLoader<E> + ?Sized,
        L::Error: From<Error>,
    {
        Self::from_event_results(loader.events()?)
    }

    fn from_event_results<F: From<Error>>(
        events: impl IntoIterator<Item = Result<Event<E::Payload>, F>>,
    ) -> Result<Self, F> {
        let mut entities: HashMap<Uuid, OrderedCollector<Timestamped<E::Payload>>> =
            HashMap::default();
        for event in events {
            let Event {
                id,
                timestamp,
                data,
            } = event?;
            entities
                .entry(id)
                .or_default()
                .push(Timestamped::new(timestamp, data));
        }
        let entities = entities
            .into_iter()
            .map(|(id, events)| {
                let sequence = EventSequence::try_from_ordered(id, events).map_err(F::from)?;
                NativeEntity::try_from(sequence)
                    .map(|entity| (id, entity))
                    .map_err(F::from)
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            entities,
            marker: PhantomData,
        })
    }
}

impl<E: EntityMarker, T: EntityHandle<Entity = E>> EntityStore<E> for EntityMap<E, T> {
    type Error = Error;
    type Handle<'a>
        = &'a T
    where
        Self: 'a;

    fn entities(&self) -> Result<impl Iterator<Item = Id<E>>, Self::Error> {
        Ok(self.entities.keys().copied().map(Id::new))
    }

    fn entity(&self, entity_id: impl Into<Id<E>>) -> Result<Option<Self::Handle<'_>>, Self::Error> {
        Ok(self.entities.get(&entity_id.into().uuid()))
    }
}

impl<E, T> BorrowedEventSequenceStore<E> for EntityMap<E, T>
where
    E: EntityMarker,
    T: EntityHandle<Entity = E> + AsRef<EventSequence<E>>,
{
    fn event_sequence(&self, entity_id: Id<E>) -> Result<&EventSequence<E>, Self::Error> {
        self.entities
            .get(&entity_id.uuid())
            .map(T::as_ref)
            .ok_or(Error::MissingEntity(entity_id.uuid()))
    }
}

impl<E, T> OwnedEventSequenceStore<E> for EntityMap<E, T>
where
    E: EntityMarker,
    T: EntityHandle<Entity = E> + AsRef<EventSequence<E>>,
    E::Payload: Clone,
{
    fn event_sequence_owned(&self, entity_id: Id<E>) -> Result<EventSequence<E>, Self::Error> {
        self.event_sequence(entity_id).cloned()
    }
}

/// Associates an entity marker with its event index type.
pub trait NativeEntityMarker: EntityMarker + Sized {
    type Index: EventIndex<Self>;
}

/// Storage for one entity's event sequence.
///
/// The associated E::Index type is a type which allows indexing into events in
/// the underlying sequence through schema-defined event names. These index
/// types are typically generated.
pub struct NativeEntity<E: NativeEntityMarker> {
    /// The sequence of events this entity emitted.
    sequence: EventSequence<E>,
    /// Positions in the sequence grouped by entity event type.
    event_index: E::Index,
}

impl<E: NativeEntityMarker> NativeEntity<E> {
    /// Borrows the event-type indices.
    pub fn event_index(&self) -> &E::Index {
        &self.event_index
    }

    /// Borrows the raw event sequence.
    pub fn event_sequence(&self) -> &EventSequence<E> {
        &self.sequence
    }
}

impl<E: NativeEntityMarker> AsRef<EventSequence<E>> for NativeEntity<E> {
    fn as_ref(&self) -> &EventSequence<E> {
        &self.sequence
    }
}

impl<E: NativeEntityMarker> EntityHandle for NativeEntity<E> {
    type Entity = E;

    fn id(&self) -> Id<E> {
        self.sequence.id()
    }

    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.sequence.earliest_timestamp()
    }

    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.sequence.latest_timestamp()
    }

    fn num_events(&self) -> NonZeroUsize {
        self.sequence.event_count()
    }
}

/// # Errors
///
/// Returns [`Error::EntityIndex`] with the indexing error as its source.
impl<E: NativeEntityMarker> TryFrom<EventSequence<E>> for NativeEntity<E> {
    type Error = Error;

    fn try_from(sequence: EventSequence<E>) -> Result<Self, Self::Error> {
        let mut entity = Self {
            sequence,
            event_index: E::Index::default(),
        };
        for (position, event) in entity.sequence.events().iter().enumerate() {
            entity
                .event_index
                .record(&event.data, position)
                .map_err(|source| Error::EntityIndex {
                    entity_id: entity.sequence.id().uuid(),
                    source: Box::new(source),
                })?;
        }
        Ok(entity)
    }
}

impl<E: NativeEntityMarker> std::fmt::Debug for NativeEntity<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeEntity")
            .field("id", &self.id().uuid())
            .field("earliest_timestamp", &self.earliest_timestamp())
            .field("latest_timestamp", &self.latest_timestamp())
            .field("event_count", &self.num_events())
            .finish_non_exhaustive()
    }
}

/// Allows generated indices to record positions in an entity event sequence.
///
/// [`Default`] must produce empty indices.
#[doc(hidden)]
pub trait EventIndex<E: EntityMarker>: Default {
    /// Records the position of an event in its matching group.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateOnceEvent`] if the corresponding
    /// once-event group is occupied, leaving the existing index unchanged.
    fn record(&mut self, payload: &E::Payload, position: usize) -> Result<(), Error>;
}

/// Inserts a value into an empty once-event slot.
///
/// # Errors
///
/// Returns the incoming value when occupied, leaving the existing value unchanged.
#[doc(hidden)]
pub fn insert_once<T>(slot: &mut Option<T>, value: T) -> Result<(), T> {
    if slot.is_some() {
        return Err(value);
    }
    *slot = Some(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use quent_events::{EntityMarker, EventPayload};

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct Task;

    impl EntityMarker for Task {
        type Payload = TaskEvent;
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TaskEvent(&'static str);

    impl EventPayload for TaskEvent {
        const NAME: &'static str = "Task";
    }

    impl OrderKey for TaskEvent {
        type Key = ();

        fn order_key(&self) -> Self::Key {}
    }

    impl NativeEntityMarker for Task {
        type Index = TaskIndex;
    }

    #[derive(Default)]
    struct TaskIndex(Option<usize>);

    impl EventIndex<Task> for TaskIndex {
        fn record(&mut self, payload: &TaskEvent, position: usize) -> Result<(), Error> {
            if payload.0 == "once" {
                insert_once(&mut self.0, position)
                    .map_err(|_| Error::DuplicateOnceEvent { event_name: "once" })?;
            }
            Ok(())
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error("open failed")]
        Open,
        #[error("read failed")]
        Read,
        #[error(transparent)]
        Index(#[from] Error),
    }

    struct TestLoader {
        fail_open: bool,
        fail_read: bool,
        duplicate_once: bool,
        reads: Rc<Cell<usize>>,
    }

    impl EventLoader<Task> for TestLoader {
        type Error = TestError;

        fn events(
            &self,
        ) -> Result<crate::event::EventIterator<TaskEvent, Self::Error>, Self::Error> {
            if self.fail_open {
                return Err(TestError::Open);
            }
            let id = Uuid::from_u128(1);
            let payload = if self.duplicate_once { "once" } else { "late" };
            let events = [
                Ok(Event::new(id, 2, TaskEvent(payload))),
                if self.fail_read {
                    Err(TestError::Read)
                } else {
                    Ok(Event::new(
                        id,
                        1,
                        TaskEvent(if self.duplicate_once { "once" } else { "early" }),
                    ))
                },
                Ok(Event::new(id, 1, TaskEvent("equal"))),
                Ok(Event::new(Uuid::from_u128(2), 3, TaskEvent("other"))),
            ];
            let reads = self.reads.clone();
            Ok(Box::new(events.into_iter().inspect(move |_| {
                reads.set(reads.get() + 1);
            })))
        }
    }

    #[test]
    fn loads_and_groups_events_without_losing_equal_timestamp_order() {
        let loader = TestLoader {
            fail_open: false,
            fail_read: false,
            duplicate_once: false,
            reads: Rc::default(),
        };
        let source: &dyn EventLoader<Task, Error = TestError> = &loader;
        let store = NativeStore::<Task>::try_from_loader(source).unwrap();
        assert_eq!(loader.reads.get(), 4);
        assert_eq!(store.entities().unwrap().count(), 2);
        let handle = store.entity(Uuid::from_u128(1)).unwrap().unwrap();
        let sequence = store.event_sequence(handle.id()).unwrap();
        assert_eq!(
            sequence
                .events()
                .iter()
                .map(|event| event.data.0)
                .collect::<Vec<_>>(),
            ["early", "equal", "late"]
        );
    }

    #[test]
    fn propagates_loader_errors_and_stops_at_the_first_read_failure() {
        for (fail_open, expected, reads) in [(true, "open failed", 0), (false, "read failed", 2)] {
            let loader = TestLoader {
                fail_open,
                fail_read: true,
                duplicate_once: false,
                reads: Rc::default(),
            };
            assert!(
                matches!(NativeStore::<Task>::try_from_loader(&loader), Err(error) if error.to_string() == expected)
            );
            assert_eq!(loader.reads.get(), reads);
        }
    }

    #[test]
    fn loader_construction_propagates_duplicate_once_errors() {
        let loader = TestLoader {
            fail_open: false,
            fail_read: false,
            duplicate_once: true,
            reads: Rc::default(),
        };
        assert!(matches!(
            NativeStore::<Task>::try_from_loader(&loader),
            Err(TestError::Index(Error::EntityIndex {
                entity_id,
                source,
            })) if entity_id == Uuid::from_u128(1)
                && matches!(*source, Error::DuplicateOnceEvent { event_name: "once" })
        ));
        assert_eq!(loader.reads.get(), 4);
    }

    #[test]
    fn groups_entities_and_preserves_equal_timestamp_input_order() {
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let store = NativeStore::<Task>::try_from_events([
            Event::new(second, 3, TaskEvent("other")),
            Event::new(first, 2, TaskEvent("late")),
            Event::new(first, 1, TaskEvent("early")),
            Event::new(first, 1, TaskEvent("equal")),
        ])
        .unwrap();

        let mut ids = store
            .entities()
            .unwrap()
            .map(|id| id.uuid())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, [first, second]);
        let handle = store.entity(first).unwrap().unwrap();
        assert_eq!(EventSequence::<Task>::type_name(), "Task");
        assert_eq!(handle.earliest_timestamp(), 1);
        assert_eq!(handle.latest_timestamp(), 2);
        assert_eq!(handle.num_events(), NonZeroUsize::new(3).unwrap());
        let sequence = store.event_sequence(handle.id()).unwrap();
        let owned = store.event_sequence_owned(handle.id()).unwrap();
        assert_eq!(owned.id().uuid(), first);
        assert_eq!(owned.event_count(), sequence.event_count());
        assert_ne!(owned.events().as_ptr(), sequence.events().as_ptr());
        let mut owned_events = owned.into_events();
        assert_eq!(owned_events[0].data, TaskEvent("early"));
        owned_events[0].data = TaskEvent("changed");
        assert_eq!(sequence.events()[0].data, TaskEvent("early"));
        assert_eq!(sequence.id().uuid(), first);
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
        let other_store = NativeStore::<Task>::try_from_events([Event::new(
            Uuid::from_u128(3),
            4,
            TaskEvent("foreign"),
        )])
        .unwrap();
        let foreign_handle = other_store.entity(Uuid::from_u128(3)).unwrap().unwrap();
        assert!(
            matches!(store.event_sequence(foreign_handle.id()), Err(Error::MissingEntity(id)) if id == foreign_handle.id().uuid())
        );
        assert!(
            matches!(store.event_sequence_owned(foreign_handle.id()), Err(Error::MissingEntity(id)) if id == foreign_handle.id().uuid())
        );
    }

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
