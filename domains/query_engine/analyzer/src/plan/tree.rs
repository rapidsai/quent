// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use rustc_hash::FxHashMap as HashMap;

use quent_analyzer::{AnalyzerError, AnalyzerResult};
use quent_query_engine_events::plan::PlanParent;
use quent_query_engine_ui as ui;
use uuid::Uuid;

use crate::plan::Plan;

/// A tree of [`Plan`]s.
///
/// [`Plan`]s under a [`Query`] may form a tree, typically with a trunk and
/// potentially a single branching point when they are fanned out over workers
/// for distributed [`Engine`]s.
///
/// Under the [`Query`] there must be always one top-level [`Plan`] (the root of
/// the tree). For example, this could be what in some [`Engine`]s is called a
/// "logical" [`Plan`].
///
/// An [`Engine`] may "lower" a [`Plan`] to produce a derived [`Plan`], any
/// arbitrary number of times. At some point, at least one [`Worker`] will
/// execute a [`Plan`], but the model is flexible enough to allow a [`Worker`]
/// to locally "lower" the [`Plan`] further.
#[derive(Clone, Debug)]
pub struct PlanTree {
    /// The [`Plan`] ID.
    pub id: Uuid,
    /// The ID of the [`Worker`] this [`Plan`] is local to.
    pub worker: Option<Uuid>,
    /// The child [`Plan`]. If this is an empty list, this is a leaf [`Plan`].
    pub children: Vec<PlanTree>,
}

impl PlanTree {
    fn build(current_plan_id: Uuid, plans: &HashMap<Uuid, &Plan>) -> AnalyzerResult<PlanTree> {
        let plan = plans
            .get(&current_plan_id)
            .ok_or(AnalyzerError::InvalidId(current_plan_id))?;

        let children = plans
            .values()
            .filter(|p| {
                p.parent
                    .as_ref()
                    .map(|parent| match parent {
                        PlanParent::Plan(uuid) => *uuid == current_plan_id,
                        PlanParent::Query(_) => false,
                    })
                    .unwrap_or(false)
            })
            .map(|p| Self::build(p.id, plans))
            .collect::<AnalyzerResult<Vec<_>>>()?;

        if children.is_empty() && plan.worker_id.is_none() {
            return Err(AnalyzerError::Validation(format!(
                "leaf plan {current_plan_id} must have a worker_id"
            )));
        }

        Ok(PlanTree {
            id: current_plan_id,
            worker: plan.worker_id,
            children,
        })
    }

    pub fn try_new<'a>(
        plans: impl Iterator<Item = &'a Plan>,
        query_id: Uuid,
    ) -> AnalyzerResult<Self> {
        let plans: HashMap<Uuid, &Plan> = plans.map(|p| (p.id, p)).collect();

        let root_plans: Vec<_> = plans
            .values()
            .filter(|p| {
                p.parent
                    .as_ref()
                    .map(|parent| match parent {
                        PlanParent::Query(uuid) => *uuid == query_id,
                        PlanParent::Plan(_) => false,
                    })
                    .unwrap_or(false)
            })
            .collect();

        if root_plans.is_empty() {
            return Err(AnalyzerError::Validation(format!(
                "no root plan found for query {query_id}"
            )));
        }

        if root_plans.len() > 1 {
            return Err(AnalyzerError::Validation(format!(
                "query {} has {} root plans (expected 1): {:?}",
                query_id,
                root_plans.len(),
                root_plans.iter().map(|p| p.id).collect::<Vec<_>>()
            )));
        }

        Self::build(root_plans[0].id, &plans)
    }

    pub fn to_ui(&self) -> ui::PlanTree {
        ui::PlanTree {
            id: self.id,
            worker: self.worker,
            children: self.children.iter().map(|c| c.to_ui()).collect(),
        }
    }

    /// Return an iterator over [`PlanTree`] nodes in depth-first pre-order.
    pub fn iter(&self) -> PlanTreeIter<'_> {
        PlanTreeIter { stack: vec![self] }
    }
}

pub struct PlanTreeIter<'a> {
    stack: Vec<&'a PlanTree>,
}

impl<'a> Iterator for PlanTreeIter<'a> {
    type Item = &'a PlanTree;
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        self.stack.extend(node.children.iter().rev());
        Some(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Create a tree with a tunk of two plans, then split out into 3
    // worker-local plans
    #[test]
    fn try_new() {
        let query_id = Uuid::now_v7();
        let trunk_ids = [Uuid::now_v7(), Uuid::now_v7()];
        let leaf_ids = [Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()];
        let worker_ids = [Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()];

        let mut plans = HashMap::default();

        let mut trunk0 = Plan::try_new(trunk_ids[0]).unwrap();
        trunk0.parent = Some(PlanParent::Query(query_id));
        plans.insert(trunk_ids[0], trunk0);

        let mut trunk1 = Plan::try_new(trunk_ids[1]).unwrap();
        trunk1.parent = Some(PlanParent::Plan(trunk_ids[0]));
        plans.insert(trunk_ids[1], trunk1);

        for i in 0..3 {
            let mut leaf = Plan::try_new(leaf_ids[i]).unwrap();
            leaf.parent = Some(PlanParent::Plan(trunk_ids[1]));
            leaf.worker_id = Some(worker_ids[i]);
            plans.insert(leaf_ids[i], leaf);
        }

        let tree = PlanTree::try_new(plans.values(), query_id).unwrap();

        assert_eq!(tree.id, trunk_ids[0]);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].id, trunk_ids[1]);
        assert_eq!(tree.children[0].children.len(), 3);

        let tree_leaf_ids: Vec<Uuid> = tree.children[0].children.iter().map(|c| c.id).collect();
        for leaf_id in &leaf_ids {
            assert!(tree_leaf_ids.contains(leaf_id));
        }

        for leaf in &tree.children[0].children {
            assert!(leaf.worker.is_some());
            assert_eq!(leaf.children.len(), 0);
        }
    }

    // Leaf plans must have a worker id.
    #[test]
    fn try_new_leaf_no_worker() {
        let query_id = Uuid::now_v7();
        let plan_id = Uuid::now_v7();

        let mut plans = HashMap::default();
        let mut plan = Plan::try_new(plan_id).unwrap();
        plan.parent = Some(PlanParent::Query(query_id));
        plans.insert(plan_id, plan);

        let result = PlanTree::try_new(plans.values(), query_id);

        assert!(matches!(result, Err(AnalyzerError::Validation(_))));
    }
}
