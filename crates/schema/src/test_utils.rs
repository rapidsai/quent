// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Helpers for constructing schema elements in tests.
//!
//! These helpers are opt-in through the `test-utils` feature.
//!
//! # Warning
//!
//! The functions in this module can panic and should only be used in tests.

use crate::builder::{EntityBuilder, EventBuilder, RecordBuilder, SchemaBuilder};
use crate::{
    Annotations, Cardinality, DataType, Entity, Event, Field, Identifier, Path, Record, Schema,
};

pub fn ident(s: &str) -> Identifier {
    Identifier::try_new(s).unwrap()
}
/// Parses a schema path, panicking if it is invalid.
pub fn path(s: &str) -> Path {
    s.parse().unwrap()
}
/// Constructs a record-reference data type, panicking if the path is invalid.
pub fn record_type(s: &str) -> DataType {
    DataType::Record(path(s))
}
pub fn field(name: &str, ty: DataType) -> Field {
    Field::new(ident(name), ty, Annotations::default())
}
pub fn event(name: &str, payload: impl IntoIterator<Item = Field>) -> Event {
    event_with(name, Cardinality::Once, payload)
}
pub fn event_with(
    name: &str,
    cardinality: Cardinality,
    payload: impl IntoIterator<Item = Field>,
) -> Event {
    EventBuilder::new(ident(name), cardinality)
        .with_fields(payload)
        .build()
        .unwrap()
}
pub fn entity(name: &str, events: impl IntoIterator<Item = Event>) -> Entity {
    EntityBuilder::new(path(name))
        .with_events(events)
        .build()
        .unwrap()
}
pub fn eventless_entity(name: &str) -> Entity {
    Entity::from_parts(path(name), Default::default(), Annotations::default())
}
pub fn record(name: &str, fields: impl IntoIterator<Item = Field>) -> Record {
    RecordBuilder::new(path(name))
        .with_fields(fields)
        .build()
        .unwrap()
}
pub fn schema(
    name: &str,
    entities: impl IntoIterator<Item = Entity>,
    records: impl IntoIterator<Item = Record>,
) -> Schema {
    SchemaBuilder::new(ident(name))
        .with_entities(entities)
        .with_records(records)
        .build()
        .unwrap()
}

/// Constructs a schema without applying [`SchemaBuilder`] validation.
pub fn unchecked_schema(
    name: &str,
    entities: impl IntoIterator<Item = Entity>,
    records: impl IntoIterator<Item = Record>,
) -> Schema {
    let entities = entities
        .into_iter()
        .map(|entity| (entity.path().clone(), entity))
        .collect();
    let records = records
        .into_iter()
        .map(|record| (record.path().clone(), record))
        .collect();
    Schema::from_parts(ident(name), entities, records, Annotations::default())
}
