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
use crate::{Annotations, Cardinality, DataType, Entity, Event, Field, Identifier, Record, Schema};

pub fn ident(s: &str) -> Identifier {
    Identifier::try_new(s).unwrap()
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
        .fields(payload)
        .unwrap()
        .build()
}
pub fn entity(name: &str, events: impl IntoIterator<Item = Event>) -> Entity {
    EntityBuilder::new(ident(name))
        .events(events)
        .unwrap()
        .build()
}
pub fn record(name: &str, fields: impl IntoIterator<Item = Field>) -> Record {
    RecordBuilder::new(ident(name))
        .fields(fields)
        .unwrap()
        .build()
}
pub fn schema(
    name: &str,
    entities: impl IntoIterator<Item = Entity>,
    records: impl IntoIterator<Item = Record>,
) -> Schema {
    SchemaBuilder::new(ident(name))
        .entities(entities)
        .unwrap()
        .records(records)
        .unwrap()
        .build()
}
