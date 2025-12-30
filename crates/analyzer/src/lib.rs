//! Analyzes raw events to produce useful performance insights.

use quent_entities::{engine::Engine, query_group::QueryGroup, worker::Worker};
use quent_events::{Event as RawEvent, EventData};
use uuid::Uuid;

use crate::{entities::Entities, query::QueryBundle};

pub mod entities;
pub mod error;
pub mod plan_tree;
pub mod query;
pub mod resource_tree;

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
}
