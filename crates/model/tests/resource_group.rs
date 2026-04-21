// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for resource_group macro.

use quent_model::{EntityData, HasEventType, ModelBuilder, ModelComponent, ResourceGroup};

quent_model::entity! {
    Engine: ResourceGroup<Root = true> {}
}

quent_model::entity! {
    QueryGroup: ResourceGroup {}
}

#[test]
fn resource_group_trait_impl() {
    fn assert_rg<T: ResourceGroup>() {}
    assert_rg::<Engine>();
    assert_rg::<QueryGroup>();
}

#[test]
fn root_resource_group() {
    const { assert!(Engine::IS_ROOT) };
    const { assert!(!QueryGroup::IS_ROOT) };
}

#[test]
fn resource_group_model_component() {
    let mut builder = ModelBuilder::new("test");
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
    fn assert_has_event<T: HasEventType>() {}
    assert_has_event::<Engine>();
    assert_has_event::<QueryGroup>();
}

#[test]
fn resource_group_entity_data() {
    // Verify EntityData is generated for resource group entities.
    fn assert_entity_data<T: EntityData>() {}
    assert_entity_data::<Engine>();
    assert_entity_data::<QueryGroup>();
}

// Resource group with custom declaration fields via attributes
#[derive(Debug, quent_model::Attributes)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ServerDetails {
    pub version: String,
    pub port: u32,
}

quent_model::entity! {
    Server: ResourceGroup<Root = true> {
        attributes: {
            version: String,
            port: u32,
        },
    }
}

#[test]
fn resource_group_custom_declaration_fields() {
    let mut builder = ModelBuilder::new("test");
    Server::collect(&mut builder);

    assert_eq!(builder.entities.len(), 1);
    let entity = &builder.entities[0];
    assert_eq!(entity.events.len(), 1);

    let decl = &entity.events[0];
    assert_eq!(decl.name, "server_declaration");
    // instance_name + version + port
    assert_eq!(decl.attributes.len(), 3);
    assert_eq!(decl.attributes[0].name, "instance_name");
    assert_eq!(decl.attributes[1].name, "version");
    assert_eq!(decl.attributes[2].name, "port");
}
