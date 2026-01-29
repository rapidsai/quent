use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::{AnalyzerResult, entities::Entities, error::AnalyzerError};

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
#[derive(TS, Clone, Debug, Serialize)]
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
    pub(crate) fn try_new(entities: &Entities, query_id: Uuid) -> AnalyzerResult<Self> {
        fn build(current_plan_id: Uuid, entities: &Entities) -> AnalyzerResult<PlanTree> {
            let plan = entities
                .plans
                .get(&current_plan_id)
                .ok_or(AnalyzerError::Logic("Plan does not exist.".to_string()))?;

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
                .collect::<AnalyzerResult<Vec<_>>>()?;

            Ok(PlanTree {
                id: current_plan_id,
                query: plan.query_id,
                worker: plan.worker_id,
                children,
            })
        }

        let root_plan = entities
            .plans
            .values()
            .find(|p| p.parent_plan_id.is_none() && p.query_id == Some(query_id))
            .ok_or(AnalyzerError::Logic(format!(
                "A root plan for query id {query_id} does not exist."
            )))?;

        build(root_plan.id, entities)
    }
}
