// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for resource_group macro.

use quent_model::prelude::*;

#[quent_model::resource_group]
pub struct Engine {
    pub name: String,
}

#[quent_model::resource_group(parent = Engine)]
pub struct QueryGroup {
    pub engine_id: Uuid,
}

#[test]
fn resource_group_trait_impl() {
    fn assert_rg<T: ResourceGroup>() {}
    assert_rg::<Engine>();
    assert_rg::<QueryGroup>();
}

#[test]
fn resource_group_model_component() {
    let mut builder = ModelBuilder::new();
    Engine::collect(&mut builder);
    QueryGroup::collect(&mut builder);

    assert_eq!(builder.resource_groups.len(), 2);
    assert_eq!(builder.resource_groups[0].name, "engine");
    assert!(builder.resource_groups[0].fixed_parent.is_none());
    assert_eq!(builder.resource_groups[1].name, "query_group");
    assert_eq!(
        builder.resource_groups[1].fixed_parent.as_deref(),
        Some("engine")
    );
}
