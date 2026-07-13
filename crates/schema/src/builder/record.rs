// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::builder::{AnnotationsBuilder, BuilderError, insert_unique};
use crate::schema::Map;
use crate::schema::identifier::IdentifierError;
use crate::{Annotations, Field, Identifier, Record};

/// Builder for a [`Record`].
pub struct RecordBuilder {
    name: Identifier,
    fields: Map<Identifier, Field>,
    annotations: AnnotationsBuilder,
}

impl RecordBuilder {
    /// Start a record named `name`.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            fields: Map::default(),
            annotations: AnnotationsBuilder::new(),
        }
    }

    /// Start a record named `name`, validating `name` as an [`Identifier`].
    ///
    /// # Errors
    ///
    /// Errors if `name` is not a valid identifier.
    pub fn try_new(
        name: impl TryInto<Identifier, Error = IdentifierError>,
    ) -> Result<Self, IdentifierError> {
        Ok(Self::new(name.try_into()?))
    }

    /// The name of the record.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// The field declared under `name`, if any.
    pub fn field(&self, name: &Identifier) -> Option<&Field> {
        self.fields.get(name)
    }

    /// Set a field, returning the replaced one with the same name, if any.
    pub fn set_field(&mut self, field: Field) -> Option<Field> {
        self.fields.insert(field.name().clone(), field)
    }

    /// Add a field.
    ///
    /// # Errors
    ///
    /// Errors if its name is already declared.
    pub fn try_insert_field(&mut self, field: Field) -> Result<&mut Self, BuilderError> {
        insert_unique(&mut self.fields, field.name().clone(), field)?;
        Ok(self)
    }

    /// Add a field, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors if its name is already declared.
    pub fn try_with_field(mut self, field: Field) -> Result<Self, BuilderError> {
        self.try_insert_field(field)?;
        Ok(self)
    }

    /// Add several fields, returning the builder for chaining.
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

    /// The annotations of the record.
    pub fn annotations(&self) -> &AnnotationsBuilder {
        &self.annotations
    }

    /// The annotations of the record.
    pub fn annotations_mut(&mut self) -> &mut AnnotationsBuilder {
        &mut self.annotations
    }

    /// Set the record's annotations, replacing any added so far, and return
    /// the builder for chaining.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = AnnotationsBuilder::from_annotations(&annotations);
        self
    }

    /// Finish building the record.
    pub fn build(self) -> Record {
        Record::from_parts(self.name, self.fields, self.annotations.build())
    }
}
