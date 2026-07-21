// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Builders for [`Schema`] and its elements.

use std::fmt::Display;
use std::hash::Hash;

use thiserror::Error;

use crate::schema::Map;
use crate::schema::identifier::IdentifierError;
use crate::{Annotations, Entity, Identifier, Record, Schema};

pub mod annotations;
pub mod entity;
pub mod event;
pub mod record;

pub use annotations::AnnotationsBuilder;
pub use entity::EntityBuilder;
pub use event::EventBuilder;
pub use record::RecordBuilder;

/// Error returned while assembling a schema element.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BuilderError {
    /// A name was added more than once within the same collection.
    #[error("duplicate name \"{0}\"")]
    DuplicateName(String),
    /// A name was empty.
    #[error("name must not be empty")]
    EmptyName,
}

pub(crate) fn insert_unique<K, V>(map: &mut Map<K, V>, key: K, value: V) -> Result<(), BuilderError>
where
    K: Eq + Hash + Display,
{
    match map.entry(key) {
        indexmap::map::Entry::Occupied(entry) => {
            Err(BuilderError::DuplicateName(entry.key().to_string()))
        }
        indexmap::map::Entry::Vacant(entry) => {
            entry.insert(value);
            Ok(())
        }
    }
}

/// Builder for a [`Schema`].
pub struct SchemaBuilder {
    name: Identifier,
    entities: Map<Identifier, Entity>,
    records: Map<Identifier, Record>,
    annotations: AnnotationsBuilder,
}

impl SchemaBuilder {
    /// Start a schema named `name`.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            entities: Map::default(),
            records: Map::default(),
            annotations: AnnotationsBuilder::new(),
        }
    }

    /// Start a schema named `name`, validating `name` as an [`Identifier`].
    ///
    /// # Errors
    ///
    /// Errors if `name` is not a valid identifier.
    pub fn try_new(
        name: impl TryInto<Identifier, Error = IdentifierError>,
    ) -> Result<Self, IdentifierError> {
        Ok(Self::new(name.try_into()?))
    }

    /// The name of the schema.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// The entity declared under `name`, if any.
    pub fn entity(&self, name: &Identifier) -> Option<&Entity> {
        self.entities.get(name)
    }

    /// Set an entity, returning the replaced one with the same name, if any.
    pub fn set_entity(&mut self, entity: Entity) -> Option<Entity> {
        self.entities.insert(entity.name().clone(), entity)
    }

    /// Add an entity.
    ///
    /// # Errors
    ///
    /// Errors if its name is already declared.
    pub fn try_insert_entity(&mut self, entity: Entity) -> Result<&mut Self, BuilderError> {
        insert_unique(&mut self.entities, entity.name().clone(), entity)?;
        Ok(self)
    }

    /// Add an entity, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors if its name is already declared.
    pub fn try_with_entity(mut self, entity: Entity) -> Result<Self, BuilderError> {
        self.try_insert_entity(entity)?;
        Ok(self)
    }

    /// Add several entities, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors on the first duplicate name.
    pub fn try_with_entities(
        mut self,
        entities: impl IntoIterator<Item = Entity>,
    ) -> Result<Self, BuilderError> {
        for entity in entities {
            self.try_insert_entity(entity)?;
        }
        Ok(self)
    }

    /// The record declared under `name`, if any.
    pub fn record(&self, name: &Identifier) -> Option<&Record> {
        self.records.get(name)
    }

    /// Set a record, returning the replaced one with the same name, if any.
    pub fn set_record(&mut self, record: Record) -> Option<Record> {
        self.records.insert(record.name().clone(), record)
    }

    /// Add a record.
    ///
    /// # Errors
    ///
    /// Errors if its name is already declared.
    pub fn try_insert_record(&mut self, record: Record) -> Result<&mut Self, BuilderError> {
        insert_unique(&mut self.records, record.name().clone(), record)?;
        Ok(self)
    }

    /// Add a record, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors if its name is already declared.
    pub fn try_with_record(mut self, record: Record) -> Result<Self, BuilderError> {
        self.try_insert_record(record)?;
        Ok(self)
    }

    /// Add several records, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors on the first duplicate name.
    pub fn try_with_records(
        mut self,
        records: impl IntoIterator<Item = Record>,
    ) -> Result<Self, BuilderError> {
        for record in records {
            self.try_insert_record(record)?;
        }
        Ok(self)
    }

    /// The annotations of the schema.
    pub fn annotations(&self) -> &AnnotationsBuilder {
        &self.annotations
    }

    /// The annotations of the schema.
    pub fn annotations_mut(&mut self) -> &mut AnnotationsBuilder {
        &mut self.annotations
    }

    /// Set the schema's annotations, replacing any added so far, and return
    /// the builder for chaining.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = AnnotationsBuilder::from_annotations(&annotations);
        self
    }

    /// Finish building the schema.
    pub fn build(self) -> Schema {
        Schema::from_parts(
            self.name,
            self.entities,
            self.records,
            self.annotations.build(),
        )
    }
}
