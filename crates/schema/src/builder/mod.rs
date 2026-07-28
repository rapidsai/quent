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
    #[error("duplicate name \"{0}\"")]
    DuplicateName(String),
    #[error("name must not be empty")]
    EmptyName,
    #[error("entity must declare at least one event")]
    NoEvents,
}

pub(crate) fn collect_unique<K, V>(
    values: impl IntoIterator<Item = V>,
    mut key: impl FnMut(&V) -> K,
) -> Result<Map<K, V>, BuilderError>
where
    K: Eq + Hash + Display,
{
    let mut map = Map::default();
    for value in values {
        match map.entry(key(&value)) {
            indexmap::map::Entry::Occupied(entry) => {
                return Err(BuilderError::DuplicateName(entry.key().to_string()));
            }
            indexmap::map::Entry::Vacant(entry) => {
                entry.insert(value);
            }
        }
    }
    Ok(map)
}

/// Builder for a [`Schema`].
pub struct SchemaBuilder {
    name: Identifier,
    entities: Vec<Entity>,
    records: Vec<Record>,
    annotations: AnnotationsBuilder,
}

impl SchemaBuilder {
    /// Start a schema named `name`.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            entities: Vec::new(),
            records: Vec::new(),
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

    /// Add an entity, returning the builder for chaining.
    pub fn with_entity(mut self, entity: Entity) -> Self {
        self.entities.push(entity);
        self
    }

    /// Add several entities, returning the builder for chaining.
    pub fn with_entities(mut self, entities: impl IntoIterator<Item = Entity>) -> Self {
        self.entities.extend(entities);
        self
    }

    /// Add a record, returning the builder for chaining.
    pub fn with_record(mut self, record: Record) -> Self {
        self.records.push(record);
        self
    }

    /// Add several records, returning the builder for chaining.
    pub fn with_records(mut self, records: impl IntoIterator<Item = Record>) -> Self {
        self.records.extend(records);
        self
    }

    /// Set the schema's annotations, replacing any added so far, and return
    /// the builder for chaining.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = AnnotationsBuilder::from_annotations(&annotations);
        self
    }

    /// Finish building the schema.
    ///
    /// # Errors
    ///
    /// Errors if a child name is repeated or the annotations are invalid.
    pub fn build(self) -> Result<Schema, BuilderError> {
        let Self {
            name,
            entities,
            records,
            annotations,
        } = self;
        let entities = collect_unique(entities, |entity| entity.path().clone())?;
        let records = collect_unique(records, |record| record.path().clone())?;
        if let Some(path) = entities.keys().find(|path| records.contains_key(*path)) {
            return Err(BuilderError::DuplicateName(path.to_string()));
        }
        let annotations = annotations.build()?;
        Ok(Schema::from_parts(name, entities, records, annotations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{entity, event, path, record};

    #[test]
    fn qualified_type_paths_coexist() {
        let schema = SchemaBuilder::try_new("Schema")
            .unwrap()
            .with_entity(entity("Foo::Q", [event("event", [])]))
            .with_entity(entity("Bar::Q", [event("event", [])]))
            .build()
            .unwrap();

        assert!(schema.entity(&path("Foo::Q")).is_some());
        assert!(schema.entity(&path("Bar::Q")).is_some());
    }

    #[test]
    fn record_and_entity_cannot_share_a_path() {
        let result = SchemaBuilder::try_new("Schema")
            .unwrap()
            .with_entity(entity("Foo::Q", [event("event", [])]))
            .with_record(record("Foo::Q", []))
            .build();
        let Err(error) = result else {
            panic!("expected a duplicate path error");
        };
        assert_eq!(error, BuilderError::DuplicateName("Foo::Q".to_string()));
    }
}
