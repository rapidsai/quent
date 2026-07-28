// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::builder::{AnnotationsBuilder, BuilderError, collect_unique};
use crate::schema::identifier::IdentifierError;
use crate::{Annotations, Cardinality, Event, Field, Identifier};

/// Builder for an [`Event`].
pub struct EventBuilder {
    name: Identifier,
    cardinality: Cardinality,
    payload: Vec<Field>,
    annotations: AnnotationsBuilder,
}

impl EventBuilder {
    /// Start an event named `name` with the given `cardinality`.
    pub fn new(name: Identifier, cardinality: Cardinality) -> Self {
        Self {
            name,
            cardinality,
            payload: Vec::new(),
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

    /// Add a payload field, returning the builder for chaining.
    pub fn with_field(mut self, field: Field) -> Self {
        self.payload.push(field);
        self
    }

    /// Add several payload fields, returning the builder for chaining.
    pub fn with_fields(mut self, fields: impl IntoIterator<Item = Field>) -> Self {
        self.payload.extend(fields);
        self
    }

    /// Set the event's annotations, replacing any added so far, and return the
    /// builder for chaining.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = AnnotationsBuilder::from_annotations(&annotations);
        self
    }

    /// Finish building the event.
    ///
    /// # Errors
    ///
    /// Errors if a field name is repeated or the annotations are invalid.
    pub fn build(self) -> Result<Event, BuilderError> {
        let Self {
            name,
            cardinality,
            payload,
            annotations,
        } = self;
        let payload = collect_unique(payload, |field| field.name().clone())?;
        let annotations = annotations.build()?;
        Ok(Event::from_parts(name, cardinality, payload, annotations))
    }
}
