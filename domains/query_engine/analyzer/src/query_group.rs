// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{Entity, resource::ResourceGroup};
use quent_events::Event;
use quent_query_engine_events::query_group::QueryGroupEvent;
use quent_query_engine_ui as ui;
use uuid::Uuid;

/// A QueryGroup is an entity that groups [`super::query::Query`]s
#[derive(Debug)]
pub struct QueryGroup {
    /// The ID of this [`QueryGroup`].
    pub id: Uuid,
    /// The ID of the Engine this QueryGroup was spawned in
    pub engine_id: Option<Uuid>,
    /// The name of this [`QueryGroup`].
    pub instance_name: Option<String>,
}

impl QueryGroup {
    pub fn try_new(id: Uuid) -> quent_analyzer::AnalyzerResult<Self> {
        if id.is_nil() {
            Err(quent_analyzer::AnalyzerError::Validation(
                "query group id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                engine_id: None,
                instance_name: None,
            })
        }
    }

    pub fn push(&mut self, event: Event<QueryGroupEvent>) {
        self.engine_id = Some(event.data.engine_id);
        self.instance_name = Some(event.data.instance_name);
    }

    pub fn to_ui(&self) -> ui::QueryGroup {
        ui::QueryGroup {
            id: self.id(),
            instance_name: self.instance_name.clone(),
            engine_id: self.engine_id,
        }
    }
}

impl Entity for QueryGroup {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "query group"
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_deref().unwrap_or_default()
    }
}

impl ResourceGroup for QueryGroup {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.engine_id
    }
}
