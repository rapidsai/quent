//! Analyzes raw events to produce useful performance insights.
//!
//! General analyzer TODOs for post-PoC:
//!
//! - Arrow-fication of the data. Right now, everything is deserialized into
//!   Rust native types. It's subjectively easier for now to capture modeling
//!   rules but when queries become more complicated, more run-time defined and
//!   interactive, it's most likely best to move this to a query engine in order
//!   to get better performance and scalability without too much engineering
//!   investment. Prior art used DataFusion.
//!
//! - Timeseries databases like InfluxDB have the ability to do various things
//!   like time binned aggregations etc. as well. How modeling rules and
//!   validation can be expressed in such frameworks is to be investigated.
//!

use quent_entities::{
    engine::Engine,
    query_group::QueryGroup,
    timeline::{ResourceTimelineBinned, ResourceTimelineBinnedByState},
    worker::Worker,
};
use quent_events::{Event as RawEvent, EventData};
use quent_time::{SpanNanoSec, bin::BinnedSpan};
use uuid::Uuid;

use crate::{
    entities::Entities,
    query_bundle::QueryBundle,
    timeline::{
        make_resource_group_timeline_bin_aggregated,
        make_resource_group_timeline_state_and_bin_aggregated,
        make_resource_timeline_bin_aggregated, make_resource_timeline_state_and_bin_aggregated,
    },
};

pub mod entities;
pub mod error;
pub mod plan_tree;
pub mod query_bundle;
pub mod resource_tree;
pub mod timeline;

pub type AnalyzerResult<T> = std::result::Result<T, error::AnalyzerError>;

pub type Event = RawEvent<EventData>;

// TODO(johanpel): make it fast
#[derive(Debug)]
pub struct Analyzer {
    entities: Entities,
}

impl Analyzer {
    pub fn try_new(engine_id: Uuid, events: impl Iterator<Item = Event>) -> AnalyzerResult<Self> {
        // Process all events into entities, flattened into maps to allow looking them up by ID.
        let entities = Entities::try_new(engine_id, events)?;
        Ok(Self { entities })
    }

    pub fn engine(&self) -> &Engine {
        &self.entities.engine
    }

    /// Return a Span that spans all event timestamps.
    pub fn timestamp_span(&self) -> SpanNanoSec {
        // TODO(johanpel): calculate this as entities are constructed
        SpanNanoSec::try_new(
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

    pub fn entities(&self) -> &Entities {
        &self.entities
    }

    pub fn query_bundle(&self, id: Uuid) -> AnalyzerResult<QueryBundle> {
        QueryBundle::try_new(&self.entities, id)
    }

    pub fn resource_usage_aggregated(
        &self,
        resource_id: Uuid,
        config: BinnedSpan,
    ) -> AnalyzerResult<ResourceTimelineBinned> {
        make_resource_timeline_bin_aggregated(&self.entities, resource_id, config)
    }

    pub fn resource_usage_states_aggregated(
        &self,
        resource_id: Uuid,
        fsm_type_name: &str,
        config: BinnedSpan,
    ) -> AnalyzerResult<ResourceTimelineBinnedByState> {
        make_resource_timeline_state_and_bin_aggregated(
            &self.entities,
            resource_id,
            fsm_type_name,
            config,
        )
    }

    pub fn resource_group_usage_aggregated(
        &self,
        resource_group_id: Uuid,
        resource_type_name: &str,
        config: BinnedSpan,
    ) -> AnalyzerResult<ResourceTimelineBinned> {
        make_resource_group_timeline_bin_aggregated(
            &self.entities,
            resource_group_id,
            resource_type_name,
            config,
        )
    }

    pub fn resource_group_usage_states_aggregated(
        &self,
        resource_id: Uuid,
        resource_type_name: &str,
        fsm_type_name: &str,
        config: BinnedSpan,
    ) -> AnalyzerResult<ResourceTimelineBinnedByState> {
        make_resource_group_timeline_state_and_bin_aggregated(
            &self.entities,
            resource_id,
            resource_type_name,
            fsm_type_name,
            config,
        )
    }
}
