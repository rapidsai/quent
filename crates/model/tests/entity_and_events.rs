// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for entity and event macros.

use quent_model::prelude::*;
use quent_model::{ModelBuilder, ModelComponent};

#[derive(Entity)]
pub struct Operator;

#[derive(Debug, Event)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlanCreated {
    pub plan_id: Uuid,
    pub query_text: String,
}

#[derive(Debug, Event)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Stats {
    pub rows: u64,
}

#[derive(Entity)]
pub struct Worker {
    pub plan_created: EmitOnce<PlanCreated>,
    pub stats: EmitOnce<Stats>,
}

#[test]
fn entity_trait_impl() {
    fn assert_entity<T: quent_model::Entity>() {}
    assert_entity::<Operator>();
}

#[test]
fn unit_entity_has_no_events() {
    let mut builder = ModelBuilder::new();
    Operator::collect(&mut builder);

    assert_eq!(builder.entities.len(), 1);
    assert_eq!(builder.entities[0].name, "operator");
    assert_eq!(builder.entities[0].events.len(), 0);
}

#[test]
fn entity_event_attributes_populated() {
    let mut builder = ModelBuilder::new();
    Worker::collect(&mut builder);

    assert_eq!(builder.entities.len(), 1);
    let entity = &builder.entities[0];
    assert_eq!(entity.events.len(), 2);

    let plan_event = &entity.events[0];
    assert_eq!(plan_event.name, "plan_created");
    assert_eq!(plan_event.attributes.len(), 2);
    assert_eq!(plan_event.attributes[0].name, "plan_id");
    assert_eq!(plan_event.attributes[1].name, "query_text");

    let stats_event = &entity.events[1];
    assert_eq!(stats_event.name, "stats");
    assert_eq!(stats_event.attributes.len(), 1);
    assert_eq!(stats_event.attributes[0].name, "rows");
}

// Self-event entity: #[derive(Entity, Event)] — struct IS the event
#[derive(Debug, Entity, Event)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Alert {
    pub severity: u32,
    pub message: String,
}

#[test]
fn self_event_entity() {
    let mut builder = ModelBuilder::new();
    Alert::collect(&mut builder);

    assert_eq!(builder.entities.len(), 1);
    let entity = &builder.entities[0];
    assert_eq!(entity.name, "alert");
    assert_eq!(entity.events.len(), 1);
    assert_eq!(entity.events[0].name, "alert");
    assert_eq!(entity.events[0].attributes.len(), 2);
    assert_eq!(entity.events[0].attributes[0].name, "severity");
    assert_eq!(entity.events[0].attributes[1].name, "message");
}
