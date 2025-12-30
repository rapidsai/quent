use py_rs::PY;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{Result, entities::Entities, plan_tree::PlanTree, resource_tree::ResourceTree};

#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct QueryBundle {
    entities: Entities,

    plan_tree: PlanTree,
    resource_tree: ResourceTree,
}

impl QueryBundle {
    pub fn try_new(entities: &Entities, query_id: Uuid) -> Result<Self> {
        let entities = entities.try_filter_by_query(query_id)?;
        let plan_tree = PlanTree::try_new(&entities, query_id)?;
        let resource_tree = ResourceTree::try_new(&entities, &plan_tree, query_id)?;
        Ok(Self {
            entities,
            plan_tree,
            resource_tree,
        })
    }
}
