// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::entity::EntityEvents;
use quent_analyzer::{AnalyzerResult, Entity, resource::ResourceGroup};
use quent_events::Event;
use quent_query_engine_model::query_group;
use quent_query_engine_ui as ui;
use uuid::Uuid;

/// A QueryGroup is an entity that groups [`super::query::Query`]s
#[derive(Debug)]
pub struct QueryGroup(EntityEvents<query_group::QueryGroup>);

impl QueryGroup {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self(EntityEvents::new(id)?))
    }

    pub fn push(&mut self, event: Event<query_group::QueryGroupEvent>) {
        self.0.push(event);
    }

    pub fn to_ui(&self) -> ui::QueryGroup {
        let d = self.0.data();
        ui::QueryGroup {
            id: self.0.id(),
            instance_name: d.declaration.as_ref().map(|d| d.instance_name.clone()),
            engine_id: d.declaration.as_ref().map(|d| d.engine_id),
        }
    }
}

impl Entity for QueryGroup {
    fn id(&self) -> Uuid {
        self.0.id()
    }
    fn type_name(&self) -> &str {
        "query group"
    }
    fn instance_name(&self) -> &str {
        self.0
            .data()
            .declaration
            .as_ref()
            .map(|d| d.instance_name.as_str())
            .unwrap_or_default()
    }
}

impl ResourceGroup for QueryGroup {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.0.data().declaration.as_ref().map(|d| d.engine_id)
    }
}
