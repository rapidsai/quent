use quent_entities::Span;
use quent_time::{TimeSec, TimeUnixNanoSec, to_secs};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AnalyzerResult,
    entities::{Entities, EntitiesUI},
    error::AnalyzerError,
    plan_tree::PlanTree,
    resource_tree::ResourceTree,
};

#[derive(TS, Debug, Serialize)]
pub struct QueryBundle {
    /// The ID of the query.
    query_id: Uuid,
    /// Maps with entities that are involved in this query.
    entities: EntitiesUI,

    /// A tree of plans involved in the execution of this query.
    plan_tree: PlanTree,
    /// A tree of resources involved in the execution of this query.
    resource_tree: ResourceTree,

    /// A list of unique operator type names.
    unique_operator_names: Vec<String>,
    /// A list of unique entity type names.
    unique_entity_names: Vec<String>,

    /// The number of nanoseconds passed since the Unix epoch at which the
    /// engine started executing this query.
    start_time_unix_ns: TimeUnixNanoSec,
    /// The duration of this query, in seconds.
    duration_s: TimeSec,
}

impl QueryBundle {
    pub fn try_new(entities: &Entities, query_id: Uuid) -> AnalyzerResult<Self> {
        let entities = entities.try_filter_by_query(query_id)?;
        // Sanity check:
        assert_eq!(entities.queries.len(), 1);

        let plan_tree = PlanTree::try_new(&entities, query_id)?;
        let resource_tree = ResourceTree::try_new(&entities, entities.resource_root)?;

        let unique_entity_names = entities.entity_type_names().collect();
        let unique_operator_names = entities.operator_type_names().map(Into::into).collect();

        let query_span = entities
            .queries
            .get(&query_id)
            .ok_or(AnalyzerError::BrokenImpl(format!(
                "unexpected missing query {query_id}"
            )))?
            .span()?;
        let start_time_unix_ns = query_span.start();
        let duration_s = to_secs(query_span.duration());

        Ok(Self {
            query_id,
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
