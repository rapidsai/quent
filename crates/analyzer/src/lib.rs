//! Analyzes raw events to produce useful performance insights.

use quent_entities::{
    engine::Engine,
    query_group::QueryGroup,
    timeline::{ResourceTimeline, ResourceTimelineBinned, ResourceTimelineBinnedByState},
    worker::Worker,
};
use quent_events::{Event as RawEvent, EventData};
use quent_time::{Span, bin::BinnedSpan};
use uuid::Uuid;

use crate::{
    entities::Entities,
    query::QueryBundle,
    timeline::{
        make_resource_timeline_bin_aggregated, make_resource_timeline_for_resource,
        make_resource_timeline_state_and_bin_aggregated,
    },
};

pub mod entities;
pub mod error;
pub mod plan_tree;
pub mod query;
pub mod resource_tree;
pub mod timeline;

pub type Result<T> = std::result::Result<T, error::Error>;

pub type Event = RawEvent<EventData>;

// TODO(johanpel): make it fast
#[derive(Debug)]
pub struct Analyzer {
    entities: Entities,
}

impl Analyzer {
    pub fn try_new(engine_id: Uuid, events: impl Iterator<Item = Event>) -> Result<Self> {
        // Process all events into entities, flattened into maps to allow looking them up by ID.
        let entities = Entities::try_new(engine_id, events)?;
        Ok(Self { entities })
    }

    pub fn engine(&self) -> &Engine {
        &self.entities.engine
    }

    /// Return a Span that spans all event timestamps.
    pub fn timestamp_span(&self) -> Span {
        // TODO(johanpel): calculate this as entities are constructed
        Span::try_new(
            self.engine().timestamps.init.unwrap(),
            self.engine().timestamps.exit.unwrap(),
        )
        .unwrap()
    }

    // TODO(johanpel): this is separated from an engine, since we assume engines can have
    //                 immense lifetimes so they could be running lots of query_groups, in
    //                 which case we may want to implement pagination for this.
    pub fn worker_ids(&self) -> Vec<Uuid> {
        self.entities.workers.keys().cloned().collect()
    }
    pub fn worker(&self, id: Uuid) -> Option<&Worker> {
        self.entities.workers.get(&id)
    }
    // TODO(johanpel): pagination
    pub fn query_group_ids(&self) -> Vec<Uuid> {
        self.entities.query_groups.keys().cloned().collect()
    }
    pub fn query_group(&self, id: Uuid) -> Option<&QueryGroup> {
        self.entities.query_groups.get(&id)
    }
    // TODO(johanpel): pagination
    pub fn query_ids(&self, query_group_id: Uuid) -> Vec<Uuid> {
        self.entities
            .queries
            .iter()
            .filter_map(|(k, v)| (v.query_group_id == query_group_id).then_some(*k))
            .collect()
    }

    #[tracing::instrument(skip(self), err)]
    pub fn query_bundle(&self, id: Uuid) -> Result<QueryBundle> {
        QueryBundle::try_new(&self.entities, id)
    }

    #[tracing::instrument(skip(self), err)]
    pub fn resource_usage_spans(&self, resource_id: Uuid) -> Result<ResourceTimeline> {
        make_resource_timeline_for_resource(&self.entities, resource_id)
    }

    #[tracing::instrument(skip(self), err)]
    pub fn resource_usage_aggregated(
        &self,
        resource_id: Uuid,
        config: BinnedSpan,
    ) -> Result<ResourceTimelineBinned> {
        make_resource_timeline_bin_aggregated(&self.entities, resource_id, config)
    }

    #[tracing::instrument(skip(self), err)]
    pub fn resource_usage_states_aggregated(
        &self,
        resource_id: Uuid,
        config: BinnedSpan,
        fsm_type_name: String,
    ) -> Result<ResourceTimelineBinnedByState> {
        make_resource_timeline_state_and_bin_aggregated(
            &self.entities,
            resource_id,
            config,
            fsm_type_name,
        )
    }
}
