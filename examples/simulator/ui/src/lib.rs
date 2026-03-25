// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Types shared with the UI.

use quent_analyzer::EntityId;
use serde::{Deserialize, Serialize};
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
        !matches!(self, EntityRef::Resource(_))
    }
    fn is_resource_group(&self) -> bool {
        !matches!(self, EntityRef::Task(_))
    }
}

#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub struct TaskFilter {
    pub operator_id: Option<Uuid>,
}

#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub query_id: Uuid,
}
