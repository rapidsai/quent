//! Quent Instrumentation API
//!
use std::sync::Arc;

use quent_events::{Event, EventData, coordinator, engine, query};
use quent_exporter::Exporter;
use quent_exporter_collector::CollectorExporter;
use tokio::runtime::{Handle, Runtime};
use tracing::{debug, warn};
use uuid::Uuid;

fn push_event(
    sender: &tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
    event: Event<EventData>,
) {
    match sender.send(event) {
        Ok(_) => (),
        Err(e) => warn!("unable to send event: {e}"),
    }
}

pub struct Context {
    _runtime: Option<tokio::runtime::Runtime>,
    events_sender: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
    _exporter: Arc<dyn Exporter>,
}

impl Context {
    pub fn try_new(engine_id: Uuid) -> Result<Self, Box<dyn std::error::Error>> {
        let (runtime, handle) = if let Ok(handle) = Handle::try_current() {
            debug!("using existing async runtime");
            (None, handle)
        } else {
            debug!("spawning new async runtime");
            if let Ok(runtime) = Runtime::new() {
                let handle = runtime.handle().clone();
                (Some(runtime), handle)
            } else {
                return Err("unable to spawn async runtime")?;
            }
        };

        let (events_sender, mut events_receiver) = tokio::sync::mpsc::unbounded_channel();

        debug!("spawning collector exporter");
        let exporter = Arc::new(handle.block_on(CollectorExporter::new(engine_id))?);

        handle.spawn({
            let exporter = Arc::clone(&exporter);
            async move {
                while let Some(event) = events_receiver.recv().await {
                    match exporter.push(event).await {
                        Ok(_) => (), // successfully pushed to exporter,
                        Err(e) => warn!("unable to export event: {e}"),
                    }
                }
            }
        });

        Ok(Context {
            _runtime: runtime,
            events_sender,
            _exporter: exporter,
        })
    }

    pub fn engine_observer(&self) -> EngineObserver {
        EngineObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn coordinator_observer(&self) -> CoordinatorObserver {
        CoordinatorObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn query_observer(&self) -> QueryObserver {
        QueryObserver {
            tx: self.events_sender.clone(),
        }
    }
}

#[derive(Clone)]
pub struct EngineObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}

impl EngineObserver {
    pub fn init(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(id, engine::EngineEvent::Init(engine::Init {}).into()),
        )
    }

    pub fn operating(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(
                id,
                engine::EngineEvent::Operating(engine::Operating {}).into(),
            ),
        )
    }

    pub fn finalizing(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(
                id,
                engine::EngineEvent::Finalizing(engine::Finalizing {}).into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(id, engine::EngineEvent::Exit(engine::Exit {}).into()),
        )
    }
}

#[derive(Clone)]
pub struct CoordinatorObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}

impl CoordinatorObserver {
    pub fn init(&self, id: Uuid, engine_id: Uuid) {
        push_event(
            &self.tx,
            Event::new(
                id,
                coordinator::CoordinatorEvent::Init(coordinator::Init { engine_id }).into(),
            ),
        )
    }

    pub fn operating(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(
                id,
                coordinator::CoordinatorEvent::Operating(coordinator::Operating {}).into(),
            ),
        )
    }

    pub fn finalizing(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(
                id,
                coordinator::CoordinatorEvent::Finalizing(coordinator::Finalizing {}).into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(
                id,
                coordinator::CoordinatorEvent::Exit(coordinator::Exit {}).into(),
            ),
        )
    }
}

#[derive(Clone)]
pub struct QueryObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}

impl QueryObserver {
    pub fn init(&self, id: Uuid, coordinator_id: Uuid) {
        push_event(
            &self.tx,
            Event::new(
                id,
                query::QueryEvent::Init(query::Init { coordinator_id }).into(),
            ),
        )
    }

    pub fn planning(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(id, query::QueryEvent::Planning(query::Planning {}).into()),
        )
    }

    pub fn executing(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(id, query::QueryEvent::Executing(query::Executing {}).into()),
        );
    }

    pub fn idle(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(id, query::QueryEvent::Idle(query::Idle {}).into()),
        );
    }

    pub fn finalizing(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(
                id,
                query::QueryEvent::Finalizing(query::Finalizing {}).into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid) {
        push_event(
            &self.tx,
            Event::new(id, query::QueryEvent::Exit(query::Exit {}).into()),
        )
    }
}
