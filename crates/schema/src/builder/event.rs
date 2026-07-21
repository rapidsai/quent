// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::builder::{AnnotationsBuilder, BuilderError, insert_unique};
use crate::schema::Map;
use crate::schema::identifier::IdentifierError;
use crate::{Annotations, Cardinality, Event, Field, Identifier};

/// Builder for an [`Event`].
pub struct EventBuilder {
    name: Identifier,
    cardinality: Cardinality,
    payload: Map<Identifier, Field>,
    annotations: AnnotationsBuilder,
}

impl EventBuilder {
    /// Start an event named `name` with the given `cardinality`.
    pub fn new(name: Identifier, cardinality: Cardinality) -> Self {
        Self {
            name,
            cardinality,
            payload: Map::default(),
            annotations: AnnotationsBuilder::new(),
        }
    }

    /// Start an event named `name` with the given `cardinality`, validating
    /// `name` as an [`Identifier`].
    ///
    /// # Errors
    ///
    /// Errors if `name` is not a valid identifier.
    pub fn try_new(
        name: impl TryInto<Identifier, Error = IdentifierError>,
        cardinality: Cardinality,
    ) -> Result<Self, IdentifierError> {
        Ok(Self::new(name.try_into()?, cardinality))
    }

    /// The name of the event.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// The cardinality of the event.
    pub fn cardinality(&self) -> Cardinality {
        self.cardinality
    }

    /// Set the cardinality of the event, returning the previous one.
    pub fn set_cardinality(&mut self, cardinality: Cardinality) -> Cardinality {
        std::mem::replace(&mut self.cardinality, cardinality)
    }

    /// The payload field declared under `name`, if any.
    pub fn field(&self, name: &Identifier) -> Option<&Field> {
        self.payload.get(name)
    }

    /// Set a payload field, returning the replaced one with the same name, if
    /// any.
    pub fn set_field(&mut self, field: Field) -> Option<Field> {
        self.payload.insert(field.name().clone(), field)
    }

    /// Add a payload field.
    ///
    /// # Errors
    ///
    /// Errors if its name is already declared.
    pub fn try_insert_field(&mut self, field: Field) -> Result<&mut Self, BuilderError> {
        insert_unique(&mut self.payload, field.name().clone(), field)?;
        Ok(self)
    }

    /// Add a payload field, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors if its name is already declared.
    pub fn try_with_field(mut self, field: Field) -> Result<Self, BuilderError> {
        self.try_insert_field(field)?;
        Ok(self)
    }

    /// Add several payload fields, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors on the first duplicate name.
    pub fn try_with_fields(
        mut self,
        fields: impl IntoIterator<Item = Field>,
    ) -> Result<Self, BuilderError> {
        for field in fields {
            self.try_insert_field(field)?;
        }
        Ok(self)
    }

    /// The annotations of the event.
    pub fn annotations(&self) -> &AnnotationsBuilder {
        &self.annotations
    }

    /// The annotations of the event.
    pub fn annotations_mut(&mut self) -> &mut AnnotationsBuilder {
        &mut self.annotations
    }

    /// Set the event's annotations, replacing any added so far, and return the
    /// builder for chaining.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = AnnotationsBuilder::from_annotations(&annotations);
        self
    }

    /// Finish building the event.
    pub fn build(self) -> Event {
        Event::from_parts(
            self.name,
            self.cardinality,
            self.payload,
            self.annotations.build(),
        )
    }
}
