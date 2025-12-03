//! Analyzes raw events to produce useful performance insights

use quent_entities::engine::Engine;
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
}

impl Analyzer {
    pub fn try_new(engine_id: Uuid, mut events: impl Iterator<Item = Event>) -> Result<Self> {
        let mut engine = Engine::new(engine_id);

        events.try_for_each(|event| {
            let event: Event = event;
            let ts = event.timestamp;

            match event.data {
                // TODO(johanpel): validation logic
                EventData::Engine(engine_event) => match engine_event {
                    EngineEvent::Init(_) => engine.init = Some(ts), // TODO(johanpel): validate engine id matches
                    EngineEvent::Operating(_) => engine.operating = Some(ts),
                    EngineEvent::Finalizing(_) => engine.finalizing = Some(ts),
                    EngineEvent::Exit(_) => engine.exit = Some(ts),
                },
                x => warn!("analysis of event type not implemented: {x:?}"),
            }
            Ok(())
        })?;

        Ok(Self { engine })
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}
