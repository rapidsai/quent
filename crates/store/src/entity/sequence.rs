// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed sequences of stored entity events.

use std::num::NonZeroUsize;

use quent_events::EntityMarker;
use quent_time::{OrderKey, OrderedCollector, TimeUnixNanoSec};
use uuid::Uuid;

use crate::{EntityHandle, Error, Id, Timestamped};

// TODO(johanpel): Deduplicate with
//                 quent_analyzer::entity::native::AnalyzedEntity
//                 as part of rapidsai/quent#516.
/// A non-empty, timestamp-ordered event sequence for one entity instance.
pub struct EventSequence<E: EntityMarker> {
    /// The ID of entity that emitted these events.
    id: Id<E>,
    /// The ordered events.
    events: Vec<Timestamped<E::Payload>>,
}

impl<E> Clone for EventSequence<E>
where
    E: EntityMarker,
    E::Payload: Clone,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id(),
            events: self.events.clone(),
        }
    }
}

impl<E: EntityMarker> EventSequence<E> {
    pub(crate) fn try_from_ordered(
        id: Uuid,
        events: OrderedCollector<Timestamped<E::Payload>>,
    ) -> Result<Self, Error>
    where
        E::Payload: OrderKey,
    {
        let events = events.into_inner();
        if events.is_empty() {
            return Err(Error::EmptyEventSequence(id));
        }
        Ok(Self {
            id: Id::new(id),
            events,
        })
    }

    /// Returns the typed entity ID.
    pub fn id(&self) -> Id<E> {
        // don't require clone/copy on E
        Id::new(self.id.uuid())
    }

    /// Returns a slice of events in timestamp order.
    pub fn events(&self) -> &[Timestamped<E::Payload>] {
        &self.events
    }

    /// Returns the number of recorded events.
    pub fn event_count(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.events.len()).expect("event sequence is non-empty")
    }

    /// Returns the first event timestamp.
    pub fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        // Self can only be constructed with at least one event so unwrap is fine.
        self.events.first().unwrap().timestamp
    }

    /// Returns the last event timestamp.
    pub fn latest_timestamp(&self) -> TimeUnixNanoSec {
        // Self can only be constructed with at least one event so unwrap is fine.
        self.events.last().unwrap().timestamp
    }

    /// Consumes the sequence and returns its events in timestamp order.
    pub fn into_events(self) -> Vec<Timestamped<E::Payload>> {
        self.events
    }
}

impl<E: EntityMarker> EntityHandle for EventSequence<E> {
    type Entity = E;

    fn id(&self) -> Id<E> {
        self.id()
    }

    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        Self::earliest_timestamp(self)
    }

    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        Self::latest_timestamp(self)
    }

    fn num_events(&self) -> NonZeroUsize {
        self.event_count()
    }
}

impl<E: EntityMarker> AsRef<Self> for EventSequence<E> {
    fn as_ref(&self) -> &Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use quent_events::EventPayload;

    use super::*;

    struct TestEntity;

    impl EntityMarker for TestEntity {
        type Payload = Self;
    }

    impl EventPayload for TestEntity {
        const NAME: &'static str = "TestEntity";
    }

    impl OrderKey for TestEntity {
        type Key = ();

        fn order_key(&self) -> Self::Key {}
    }

    // Checks an empty collector cannot create a sequence.
    #[test]
    fn rejects_empty_sequence() {
        let id = Uuid::from_u128(1);
        assert!(matches!(
            EventSequence::<TestEntity>::try_from_ordered(id, OrderedCollector::default()),
            Err(Error::EmptyEventSequence(actual)) if actual == id
        ));
    }
}
