// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Entity discovery and event loading.

use std::collections::HashMap;
use std::marker::PhantomData;

use quent_events::{Event, ModelEvents};
use quent_time::TimeOrderedCollector;
use smallvec::SmallVec;
use uuid::Uuid;

use crate::event::{
    EntityEventLoader, EntityEventStore, EventIterator, ModelEventLoader, ModelEventStore,
    StoredEntity,
};

/// Error returned when constructing a [`ContextSet`].
#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
pub enum ContextSetError {
    /// No contexts were provided.
    #[error("an entity query requires at least one context")]
    Empty,
}

/// An ordered, non-empty set of contexts included in an entity query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextSet {
    context_ids: SmallVec<[Uuid; 1]>,
}

impl ContextSet {
    /// Creates a context set, removing duplicate IDs while preserving their first occurrence.
    ///
    /// # Errors
    ///
    /// Returns [`ContextSetError::Empty`] when `context_ids` is empty.
    pub fn try_new(context_ids: impl IntoIterator<Item = Uuid>) -> Result<Self, ContextSetError> {
        let mut unique = SmallVec::new();
        for context_id in context_ids {
            if !unique.contains(&context_id) {
                unique.push(context_id);
            }
        }
        if unique.is_empty() {
            return Err(ContextSetError::Empty);
        }
        Ok(Self {
            context_ids: unique,
        })
    }

    /// Returns context IDs in query order.
    pub fn as_slice(&self) -> &[Uuid] {
        &self.context_ids
    }
}

struct EntityLocation {
    entity_id: Uuid,
    context_ids: SmallVec<[Uuid; 1]>,
}

impl EntityLocation {
    fn new(entity_id: Uuid, context_id: Uuid) -> Self {
        Self {
            entity_id,
            context_ids: SmallVec::from_buf([context_id]),
        }
    }

    fn add_context(&mut self, context_id: Uuid) {
        if self.context_ids.last() != Some(&context_id) {
            self.context_ids.push(context_id);
        }
    }
}

/// A typed entity locator discovered from one or more contexts.
///
/// A handle records contributing contexts but is not a snapshot. Loading its
/// events reads the backing store's current contents for those contexts.
pub struct EntityHandle<M, E> {
    entity_id: Uuid,
    context_ids: SmallVec<[Uuid; 1]>,
    marker: PhantomData<fn() -> (M, E)>,
}

impl<M, E> EntityHandle<M, E> {
    fn from_location(location: EntityLocation) -> Self {
        Self {
            entity_id: location.entity_id,
            context_ids: location.context_ids,
            marker: PhantomData,
        }
    }

    /// Returns the entity UUID.
    pub fn id(&self) -> Uuid {
        self.entity_id
    }

    /// Returns the contexts that contributed events in query order.
    pub fn contexts(&self) -> &[Uuid] {
        &self.context_ids
    }

    /// Loads this entity's events and orders them by timestamp.
    ///
    /// Equal timestamps retain context order followed by the backing store's
    /// per-context input order. The result is not a causal ordering across
    /// contexts.
    pub fn load_events<S>(
        &self,
        store: &S,
    ) -> Result<TimeOrderedCollector<Event<E::Event>>, <S as EntityEventStore<M>>::Error>
    where
        E: StoredEntity<M>,
        S: EntityStore<M> + EntityEventLoader<E, Error = <S as EntityEventStore<M>>::Error>,
    {
        load_ordered_events(&self.context_ids, self.entity_id, |context_id| {
            store.entity_events::<E>(context_id)
        })
    }
}

/// A model-wide entity locator discovered through umbrella events.
///
/// A handle records contributing contexts but is not a snapshot. Loading its
/// events reads the backing store's current contents for those contexts.
pub struct AnyEntityHandle<M: ModelEvents> {
    entity_id: Uuid,
    context_ids: SmallVec<[Uuid; 1]>,
    marker: PhantomData<fn() -> M>,
}

impl<M: ModelEvents> AnyEntityHandle<M> {
    fn from_location(location: EntityLocation) -> Self {
        Self {
            entity_id: location.entity_id,
            context_ids: location.context_ids,
            marker: PhantomData,
        }
    }

    /// Returns the entity UUID.
    pub fn id(&self) -> Uuid {
        self.entity_id
    }

    /// Returns the contexts that contributed events in query order.
    pub fn contexts(&self) -> &[Uuid] {
        &self.context_ids
    }

