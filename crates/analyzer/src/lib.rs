//! Analyzes raw events to produce useful performance insights

use std::collections::HashMap;

use quent_entities::{coordinator::Coordinator, engine::Engine};
use quent_events::{Event as RawEvent, EventData, engine::EngineEvent};
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
}

impl Analyzer {
    pub fn try_new(engine_id: Uuid, mut events: impl Iterator<Item = Event>) -> Result<Self> {
        // TODO(johanpel): we need to sit down and think about how to do this as quickly as
        //                 possible for larger datasets, this is just a trivial implementation
        //                 to make it work. This is known to get pretty intense.
        let mut engine = Engine::new(engine_id);
        let mut coordinators: HashMap<Uuid, Coordinator> = HashMap::new();

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
                EventData::Coordinator(coordinator_event) => match coordinator_event {
                    quent_events::coordinator::CoordinatorEvent::Init(init) => {
                        let entry = coordinators.entry(event.id).or_default();
                        entry.engine_id = init.engine_id;
                        entry.timestamps.init = Some(ts);
                    }
                    quent_events::coordinator::CoordinatorEvent::Operating(_) => {
                        coordinators
                            .entry(event.id)
                            .or_default()
                            .timestamps
                            .operating = Some(ts)
                    }
                    quent_events::coordinator::CoordinatorEvent::Finalizing(_) => {
                        coordinators
                            .entry(event.id)
                            .or_default()
                            .timestamps
                            .finalizing = Some(ts)
                    }
                    quent_events::coordinator::CoordinatorEvent::Exit(_) => {
                        coordinators.entry(event.id).or_default().timestamps.exit = Some(ts)
                    }
                },
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

        Ok(Self {
            engine,
            coordinators,
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
}
