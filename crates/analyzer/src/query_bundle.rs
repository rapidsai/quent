use quent_time::{TimeSec, TimeUnixNanoSec, to_secs};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    Result,
    entities::{Entities, EntitiesUI},
    plan_tree::PlanTree,
    resource_tree::ResourceTree,
};

#[derive(TS, Debug, Serialize)]
pub struct QueryBundle {
    entities: EntitiesUI,

    plan_tree: PlanTree,
    resource_tree: ResourceTree,

    unique_operator_names: Vec<String>,
    unique_entity_names: Vec<String>,

    start_time_unix_ns: TimeUnixNanoSec,
    duration_s: TimeSec,
}

impl QueryBundle {
    pub fn try_new(entities: &Entities, query_id: Uuid) -> Result<Self> {
        let entities = entities.try_filter_by_query(query_id)?;
        let plan_tree = PlanTree::try_new(&entities, query_id)?;
        let resource_tree = ResourceTree::try_new(&entities, &plan_tree, query_id)?;

        let unique_entity_names = entities.unique_entity_type_names().collect();
        let unique_operator_names = entities.unique_operator_names().map(Into::into).collect();

        let start_time_unix_ns = entities.span.start();
        let duration_s = to_secs(entities.span.duration());

        Ok(Self {
            entities: entities.into(),
            plan_tree,
            resource_tree,
            unique_entity_names,
            unique_operator_names,
            start_time_unix_ns,
            duration_s,
        })
    }
}
