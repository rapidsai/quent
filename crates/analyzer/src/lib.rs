//! Analyzes raw events to produce useful performance insights

use std::collections::HashMap;

use quent_entities::{coordinator::Coordinator, engine::Engine, query::Query};
use quent_events::{
    Event as RawEvent, EventData, coordinator::CoordinatorEvent, engine::EngineEvent,
    query::QueryEvent,
};
use tracing::warn;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("validation error: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub type Event = RawEvent<EventData>;

// TODO(johanpel): make it fast
pub struct Analyzer {
    engine: Engine,
    coordinators: HashMap<Uuid, Coordinator>,
    queries: HashMap<Uuid, Query>,
}

// Just a slightly shorter way to get entry or insert default
fn entry<T>(map: &mut HashMap<Uuid, T>, id: Uuid) -> &mut T
where
    T: Default,
{
    map.entry(id).or_default()
}

impl Analyzer {
    pub fn try_new(engine_id: Uuid, mut events: impl Iterator<Item = Event>) -> Result<Self> {
        // TODO(johanpel): we need to sit down and think about how to do this as quickly as
        //                 possible for larger datasets, this is just a trivial implementation
        //                 to make it work. This is known to get pretty intense.
        let mut engine = Engine::new(engine_id);
        let mut coordinators: HashMap<Uuid, Coordinator> = HashMap::new();
        let mut queries: HashMap<Uuid, Query> = HashMap::new();

        events.try_for_each(|event| {
            let event: Event = event;
            let ts = event.timestamp;

            match event.data {
                // TODO(johanpel): validation logic
                EventData::Engine(engine_event) => match engine_event {
                    EngineEvent::Init(_) => engine.timestamps.init = Some(ts), // TODO(johanpel): validate engine id matches
                    EngineEvent::Operating(_) => engine.timestamps.operating = Some(ts),
                    EngineEvent::Finalizing(_) => engine.timestamps.finalizing = Some(ts),
                    EngineEvent::Exit(_) => engine.timestamps.exit = Some(ts),
                },
                EventData::Coordinator(coordinator_event) => {
                    let entry = entry(&mut coordinators, event.id);
                    match coordinator_event {
                        CoordinatorEvent::Init(init) => {
                            entry.id = event.id;
                            entry.engine_id = init.engine_id;
                            entry.timestamps.init = Some(ts);
                        }
                        CoordinatorEvent::Operating(_) => entry.timestamps.operating = Some(ts),
                        CoordinatorEvent::Finalizing(_) => entry.timestamps.finalizing = Some(ts),
                        CoordinatorEvent::Exit(_) => entry.timestamps.exit = Some(ts),
                    }
                }
                EventData::Query(query_event) => {
                    let entry = entry(&mut queries, event.id);
                    match query_event {
                        QueryEvent::Init(init) => {
                            entry.id = event.id;
                            entry.coordinator_id = init.coordinator_id;
                            entry.timestamps.init = Some(ts);
                        }
                        QueryEvent::Planning(_) => entry.timestamps.planning = Some(ts),
                        QueryEvent::Executing(_) => entry.timestamps.executing = Some(ts),
                        QueryEvent::Idle(_) => entry.timestamps.idle = Some(ts),
                        QueryEvent::Finalizing(_) => entry.timestamps.finalizing = Some(ts),
                        QueryEvent::Exit(_) => entry.timestamps.exit = Some(ts),
                    }
                }
                x => warn!("analysis of event type not implemented: {x:?}"),
            }
            Ok(())
        })?;

        // All events are transformed into entities. Filter out parentless entities.
        for key in coordinators.keys().cloned().collect::<Vec<_>>() {
            if coordinators.get(&key).unwrap().engine_id != engine_id {
                coordinators.remove(&key);
            }
        }
        for key in queries.keys().cloned().collect::<Vec<_>>() {
            if !coordinators.contains_key(&queries.get(&key).unwrap().coordinator_id) {
                queries.remove(&key);
            }
        }

        dbg!(&engine);
        dbg!(&coordinators);
        dbg!(&queries);

        Ok(Self {
            engine,
            coordinators,
            queries,
        })
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    // TODO(johanpel): this is separated from an engine, since we assume engines can have
    //                 immense lifetimes so they could be running lots of coordinators, in
    //                 which case we may want to implement pagination for this.
    pub fn coordinator_ids(&self) -> Vec<Uuid> {
        self.coordinators.keys().cloned().collect()
    }

    pub fn coordinator(&self, id: Uuid) -> Option<&Coordinator> {
        self.coordinators.get(&id)
    }

    // TODO(johanpel): pagination
    pub fn query_ids(&self, coordinator_id: Uuid) -> Vec<Uuid> {
        self.queries
            .iter()
            .filter_map(|(k, v)| (v.coordinator_id == coordinator_id).then_some(*k))
            .collect()
    }

    pub fn query(&self, id: Uuid) -> Option<&Query> {
        self.queries.get(&id)
    }
}
