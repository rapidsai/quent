// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for entity and event macros.

use quent_model::prelude::*;

#[quent_model::entity]
pub struct Operator {
    pub plan_id: Uuid,
    pub type_name: String,
}

#[quent_model::event(entity = Operator)]
pub struct OperatorStatistics {
    pub rows_processed: u64,
    pub bytes_read: u64,
}

#[test]
fn entity_trait_impl() {
    fn assert_entity<T: Entity>() {}
    assert_entity::<Operator>();
}

#[test]
fn entity_event_trait_impl() {
    fn assert_entity_event<T: EntityEvent>() {}
    assert_entity_event::<OperatorStatistics>();
}

#[test]
fn entity_model_component() {
    let mut builder = ModelBuilder::new();
    Operator::collect(&mut builder);

    assert_eq!(builder.entities.len(), 1);
    assert_eq!(builder.entities[0].name, "operator");
    assert_eq!(builder.entities[0].attributes.len(), 2);
}
