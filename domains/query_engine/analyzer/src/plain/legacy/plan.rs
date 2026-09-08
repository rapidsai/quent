// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::entity::EntityEvents;
use quent_analyzer::{AnalyzerResult, Entity, resource::ResourceGroup};
use quent_events::Event;
use quent_query_engine_model::plan::{self, Edge, PlanParent};
use quent_query_engine_ui as ui;
use uuid::Uuid;

use crate::PlanEntity;

/// An event-backed plan DAG.
#[derive(Debug)]
pub struct Plan(EntityEvents<plan::Plan>);

impl Plan {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self(EntityEvents::new(id)?))
    }

    pub fn push(&mut self, event: Event<plan::PlanEvent>) {
        self.0.push(event);
    }
}

impl PlanEntity for Plan {
    fn parent(&self) -> Option<&PlanParent> {
        self.0.data().declaration.as_ref().map(|d| &d.parent)
    }

    fn worker_id(&self) -> Option<Uuid> {
        self.0
            .data()
            .declaration
            .as_ref()
            .and_then(|d| d.worker_id.map(|r| r.uuid()))
    }

    fn edges(&self) -> &[Edge] {
        self.0
            .data()
            .declaration
            .as_ref()
            .map(|d| d.edges.as_slice())
            .unwrap_or_default()
    }

    fn to_ui(&self) -> ui::Plan {
        let parent = self.parent().map(|p| {
            p.query_id
                .map(|r| r.uuid())
                .or(p.plan_id.map(|r| r.uuid()))
                .unwrap_or_default()
        });

        ui::Plan {
            id: self.0.id(),
            instance_name: self
                .0
                .data()
                .declaration
                .as_ref()
                .map(|d| d.instance_name.clone()),
            parent,
            worker_id: self.worker_id(),
            edges: self
                .edges()
                .iter()
                .map(|e| ui::Edge {
                    source: e.source.uuid(),
                    target: e.target.uuid(),
                })
                .collect(),
        }
    }
}

impl Entity for Plan {
    fn id(&self) -> Uuid {
        self.0.id()
    }

    fn type_name(&self) -> &str {
        "plan"
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

impl ResourceGroup for Plan {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.worker_id().or(self.parent().and_then(|parent| {
            parent
                .query_id
                .map(|r| r.uuid())
                .or(parent.plan_id.map(|r| r.uuid()))
        }))
    }
}
