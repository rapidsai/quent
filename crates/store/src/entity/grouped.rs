// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Native entities with event-type-scoped storage.

use std::marker::PhantomData;

use quent_events::{EntityMarker, Event};
use uuid::Uuid;

use super::{DuplicateOnceEvent, EntityHandle, native::EntityProperties, sequence::EventSequence};

/// Stores an entity's events by event type.
///
/// [`Default`] must produce empty storage. Successful insertions preserve event
/// metadata and insertion order within each group.
pub trait EventStorage<E: EntityMarker>: Default {
    /// Moves an event into its matching group.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateOnceEvent`] if the corresponding once-event group is
    /// occupied, leaving the existing event unchanged.
    fn push(&mut self, event: Event<E::Payload>) -> Result<(), DuplicateOnceEvent>;
}

/// Owns an entity's identity and event storage.
///
/// Conversion from [`EventSequence`] inserts events in timestamp order without
/// requiring [`Clone`] and stops at the first grouping error.
pub struct NativeEntity<E: EntityMarker, S: EventStorage<E>> {
    properties: EntityProperties,
    event_storage: S,
    marker: PhantomData<fn() -> E>,
}

impl<E: EntityMarker, S: EventStorage<E>> NativeEntity<E, S> {
    /// Returns metadata for the complete input event sequence.
    pub fn properties(&self) -> &EntityProperties {
        &self.properties
    }

    /// Borrows the event storage.
    pub fn event_storage(&self) -> &S {
        &self.event_storage
    }
}

impl<E: EntityMarker, S: EventStorage<E>> EntityHandle for NativeEntity<E, S> {
    type Entity = E;

    fn id(&self) -> Uuid {
        self.properties.id
    }
}

impl<E: EntityMarker, S: EventStorage<E>> TryFrom<EventSequence<E>> for NativeEntity<E, S> {
    type Error = DuplicateOnceEvent;

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
