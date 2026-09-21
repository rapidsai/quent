// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Analysis-time entities in Rust-native in-memory storage.

use quent_events::Event;
use quent_time::TimeUnixNanoSec;
use uuid::Uuid;

use crate::{AnalyzerError, AnalyzerResult};

/// Accumulates events of its associated entity event type.
pub trait EntityEventAccumulator: Default {
    /// The event payload accumulated by this type.
    type Event: quent_events::EntityEvent;

    /// Incorporates one event payload into the retained analysis.
    fn push(&mut self, event: Self::Event);
}

/// Rust-native struct wrapping around an application-specific
/// [`EntityEventAccumulator`].
///
/// This pre-computes generic entity properties that are derived as events are
/// pushed into the accumulator.
pub struct AnalyzedEntity<A: EntityEventAccumulator> {
    id: Uuid,
    earliest_timestamp: TimeUnixNanoSec,
    latest_timestamp: TimeUnixNanoSec,
    accumulator: A,
}

impl<A: EntityEventAccumulator> std::fmt::Debug for AnalyzedEntity<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyzedEntity")
            .field("id", &self.id)
            .field("earliest_timestamp", &self.earliest_timestamp)
            .field("latest_timestamp", &self.latest_timestamp)
            .finish_non_exhaustive()
    }
}

impl<A: EntityEventAccumulator> AnalyzedEntity<A> {
    /// Creates a new analyzed entity from an event.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerError::Validation`] if `id` is nil.
    pub fn try_from_event(event: Event<A::Event>) -> AnalyzerResult<Self> {
        if event.id.is_nil() {
            Err(AnalyzerError::Validation(
                "entity id cannot be nil".to_owned(),
            ))
        } else {
            let mut accumulator = A::default();
            accumulator.push(event.data);
            Ok(Self {
                id: event.id,
                earliest_timestamp: event.timestamp,
                latest_timestamp: event.timestamp,
                accumulator,
            })
        }
    }

    /// Accumulates one event and extends the retained timestamp bounds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerError::Validation`] if the event ID differs from the
    /// entity ID.
    pub fn push(&mut self, event: Event<A::Event>) -> AnalyzerResult<()> {
        if event.id != self.id {
            return Err(AnalyzerError::Validation(format!(
                "event id {} does not match entity id {}",
                event.id, self.id
            )));
        }
        self.earliest_timestamp = self.earliest_timestamp.min(event.timestamp);
        self.latest_timestamp = self.latest_timestamp.max(event.timestamp);
        self.accumulator.push(event.data);
        Ok(())
    }

    /// Returns the event accumulator.
    pub fn accumulator(&self) -> &A {
        &self.accumulator
    }
}

impl<A: EntityEventAccumulator> crate::entity::Entity for AnalyzedEntity<A> {
    fn id(&self) -> Uuid {
        self.id
    }

    fn type_name(&self) -> &str {
        <A::Event as quent_events::EntityEvent>::NAME
    }

    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.earliest_timestamp
    }

    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.latest_timestamp
    }
}

#[cfg(test)]
mod tests {
    use quent_events::EntityEvent;

    use super::*;
    use crate::entity::Entity as _;

    struct Increment;

    impl EntityEvent for Increment {
        const NAME: &'static str = "Increment";
    }

    #[derive(Default)]
    struct Counter(u32);

    impl EntityEventAccumulator for Counter {
        type Event = Increment;

        fn push(&mut self, _event: Self::Event) {
            self.0 += 1;
        }
    }

    #[test]
    fn rejects_event_for_another_entity() {
        let entity_id = Uuid::from_u128(1);
        let mut entity =
            AnalyzedEntity::<Counter>::try_from_event(Event::new(entity_id, 5, Increment)).unwrap();

        assert!(matches!(
            entity.push(Event::new(Uuid::from_u128(2), 10, Increment)),
            Err(AnalyzerError::Validation(_))
        ));
        assert_eq!(entity.accumulator().0, 1);
        assert_eq!(entity.earliest_timestamp(), 5);
        assert_eq!(entity.latest_timestamp(), 5);
    }

    #[test]
    fn retains_observed_event_bounds() {
        let entity_id = Uuid::from_u128(1);
        let mut entity =
            AnalyzedEntity::<Counter>::try_from_event(Event::new(entity_id, 20, Increment))
                .unwrap();

        entity.push(Event::new(entity_id, 10, Increment)).unwrap();

        assert_eq!(entity.earliest_timestamp(), 10);
        assert_eq!(entity.latest_timestamp(), 20);
    }
}
