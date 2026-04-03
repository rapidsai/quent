// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{Entity, resource::ResourceGroup};
use quent_events::Event;
use quent_query_engine_events::plan::{Edge, PlanEvent, PlanParent};
use quent_query_engine_ui as ui;
use uuid::Uuid;

pub mod tree;

/// A Directed-Acyclic-Graph of [`Operator`]s and [`Edge`]s.
///
/// Represents the dataflow starting at data sources, through operators
/// performing transformations, to an output.
#[derive(Debug)]
pub struct Plan {
    /// The ID of this [`Plan`].
    pub id: Uuid,
    /// The name of this [`Plan`].
    pub instance_name: Option<String>,
    /// The ID of the parent of this [`Plan`].
    pub parent: Option<PlanParent>,
    /// The ID of the [`super::worker::Worker`] that executed this [`Plan`].
    ///
    /// If this level of [`Plan`] was not directly executed by a [`Worker`],
    /// then this may be set to None.
    pub worker_id: Option<Uuid>,
    /// The [`Edge`]s between [`Operator`]s of this [`Plan`].
    pub edges: Vec<Edge>,
}

impl Plan {
    pub fn try_new(id: Uuid) -> quent_analyzer::AnalyzerResult<Self> {
        if id.is_nil() {
            Err(quent_analyzer::AnalyzerError::Validation(
                "plan id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                instance_name: None,
                parent: None,
                worker_id: None,
                edges: Vec::new(),
            })
        }
    }

    pub fn push(&mut self, event: Event<PlanEvent>) {
        self.edges = event.data.edges;
        self.worker_id = event.data.worker_id;
        self.parent = Some(event.data.parent);
        self.instance_name = Some(event.data.instance_name);
    }

    pub fn to_ui(&self) -> ui::Plan {
        let parent = self.parent.as_ref().map(|p| match p {
            PlanParent::Query(uuid) => *uuid,
            PlanParent::Plan(uuid) => *uuid,
        });

        ui::Plan {
            id: self.id,
            instance_name: self.instance_name.clone(),
            parent,
            worker_id: self.worker_id,
            edges: self
                .edges
                .iter()
                .map(|e| ui::Edge {
                    source: e.source,
                    target: e.target,
                })
                .collect(),
        }
    }
}

impl Entity for Plan {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "plan"
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_deref().unwrap_or_default()
    }
}

impl ResourceGroup for Plan {
    fn parent_group_id(&self) -> Option<Uuid> {
        // If this is a plan associated with a worker, we consider this plan to
        // be a resource group under the worker resource group
        self.worker_id
            .or(self.parent.as_ref().map(|parent| *match parent {
                PlanParent::Query(uuid) => uuid,
                PlanParent::Plan(uuid) => uuid,
            }))
    }
}
