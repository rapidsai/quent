// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Types shared with the UI.

use quent_analyzer::EntityId;
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

/// A reference to an entity
#[derive(TS, Debug, Serialize)]
pub enum EntityRef {
    Engine(Uuid),
    Worker(Uuid),
    QueryGroup(Uuid),
    Query(Uuid),
    Plan(Uuid),
    Operator(Uuid),
    Port(Uuid),
    Resource(Uuid),
    ResourceGroup(Uuid),
    Task(Uuid),
}

impl EntityId for EntityRef {
    fn is_resource(&self) -> bool {
        matches!(self, EntityRef::Resource(_))
    }
    fn is_resource_group(&self) -> bool {
        matches!(self, EntityRef::ResourceGroup(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_ref_resource_predicates_match_only_resource_variants() {
        let id = Uuid::now_v7();
        let cases = [
            (EntityRef::Engine(id), false, false),
            (EntityRef::Worker(id), false, false),
            (EntityRef::QueryGroup(id), false, false),
            (EntityRef::Query(id), false, false),
            (EntityRef::Plan(id), false, false),
            (EntityRef::Operator(id), false, false),
            (EntityRef::Port(id), false, false),
            (EntityRef::Resource(id), true, false),
            (EntityRef::ResourceGroup(id), false, true),
            (EntityRef::Task(id), false, false),
        ];

        for (entity, is_resource, is_resource_group) in cases {
            assert_eq!(entity.is_resource(), is_resource);
            assert_eq!(entity.is_resource_group(), is_resource_group);
        }
    }
}