    /// Loads this entity's umbrella events and orders them by timestamp.
    ///
    /// Equal timestamps retain context order followed by the backing store's
    /// per-context input order. The result is not a causal ordering across
    /// contexts.
    pub fn load_events<S>(
        &self,
        store: &S,
    ) -> Result<TimeOrderedCollector<Event<M::UmbrellaEvent>>, <S as EntityEventStore<M>>::Error>
    where
        S: ModelEntityStore<M> + ModelEventLoader<M, Error = <S as EntityEventStore<M>>::Error>,
    {
        load_ordered_events(&self.context_ids, self.entity_id, |context_id| {
            store.events(context_id)
        })
    }
}

/// Discovers and loads typed entities from an event store.
///
/// The default methods adapt an [`EntityEventStore`] by scanning its event
/// iterators. Backends opt into this trait explicitly and may override the
/// defaults when they can use an index or another more efficient mechanism.
pub trait EntityStore<M>: EntityEventStore<M> {
    /// Discovers typed entities across `contexts` in UUID order.
    ///
    /// Discovery scans all selected contexts so each returned handle contains
    /// its complete set of contributing contexts. Event payloads are not
    /// retained.
    fn entities<E>(
        &self,
        contexts: &ContextSet,
    ) -> Result<impl Iterator<Item = EntityHandle<M, E>>, <Self as EntityEventStore<M>>::Error>
    where
        E: StoredEntity<M>,
        Self: EntityEventLoader<E, Error = <Self as EntityEventStore<M>>::Error>,
    {
        discover(contexts, |context_id| self.entity_events::<E>(context_id))
            .map(|locations| locations.into_iter().map(EntityHandle::from_location))
    }

    /// Finds a typed entity and all of its contributing contexts.
    fn entity<E>(
        &self,
        contexts: &ContextSet,
        entity_id: Uuid,
    ) -> Result<Option<EntityHandle<M, E>>, <Self as EntityEventStore<M>>::Error>
    where
        E: StoredEntity<M>,
        Self: EntityEventLoader<E, Error = <Self as EntityEventStore<M>>::Error>,
    {
        find(contexts, entity_id, |context_id| {
            self.entity_events::<E>(context_id)
        })
        .map(|location| location.map(EntityHandle::from_location))
    }
}

/// Discovers and loads entities through model-wide umbrella events.
///
/// The default methods adapt a [`ModelEventStore`] by scanning its umbrella
/// event iterators. Backends opt into this trait explicitly and may override
/// the defaults when they can use an index or another more efficient mechanism.
pub trait ModelEntityStore<M>: EntityStore<M> + ModelEventStore<M>
where
    M: ModelEvents,
{
    /// Discovers entities of any model type across `contexts` in UUID order.
    fn any_entities(
        &self,
        contexts: &ContextSet,
    ) -> Result<impl Iterator<Item = AnyEntityHandle<M>>, <Self as EntityEventStore<M>>::Error>
    where
        Self: ModelEventLoader<M, Error = <Self as EntityEventStore<M>>::Error>,
    {
        discover(contexts, |context_id| self.events(context_id))
            .map(|locations| locations.into_iter().map(AnyEntityHandle::from_location))
    }

    /// Finds an entity of any model type and all of its contributing contexts.
    fn any_entity(
        &self,
        contexts: &ContextSet,
        entity_id: Uuid,
    ) -> Result<Option<AnyEntityHandle<M>>, <Self as EntityEventStore<M>>::Error>
    where
        Self: ModelEventLoader<M, Error = <Self as EntityEventStore<M>>::Error>,
    {
        find(contexts, entity_id, |context_id| self.events(context_id))
            .map(|location| location.map(AnyEntityHandle::from_location))
    }
}

fn discover<T, Error>(
    contexts: &ContextSet,
    mut load: impl FnMut(Uuid) -> Result<EventIterator<T, Error>, Error>,
) -> Result<Vec<EntityLocation>, Error> {
    let mut locations: Vec<EntityLocation> = Vec::new();
    let mut positions: HashMap<Uuid, usize> = HashMap::new();
    for &context_id in contexts.as_slice() {
        for event in load(context_id)? {
            let event = event?;
            if let Some(&position) = positions.get(&event.id) {
                locations[position].add_context(context_id);
            } else {
                positions.insert(event.id, locations.len());
                locations.push(EntityLocation::new(event.id, context_id));
            }
        }
    }
    locations.sort_unstable_by_key(|location| location.entity_id);
    Ok(locations)
}

fn find<T, Error>(
    contexts: &ContextSet,
    entity_id: Uuid,
    mut load: impl FnMut(Uuid) -> Result<EventIterator<T, Error>, Error>,
) -> Result<Option<EntityLocation>, Error> {
    let mut context_ids = SmallVec::new();
    for &context_id in contexts.as_slice() {
        let mut found = false;
        for event in load(context_id)? {
            if event?.id == entity_id {
                found = true;
                break;
            }
        }
        if found {
            context_ids.push(context_id);
        }
    }
    Ok((!context_ids.is_empty()).then_some(EntityLocation {
        entity_id,
        context_ids,
    }))
}

