// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::plan::{PlanEventStorage, PlanEvents};

use super::*;

pub type Plan = StoredEntity<schema::Plan, PlanEventStorage>;

impl RefTreeEntity for Plan {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.parent.query_id.target)
    }
}

impl PlanEntity for Plan {
    fn parent_query_id(&self) -> Option<Uuid> {
        self.parent_plan_id()
            .is_none()
            .then(|| self.declaration().data.parent.query_id.target)
    }

    fn parent_plan_id(&self) -> Option<Uuid> {
        self.declaration()
            .data
            .parent
            .plan_id
            .as_ref()
            .map(|plan| plan.target)
    }

    fn worker_id(&self) -> Option<Uuid> {
        self.declaration()
            .data
            .worker_id
            .as_ref()
            .map(|worker| worker.target)
    }

    fn edges(&self) -> impl Iterator<Item = (Uuid, Uuid)> + '_ {
        self.declaration()
            .data
            .edges
            .iter()
            .map(|edge| (edge.source.target, edge.target.target))
    }

    fn to_ui(&self) -> query_engine_ui::Plan {
        query_engine_ui::Plan {
            id: self.id(),
            instance_name: Some(self.declaration().data.instance_name.clone()),
            parent: self.parent_plan_id().or(self.parent_id()),
            worker_id: self.worker_id(),
            edges: self
                .edges()
                .map(|(source, target)| query_engine_ui::Edge { source, target })
                .collect(),
        }
    }
}
