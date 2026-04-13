// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for entity and event macros.

use quent_model::{Attributes, ModelBuilder, ModelComponent};
use uuid::Uuid;

quent_model::entity! {
    Operator {
        events: {},
    }
}

#[derive(Debug, Attributes)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlanCreated {
    pub plan_id: Uuid,
    pub query_text: String,
}

#[derive(Debug, Attributes)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Stats {
    pub rows: u64,
}

quent_model::entity! {
    Worker {
        events: {
            plan_created: PlanCreated,
            stats: Stats,
        },
    }
}

#[test]
fn entity_trait_impl() {
    fn assert_entity<T: quent_model::Entity>() {}
    assert_entity::<Operator>();
}

#[test]
fn unit_entity_has_no_events() {
    let mut builder = ModelBuilder::new("test");
    Operator::collect(&mut builder);

    assert_eq!(builder.entities.len(), 1);
    assert_eq!(builder.entities[0].name, "operator");
    assert_eq!(builder.entities[0].events.len(), 0);
}

#[test]
fn entity_event_attributes_populated() {
    let mut builder = ModelBuilder::new("test");
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

// Self-event entity: struct IS the event
quent_model::entity! {
    Alert {
        attributes: {
            severity: u32,
            message: String,
        },
    }
}

#[test]
fn self_event_entity() {
    let mut builder = ModelBuilder::new("test");
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
