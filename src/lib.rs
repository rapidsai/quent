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
    pub fn init(&self, id: Uuid, init: engine::Init) {
        push_event(
            &self.tx,
            Event::new(id, engine::EngineEvent::Init(init).into()),
        )
    }

    pub fn operating(&self, id: Uuid, operating: engine::Operating) {
        push_event(
            &self.tx,
            Event::new(id, engine::EngineEvent::Operating(operating).into()),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: engine::Finalizing) {
        push_event(
            &self.tx,
            Event::new(id, engine::EngineEvent::Finalizing(finalizing).into()),
        )
    }

    pub fn exit(&self, id: Uuid, exit: engine::Exit) {
        push_event(
            &self.tx,
            Event::new(id, engine::EngineEvent::Exit(exit).into()),
        )
    }
}

#[derive(Clone)]
pub struct CoordinatorObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}

impl CoordinatorObserver {
    pub fn init(&self, id: Uuid, init: coordinator::Init) {
        push_event(
            &self.tx,
            Event::new(id, coordinator::CoordinatorEvent::Init(init).into()),
        )
    }

    pub fn operating(&self, id: Uuid, operating: coordinator::Operating) {
        push_event(
            &self.tx,
            Event::new(
                id,
                coordinator::CoordinatorEvent::Operating(operating).into(),
            ),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: coordinator::Finalizing) {
        push_event(
            &self.tx,
            Event::new(
                id,
                coordinator::CoordinatorEvent::Finalizing(finalizing).into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid, exit: coordinator::Exit) {
        push_event(
            &self.tx,
            Event::new(id, coordinator::CoordinatorEvent::Exit(exit).into()),
        )
    }
}

#[derive(Clone)]
pub struct QueryObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}

impl QueryObserver {
    pub fn init(&self, id: Uuid, init: query::Init) {
        push_event(
            &self.tx,
            Event::new(id, query::QueryEvent::Init(init).into()),
        )
    }

    pub fn planning(&self, id: Uuid, planning: query::Planning) {
        push_event(
            &self.tx,
            Event::new(id, query::QueryEvent::Planning(planning).into()),
        )
    }

    pub fn executing(&self, id: Uuid, executing: query::Executing) {
        push_event(
            &self.tx,
            Event::new(id, query::QueryEvent::Executing(executing).into()),
        );
    }

    pub fn idle(&self, id: Uuid, idle: query::Idle) {
        push_event(
            &self.tx,
            Event::new(id, query::QueryEvent::Idle(idle).into()),
        );
    }

    pub fn finalizing(&self, id: Uuid, finalizing: query::Finalizing) {
        push_event(
            &self.tx,
            Event::new(id, query::QueryEvent::Finalizing(finalizing).into()),
        )
    }

    pub fn exit(&self, id: Uuid, exit: query::Exit) {
        push_event(
            &self.tx,
            Event::new(id, query::QueryEvent::Exit(exit).into()),
        )
    }
}
