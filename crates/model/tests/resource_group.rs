// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for resource_group macro.

use quent_model::prelude::*;

#[derive(Entity)]
#[resource_group(root)]
pub struct Engine;

#[derive(Entity)]
#[resource_group]
pub struct QueryGroup;

#[test]
fn resource_group_trait_impl() {
    fn assert_rg<T: quent_model::ResourceGroup>() {}
    assert_rg::<Engine>();
    assert_rg::<QueryGroup>();
}

#[test]
fn root_resource_group() {
    assert!(Engine::IS_ROOT);
    assert!(!QueryGroup::IS_ROOT);
}

#[test]
fn resource_group_model_component() {
    let mut builder = ModelBuilder::new();
    Engine::collect(&mut builder);
    QueryGroup::collect(&mut builder);

    assert_eq!(builder.resource_groups.len(), 2);
    assert_eq!(builder.resource_groups[0].name, "engine");
    assert!(builder.resource_groups[0].is_root);
    assert_eq!(builder.resource_groups[1].name, "query_group");
    assert!(!builder.resource_groups[1].is_root);
}

#[test]
fn resource_group_has_event_type() {
    // Resource group entities with no explicit events should still
    // generate HasEventType via the implicit declaration event.
    fn assert_has_event<T: quent_model::HasEventType>() {}
    assert_has_event::<Engine>();
    assert_has_event::<QueryGroup>();
}

#[test]
fn resource_group_entity_data() {
    // Verify EntityData is generated for resource group entities.
    fn assert_entity_data<T: quent_model::EntityData>() {}
    assert_entity_data::<Engine>();
    assert_entity_data::<QueryGroup>();
}
