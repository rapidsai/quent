use py_rs::PY;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{Result, entities::Entities, error::Error};

/// A tree of Plans.
///
/// Plans under a Query can form a simple tree with a trunk and potentially a
/// single branching point when they are fanned out over workers for distributed
/// engines. Under the Queryy there must be always one top-level Plan (the root
/// of the tree). For example, this could be what in some engines is called a
/// "logical" plan. An engine may then "lower" a Plan to produce a derived Plan,
/// any arbitrary number of times. At some point, at least one worker will
/// execute a Plan, but the model is flexible enough to allow a worker to
/// locally lower the plan further.
#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct PlanTree {
    /// The Plan ID.
    pub id: Uuid,
    /// The ID of the Query this Plan is logically nested under.
    pub query: Option<Uuid>,
    /// The ID of the Worker this Plan is local to.
    pub worker: Option<Uuid>,
    /// The child plan. If this is an empty list, this is a leaf plan.
    pub children: Vec<PlanTree>,
}

impl PlanTree {
    /// Depth-first traverse the plan tree with a visitor function.
    pub(crate) fn visit_depth_first<F>(&self, visitor: &mut F) -> Result<()>
    where
        F: FnMut(&PlanTree) -> Result<()>,
    {
        visitor(self)?;
        for child in &self.children {
            child.visit_depth_first(visitor)?;
        }
        Ok(())
    }
}

impl PlanTree {
    pub(crate) fn try_new(entities: &Entities, query_id: Uuid) -> Result<Self> {
        fn build(current_plan_id: Uuid, entities: &Entities) -> Result<PlanTree> {
            let plan = entities
                .plans
                .get(&current_plan_id)
                .ok_or(Error::Logic("Plan does not exist.".to_string()))?;

            let children = entities
                .plans
                .values()
                .filter_map(|p| {
                    if p.parent_plan_id == Some(current_plan_id) {
                        Some(build(p.id, entities))
                    } else {
                        None
                    }
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(PlanTree {
                id: current_plan_id,
                query: Some(plan.query_id),
                worker: plan.worker_id,
                children,
            })
        }

        let root_plan = entities
            .plans
            .values()
            .find(|p| p.parent_plan_id.is_none() && p.query_id == query_id)
            .ok_or(Error::Logic(format!(
                "A root plan for query id {query_id} does not exist."
            )))?;

        build(root_plan.id, entities)
    }
}
