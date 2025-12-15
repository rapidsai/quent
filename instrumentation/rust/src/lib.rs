//! Quent Instrumentation API
//!
use std::sync::Arc;

use quent_events::{
    Event, EventData, engine, operator, plan, query, query_group,
    resource::{channel, group, memory, processor},
    worker,
};
use quent_exporter::Exporter;
use quent_exporter_collector::{CollectorExporter, CollectorExporterOptions};
use quent_exporter_ndjson::NdjsonExporter;
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

pub enum ExporterOptions {
    Collector(CollectorExporterOptions),
    Ndjson,
}

pub struct Context {
    _runtime: Option<tokio::runtime::Runtime>,
    events_sender: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
    _exporter: Arc<dyn Exporter>,
}

impl Context {
    pub fn try_new(
        exporter: ExporterOptions,
        engine_id: Uuid,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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

        debug!("constructing exporter");
        let exporter: Arc<dyn Exporter> = match exporter {
            ExporterOptions::Collector(opts) => {
                Arc::new(handle.block_on(CollectorExporter::new(engine_id, opts))?)
            }
            ExporterOptions::Ndjson => {
                Arc::new(handle.block_on(NdjsonExporter::try_new(engine_id))?)
            }
        };

        handle.spawn({
            let exporter: Arc<dyn Exporter> = Arc::clone(&exporter);
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

    // This is a lot of repetition but some FFIs don't allow generics so either
    // we need to do macros or just keep it spelled out like this.
    // Or move this burden to the FFI layer itself.

    pub fn engine_observer(&self) -> EngineObserver {
        EngineObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn query_group_observer(&self) -> QueryGroupObserver {
        QueryGroupObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn worker_observer(&self) -> WorkerObserver {
        WorkerObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn query_observer(&self) -> QueryObserver {
        QueryObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn plan_observer(&self) -> PlanObserver {
        PlanObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn operator_observer(&self) -> OperatorObserver {
        OperatorObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn memory_resource_observer(&self) -> MemoryResourceObserver {
        MemoryResourceObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn processor_resource_observer(&self) -> ProcessorResourceObserver {
        ProcessorResourceObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn channel_resource_observer(&self) -> ChannelResourceObserver {
        ChannelResourceObserver {
            tx: self.events_sender.clone(),
        }
    }
    pub fn resource_group_observer(&self) -> ResourceGroupObserver {
        ResourceGroupObserver {
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
pub struct QueryGroupObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}

impl QueryGroupObserver {
    pub fn init(&self, id: Uuid, init: query_group::Init) {
        push_event(
            &self.tx,
            Event::new(id, query_group::QueryGroupEvent::Init(init).into()),
        )
    }

    pub fn operating(&self, id: Uuid, operating: query_group::Operating) {
        push_event(
            &self.tx,
            Event::new(
                id,
                query_group::QueryGroupEvent::Operating(operating).into(),
            ),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: query_group::Finalizing) {
        push_event(
            &self.tx,
            Event::new(
                id,
                query_group::QueryGroupEvent::Finalizing(finalizing).into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid, exit: query_group::Exit) {
        push_event(
            &self.tx,
            Event::new(id, query_group::QueryGroupEvent::Exit(exit).into()),
        )
    }
}

#[derive(Clone)]
pub struct WorkerObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}

impl WorkerObserver {
    pub fn init(&self, id: Uuid, init: worker::Init) {
        push_event(
            &self.tx,
            Event::new(id, worker::WorkerEvent::Init(init).into()),
        )
    }

    pub fn operating(&self, id: Uuid, operating: worker::Operating) {
        push_event(
            &self.tx,
            Event::new(id, worker::WorkerEvent::Operating(operating).into()),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: worker::Finalizing) {
        push_event(
            &self.tx,
            Event::new(id, worker::WorkerEvent::Finalizing(finalizing).into()),
        )
    }

    pub fn exit(&self, id: Uuid, exit: worker::Exit) {
        push_event(
            &self.tx,
            Event::new(id, worker::WorkerEvent::Exit(exit).into()),
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

#[derive(Clone)]
pub struct PlanObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}

impl PlanObserver {
    pub fn init(&self, id: Uuid, init: plan::Init) {
        push_event(&self.tx, Event::new(id, plan::PlanEvent::Init(init).into()))
    }

    pub fn executing(&self, id: Uuid, executing: plan::Executing) {
        push_event(
            &self.tx,
            Event::new(id, plan::PlanEvent::Executing(executing).into()),
        );
    }

    pub fn idle(&self, id: Uuid, idle: plan::Idle) {
        push_event(&self.tx, Event::new(id, plan::PlanEvent::Idle(idle).into()));
    }

    pub fn finalizing(&self, id: Uuid, finalizing: plan::Finalizing) {
        push_event(
            &self.tx,
            Event::new(id, plan::PlanEvent::Finalizing(finalizing).into()),
        )
    }

    pub fn exit(&self, id: Uuid, exit: plan::Exit) {
        push_event(&self.tx, Event::new(id, plan::PlanEvent::Exit(exit).into()))
    }
}

#[derive(Clone)]
pub struct OperatorObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}

impl OperatorObserver {
    pub fn init(&self, id: Uuid, init: operator::Init) {
        push_event(
            &self.tx,
            Event::new(id, operator::OperatorEvent::Init(init).into()),
        )
    }

    pub fn waiting_for_inputs(&self, id: Uuid, waiting_for_inputs: operator::WaitingForInputs) {
        push_event(
            &self.tx,
            Event::new(
                id,
                operator::OperatorEvent::WaitingForInputs(waiting_for_inputs).into(),
            ),
        );
    }

    pub fn executing(&self, id: Uuid, executing: operator::Executing) {
        push_event(
            &self.tx,
            Event::new(id, operator::OperatorEvent::Executing(executing).into()),
        );
    }

    pub fn blocked(&self, id: Uuid, blocked: operator::Blocked) {
        push_event(
            &self.tx,
            Event::new(id, operator::OperatorEvent::Blocked(blocked).into()),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: operator::Finalizing) {
        push_event(
            &self.tx,
            Event::new(id, operator::OperatorEvent::Finalizing(finalizing).into()),
        )
    }

    pub fn exit(&self, id: Uuid, exit: operator::Exit) {
        push_event(
            &self.tx,
            Event::new(id, operator::OperatorEvent::Exit(exit).into()),
        )
    }
}

#[derive(Clone)]
pub struct MemoryResourceObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}
impl MemoryResourceObserver {
    pub fn init(&self, id: Uuid, init: memory::Init) {
        push_event(
            &self.tx,
            Event::new(id, memory::MemoryResourceEvent::Init(init).into()),
        )
    }

    pub fn operating(&self, id: Uuid, operating: memory::Operating) {
        push_event(
            &self.tx,
            Event::new(id, memory::MemoryResourceEvent::Operating(operating).into()),
        )
    }

    pub fn resizing(&self, id: Uuid, resizing: memory::Resizing) {
        push_event(
            &self.tx,
            Event::new(id, memory::MemoryResourceEvent::Resizing(resizing).into()),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: memory::Finalizing) {
        push_event(
            &self.tx,
            Event::new(
                id,
                memory::MemoryResourceEvent::Finalizing(finalizing).into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid, exit: memory::Exit) {
        push_event(
            &self.tx,
            Event::new(id, memory::MemoryResourceEvent::Exit(exit).into()),
        )
    }
}

#[derive(Clone)]
pub struct ProcessorResourceObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}
impl ProcessorResourceObserver {
    pub fn init(&self, id: Uuid, init: processor::Init) {
        push_event(
            &self.tx,
            Event::new(id, processor::ProcessorResourceEvent::Init(init).into()),
        )
    }

    pub fn operating(&self, id: Uuid, operating: processor::Operating) {
        push_event(
            &self.tx,
            Event::new(
                id,
                processor::ProcessorResourceEvent::Operating(operating).into(),
            ),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: processor::Finalizing) {
        push_event(
            &self.tx,
            Event::new(
                id,
                processor::ProcessorResourceEvent::Finalizing(finalizing).into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid, exit: processor::Exit) {
        push_event(
            &self.tx,
            Event::new(id, processor::ProcessorResourceEvent::Exit(exit).into()),
        )
    }
}

#[derive(Clone)]
pub struct ChannelResourceObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}
impl ChannelResourceObserver {
    pub fn init(&self, id: Uuid, init: channel::Init) {
        push_event(
            &self.tx,
            Event::new(id, channel::ChannelResourceEvent::Init(init).into()),
        )
    }

    pub fn operating(&self, id: Uuid, operating: channel::Operating) {
        push_event(
            &self.tx,
            Event::new(
                id,
                channel::ChannelResourceEvent::Operating(operating).into(),
            ),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: channel::Finalizing) {
        push_event(
            &self.tx,
            Event::new(
                id,
                channel::ChannelResourceEvent::Finalizing(finalizing).into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid, exit: channel::Exit) {
        push_event(
            &self.tx,
            Event::new(id, channel::ChannelResourceEvent::Exit(exit).into()),
        )
    }
}

#[derive(Clone)]
pub struct ResourceGroupObserver {
    tx: tokio::sync::mpsc::UnboundedSender<Event<EventData>>,
}
impl ResourceGroupObserver {
    pub fn init(&self, id: Uuid, init: group::Init) {
        push_event(
            &self.tx,
            Event::new(id, group::ResourceGroupEvent::Init(init).into()),
        )
    }

    pub fn operating(&self, id: Uuid, operating: group::Operating) {
        push_event(
            &self.tx,
            Event::new(id, group::ResourceGroupEvent::Operating(operating).into()),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: group::Finalizing) {
        push_event(
            &self.tx,
            Event::new(id, group::ResourceGroupEvent::Finalizing(finalizing).into()),
        )
    }

    pub fn exit(&self, id: Uuid, exit: group::Exit) {
        push_event(
            &self.tx,
            Event::new(id, group::ResourceGroupEvent::Exit(exit).into()),
        )
    }
}
