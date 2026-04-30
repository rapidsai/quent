// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use rustc_hash::FxHashMap as HashMap;

use quent_analyzer::{AnalyzerError, AnalyzerResult, Entity};
use quent_query_engine_ui as ui;
use uuid::Uuid;

use crate::plan::Plan;

/// A tree of [`Plan`]s.
///
/// [`Plan`]s under a `Query` may form a tree, typically with a trunk and
/// potentially a single branching point when they are fanned out over workers
/// for distributed `Engine`s.
///
/// Under the `Query` there must be always one top-level [`Plan`] (the root of
/// the tree). For example, this could be what in some `Engine`s is called a
/// "logical" [`Plan`].
///
/// An `Engine` may "lower" a [`Plan`] to produce a derived [`Plan`], any
/// arbitrary number of times. At some point, at least one `Worker` will
/// execute a [`Plan`], but the model is flexible enough to allow a `Worker`
/// to locally "lower" the [`Plan`] further.
#[derive(Clone, Debug)]
pub struct PlanTree {
    /// The [`Plan`] ID.
    pub id: Uuid,
    /// The ID of the `Worker` this [`Plan`] is local to.
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
                p.parent()
                    .and_then(|parent| parent.plan_id)
                    .is_some_and(|r| r.uuid() == current_plan_id)
            })
            .map(|p| Self::build(p.id(), plans))
            .collect::<AnalyzerResult<Vec<_>>>()?;

        if children.is_empty() && plan.worker_id().is_none() {
            return Err(AnalyzerError::Validation(format!(
                "leaf plan {current_plan_id} must have a worker_id"
            )));
        }

        Ok(PlanTree {
            id: current_plan_id,
            worker: plan.worker_id(),
            children,
        })
    }

    pub fn try_new<'a>(
        plans: impl Iterator<Item = &'a Plan>,
        query_id: Uuid,
    ) -> AnalyzerResult<Self> {
        let plans: HashMap<Uuid, &Plan> = plans.map(|p| (p.id(), p)).collect();

        let root_plans: Vec<_> = plans
            .values()
            .filter(|p: &&&Plan| {
                p.parent()
                    .and_then(|parent| parent.query_id)
                    .is_some_and(|r| r.uuid() == query_id)
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
                root_plans.iter().map(|p| p.id()).collect::<Vec<_>>()
            )));
        }

        Self::build(root_plans[0].id(), &plans)
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
    use quent_events::Event;
    use quent_model::Ref;
    use quent_query_engine_model::plan::{Declaration, PlanEvent, PlanParent};
    use quent_time::TimeUnixNanoSec;

    use super::*;

    fn make_plan(id: Uuid, parent: PlanParent, worker_id: Option<Uuid>) -> Plan {
        let mut plan = Plan::try_new(id).unwrap();
        plan.push(Event::new(
            id,
            TimeUnixNanoSec::default(),
            PlanEvent::Declaration(Declaration {
                parent,
                instance_name: String::new(),
                edges: Vec::new(),
                worker_id: worker_id.map(Ref::new),
            }),
        ));
        plan
    }

    // Create a tree with a tunk of two plans, then split out into 3
    // worker-local plans
    #[test]
    fn try_new() {
        let query_id = Uuid::now_v7();
        let trunk_ids = [Uuid::now_v7(), Uuid::now_v7()];
        let leaf_ids = [Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()];
        let worker_ids = [Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()];

        let mut plans = HashMap::default();

        plans.insert(
            trunk_ids[0],
            make_plan(
                trunk_ids[0],
                PlanParent {
                    query_id: Some(Ref::new(query_id)),
                    plan_id: None,
                },
                None,
            ),
        );
        plans.insert(
            trunk_ids[1],
            make_plan(
                trunk_ids[1],
                PlanParent {
                    query_id: None,
                    plan_id: Some(Ref::new(trunk_ids[0])),
                },
                None,
            ),
        );

        for i in 0..3 {
            plans.insert(
                leaf_ids[i],
                make_plan(
                    leaf_ids[i],
                    PlanParent {
                        query_id: None,
                        plan_id: Some(Ref::new(trunk_ids[1])),
                    },
                    Some(worker_ids[i]),
                ),
            );
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
        plans.insert(
            plan_id,
            make_plan(
                plan_id,
                PlanParent {
                    query_id: Some(Ref::new(query_id)),
                    plan_id: None,
                },
                None,
            ),
        );

        let result = PlanTree::try_new(plans.values(), query_id);

        assert!(matches!(result, Err(AnalyzerError::Validation(_))));
    }
}
