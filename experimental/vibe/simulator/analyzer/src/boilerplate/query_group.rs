// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::query_group::{QueryGroupEventStorage, QueryGroupEvents};

use super::*;

pub type QueryGroup = StoredEntity<schema::QueryGroup, QueryGroupEventStorage>;

impl RefTreeEntity for QueryGroup {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.engine_id.target)
    }
}

impl QueryGroupEntity for QueryGroup {
    fn to_ui(&self) -> query_engine_ui::QueryGroup {
        query_engine_ui::QueryGroup {
            id: self.id(),
            instance_name: Some(self.declaration().data.instance_name.clone()),
            engine_id: self.parent_id(),
        }
    }
}
