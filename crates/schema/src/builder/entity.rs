// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::builder::{AnnotationsBuilder, BuilderError, insert_unique};
use crate::schema::Map;
use crate::schema::identifier::IdentifierError;
use crate::{Annotations, Entity, Event, Identifier};

/// Builder for an [`Entity`].
pub struct EntityBuilder {
    name: Identifier,
    events: Map<Identifier, Event>,
    annotations: AnnotationsBuilder,
}

impl EntityBuilder {
    /// Start an entity named `name`.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            events: Map::default(),
            annotations: AnnotationsBuilder::new(),
        }
    }

    /// Start an entity named `name`, validating `name` as an [`Identifier`].
    ///
    /// # Errors
    ///
    /// Errors if `name` is not a valid identifier.
    pub fn try_new(
        name: impl TryInto<Identifier, Error = IdentifierError>,
    ) -> Result<Self, IdentifierError> {
        Ok(Self::new(name.try_into()?))
    }

    /// The name of the entity.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// The event declared under `name`, if any.
    pub fn event(&self, name: &Identifier) -> Option<&Event> {
        self.events.get(name)
    }

    /// Set an event, returning the replaced one with the same name, if any.
    pub fn set_event(&mut self, event: Event) -> Option<Event> {
        self.events.insert(event.name().clone(), event)
    }

    /// Add an event.
    ///
    /// # Errors
    ///
    /// Errors if its name is already declared.
    pub fn try_insert_event(&mut self, event: Event) -> Result<&mut Self, BuilderError> {
        insert_unique(&mut self.events, event.name().clone(), event)?;
        Ok(self)
    }

    /// Add an event, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors if its name is already declared.
    pub fn try_with_event(mut self, event: Event) -> Result<Self, BuilderError> {
        self.try_insert_event(event)?;
        Ok(self)
    }

    /// Add several events, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors on the first duplicate name.
    pub fn try_with_events(
        mut self,
        events: impl IntoIterator<Item = Event>,
    ) -> Result<Self, BuilderError> {
        for event in events {
            self.try_insert_event(event)?;
        }
        Ok(self)
    }

    /// The annotations of the entity.
    pub fn annotations(&self) -> &AnnotationsBuilder {
        &self.annotations
    }

    /// The annotations of the entity.
    pub fn annotations_mut(&mut self) -> &mut AnnotationsBuilder {
        &mut self.annotations
    }

    /// Set the entity's annotations, replacing any added so far, and return
    /// the builder for chaining.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = AnnotationsBuilder::from_annotations(&annotations);
        self
    }

    /// Finish building the entity.
    pub fn build(self) -> Entity {
        Entity::from_parts(self.name, self.events, self.annotations.build())
    }
}