fn load_ordered_events<T, Error>(
    context_ids: &[Uuid],
    entity_id: Uuid,
    mut load: impl FnMut(Uuid) -> Result<EventIterator<T, Error>, Error>,
) -> Result<TimeOrderedCollector<Event<T>>, Error> {
    let mut events = TimeOrderedCollector::default();
    for &context_id in context_ids {
        for event in load(context_id)? {
            let event = event?;
            if event.id == entity_id {
                events.push(event);
            }
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use quent_events::{Entity, EntityEvent};

    use super::*;

    struct TestModel;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct AlphaEvent(&'static str);

    impl EntityEvent for AlphaEvent {
        const NAME: &'static str = "Alpha";
    }

    struct Alpha;

    impl Entity for Alpha {
        type Event = AlphaEvent;
    }

    impl StoredEntity<TestModel> for Alpha {}

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum TestEvent {
        Alpha(&'static str),
        Beta(&'static str),
    }

    impl ModelEvents for TestModel {
        type UmbrellaEvent = TestEvent;
    }

    type Stored<T> = HashMap<Uuid, Vec<(Uuid, u64, T)>>;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum LoadKind {
        Typed,
        Umbrella,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TestError(Uuid);

    #[derive(Default)]
    struct TestStore {
        alpha: Stored<AlphaEvent>,
        umbrella: Stored<TestEvent>,
        calls: RefCell<Vec<(LoadKind, Uuid)>>,
        fail_context: Option<Uuid>,
        item_fail_context: Option<Uuid>,
    }

    impl TestStore {
        fn load<T: Clone + 'static>(
            &self,
            stored: &Stored<T>,
            kind: LoadKind,
            context_id: Uuid,
        ) -> Result<EventIterator<T, TestError>, TestError> {
            self.calls.borrow_mut().push((kind, context_id));
            if self.fail_context == Some(context_id) {
                return Err(TestError(context_id));
            }
            if self.item_fail_context == Some(context_id) {
                return Ok(Box::new(std::iter::once(Err(TestError(context_id)))));
            }
            let events = stored.get(&context_id).cloned().unwrap_or_default();
            Ok(Box::new(events.into_iter().map(|(id, timestamp, data)| {
                Ok(Event::new(id, timestamp, data))
            })))
        }
    }

    impl EntityEventStore<TestModel> for TestStore {
        type Error = TestError;
    }

    impl EntityEventLoader<Alpha> for TestStore {
        type Error = TestError;

        fn load_entity_events(
            &self,
            context_id: Uuid,
        ) -> Result<EventIterator<AlphaEvent, Self::Error>, Self::Error> {
            self.load(&self.alpha, LoadKind::Typed, context_id)
        }
    }

    impl EntityStore<TestModel> for TestStore {}

    impl ModelEventStore<TestModel> for TestStore {}

    impl ModelEventLoader<TestModel> for TestStore {
        type Error = TestError;

        fn load_model_events(
            &self,
            context_id: Uuid,
        ) -> Result<EventIterator<TestEvent, Self::Error>, Self::Error> {
            self.load(&self.umbrella, LoadKind::Umbrella, context_id)
        }
    }

    impl ModelEntityStore<TestModel> for TestStore {}

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    #[test]
    fn context_set_is_non_empty_ordered_and_deduplicated() {
        let first = id(1);
        let second = id(2);

        assert_eq!(ContextSet::try_new([]), Err(ContextSetError::Empty));
        assert_eq!(
            ContextSet::try_new([first, second, first])
                .unwrap()
                .as_slice(),
            [first, second]
        );
    }

    #[test]
    fn typed_handles_group_contexts_and_load_events_on_demand() {
        let first_context = id(1);
        let second_context = id(2);
        let excluded_context = id(3);
        let first_entity = id(10);
        let second_entity = id(20);
        let mut store = TestStore::default();
        store.alpha.insert(
            first_context,
            vec![
                (first_entity, 20, AlphaEvent("late")),
                (first_entity, 10, AlphaEvent("first-context-1")),
                (first_entity, 10, AlphaEvent("first-context-2")),
                (second_entity, 5, AlphaEvent("other-entity")),
            ],
        );
        store.alpha.insert(
            second_context,
            vec![(first_entity, 10, AlphaEvent("second-context"))],
        );
        store.alpha.insert(
            excluded_context,
            vec![(first_entity, 1, AlphaEvent("excluded"))],
        );
        let contexts = ContextSet::try_new([second_context, first_context]).unwrap();

        let handles = store
            .entities::<Alpha>(&contexts)
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(
            handles.iter().map(EntityHandle::id).collect::<Vec<_>>(),
            [first_entity, second_entity]
        );
        assert_eq!(handles[0].contexts(), [second_context, first_context]);
        assert_eq!(handles[1].contexts(), [first_context]);
        assert_eq!(
            *store.calls.borrow(),
            [
                (LoadKind::Typed, second_context),
                (LoadKind::Typed, first_context),
            ]
        );

        let events = handles[0]
            .load_events(&store)
            .unwrap()
            .into_inner()
            .into_iter()
            .map(|event| event.data.0)
            .collect::<Vec<_>>();

        assert_eq!(
            events,
            [
                "second-context",
                "first-context-1",
                "first-context-2",
                "late",
            ]
        );
        assert_eq!(
            &store.calls.borrow()[2..],
            [
                (LoadKind::Typed, second_context),
                (LoadKind::Typed, first_context),
            ]
        );
        assert!(
            !store
                .calls
                .borrow()
                .iter()
                .any(|(_, context_id)| *context_id == excluded_context)
        );
    }

    #[test]
    fn typed_lookup_finds_all_contributing_contexts_and_reports_absence() {
        let first_context = id(1);
        let second_context = id(2);
        let entity_id = id(10);
        let mut store = TestStore::default();
        store
            .alpha
            .insert(first_context, vec![(entity_id, 1, AlphaEvent("first"))]);
        store
            .alpha
            .insert(second_context, vec![(entity_id, 2, AlphaEvent("second"))]);
        let contexts = ContextSet::try_new([first_context, second_context]).unwrap();

        let handle = store
            .entity::<Alpha>(&contexts, entity_id)
            .unwrap()
            .unwrap();

        assert_eq!(handle.id(), entity_id);
        assert_eq!(handle.contexts(), [first_context, second_context]);
        assert!(store.entity::<Alpha>(&contexts, id(99)).unwrap().is_none());
    }

    #[test]
    fn umbrella_handles_group_and_load_all_event_types() {
        let first_context = id(1);
        let second_context = id(2);
        let first_entity = id(10);
        let second_entity = id(20);
        let mut store = TestStore::default();
        store.umbrella.insert(
            first_context,
            vec![
                (first_entity, 2, TestEvent::Alpha("alpha")),
                (second_entity, 1, TestEvent::Beta("other")),
            ],
        );
        store.umbrella.insert(
            second_context,
            vec![(first_entity, 1, TestEvent::Beta("beta"))],
        );
        let contexts = ContextSet::try_new([first_context, second_context]).unwrap();

        let handles = store.any_entities(&contexts).unwrap().collect::<Vec<_>>();
        let lookup = store.any_entity(&contexts, first_entity).unwrap().unwrap();

        assert_eq!(
            handles.iter().map(AnyEntityHandle::id).collect::<Vec<_>>(),
            [first_entity, second_entity]
        );
        assert_eq!(lookup.contexts(), [first_context, second_context]);
        assert_eq!(
            lookup
                .load_events(&store)
                .unwrap()
                .into_inner()
                .into_iter()
                .map(|event| event.data)
                .collect::<Vec<_>>(),
            [TestEvent::Beta("beta"), TestEvent::Alpha("alpha")]
        );
        assert!(store.any_entity(&contexts, id(99)).unwrap().is_none());
    }

    #[test]
    fn discovery_propagates_backend_errors_without_partial_handles() {
        let failed_context = id(2);
        let store = TestStore {
            fail_context: Some(failed_context),
            ..TestStore::default()
        };
        let contexts = ContextSet::try_new([id(1), failed_context]).unwrap();

        let error = match store.entities::<Alpha>(&contexts) {
            Ok(_) => panic!("discovery unexpectedly succeeded"),
            Err(error) => error,
        };

        assert_eq!(error, TestError(failed_context));
    }

    #[test]
    fn event_iteration_errors_propagate_during_discovery_and_loading() {
        let context_id = id(1);
        let entity_id = id(10);
        let contexts = ContextSet::try_new([context_id]).unwrap();
        let mut store = TestStore::default();
        store
            .alpha
            .insert(context_id, vec![(entity_id, 1, AlphaEvent("event"))]);

        let handle = store
            .entity::<Alpha>(&contexts, entity_id)
            .unwrap()
            .unwrap();
        store.item_fail_context = Some(context_id);

        let discovery_error = match store.entities::<Alpha>(&contexts) {
            Ok(_) => panic!("discovery unexpectedly succeeded"),
            Err(error) => error,
        };
        let loading_error = match handle.load_events(&store) {
            Ok(_) => panic!("event loading unexpectedly succeeded"),
            Err(error) => error,
        };

        assert_eq!(discovery_error, TestError(context_id));
        assert_eq!(loading_error, TestError(context_id));
    }
}
