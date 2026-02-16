//! Quent Instrumentation API
//!
use std::sync::Arc;

use quent_events::{Event, resource};
use quent_exporter::Exporter;
use quent_exporter_collector::{CollectorExporter, CollectorExporterOptions};
use quent_exporter_msgpack::MsgpackExporter;
use quent_exporter_ndjson::NdjsonExporter;
use quent_exporter_postcard::PostcardExporter;
use serde::Serialize;
use tokio::{
    runtime::{Handle, Runtime},
    sync::mpsc::{UnboundedSender, unbounded_channel},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use uuid::Uuid;

pub enum ExporterOptions {
    Collector(CollectorExporterOptions),
    Ndjson,
    Msgpack,
    Postcard,
}

pub struct Context<T>
where
    T: Serialize + Send + std::fmt::Debug + 'static,
{
    handle: Handle,
    events_sender: UnboundedSender<Event<T>>,
    exporter: Arc<dyn Exporter<T>>,
    cancellation_token: CancellationToken,
    forwarder_handle: Option<JoinHandle<()>>,

    // The runtime should be the last field, so it is dropped the last
    // (see https://doc.rust-lang.org/reference/destructors.html for
    // drop order of structs) because other tasks for exporters and
    // forwarders rely on this runtime.
    _runtime: Option<tokio::runtime::Runtime>,
}

impl<T> Context<T>
where
    T: Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn try_new(
        exporter: ExporterOptions,
        id: Uuid,
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

        let (events_sender, mut events_receiver) = unbounded_channel();

        debug!("constructing exporter");
        let exporter: Arc<dyn Exporter<T>> = match exporter {
            ExporterOptions::Collector(opts) => {
                Arc::new(handle.block_on(CollectorExporter::new(id, opts))?)
            }
            ExporterOptions::Ndjson => Arc::new(handle.block_on(NdjsonExporter::try_new(id))?),
            ExporterOptions::Msgpack => Arc::new(handle.block_on(MsgpackExporter::try_new(id))?),
            ExporterOptions::Postcard => Arc::new(handle.block_on(PostcardExporter::try_new(id))?),
        };

        let cancellation_token = CancellationToken::new();
        let cloned_token = cancellation_token.clone();

        let forwarder_handle = handle.spawn({
            let exporter: Arc<dyn Exporter<T>> = Arc::clone(&exporter);
            async move {
                loop {
                    tokio::select! {
                        Some(event) = events_receiver.recv() => {
                            match exporter.push(event).await {
                                Ok(_) => (), // successfully pushed to exporter,
                                Err(e) => warn!("unable to export event: {e}"),
                            }
                        },
                        () = cloned_token.cancelled() => {
                            events_receiver.close();
                            // drain events that are buffered
                            while let Some(event) = events_receiver.recv().await {
                                match exporter.push(event).await {
                                    Ok(_) => (), // successfully pushed to exporter,
                                    Err(e) => warn!("unable to export event: {e}"),
                                }
                            }
                            break
                        },
                        else => {
                            // we only enter here when the events_receiver
                            // channel has been closed (.recv() returns None)
                            // so no messages to receive or push to the
                            // exporter, so simply break.
                            break
                        }
                    }
                }
            }
        });

        Ok(Context {
            handle,
            events_sender,
            exporter,
            cancellation_token,
            forwarder_handle: Some(forwarder_handle),
            _runtime: runtime,
        })
    }

    pub fn events_sender(&self) -> UnboundedSender<Event<T>> {
        self.events_sender.clone()
    }

    pub fn push_event(sender: &UnboundedSender<Event<T>>, event: Event<T>) {
        match sender.send(event) {
            Ok(_) => (),
            Err(e) => warn!("unable to send event: {e}"),
        }
    }
}

impl<T> Drop for Context<T>
where
    T: Serialize + Send + std::fmt::Debug + 'static,
{
    fn drop(&mut self) {
        self.cancellation_token.cancel();

        // Wait for the forwarder to finish processing remaining events
        if let Some(forwarder_handle) = self.forwarder_handle.take()
            && let Err(e) = self.handle.block_on(forwarder_handle)
        {
            warn!("forwarder task failed: {e}");
        }

        // Flush the exporter to ensure all events are sent
        if let Err(e) = self.handle.block_on(self.exporter.force_flush()) {
            warn!("failed to flush exporter: {e}");
        }
    }
}

#[derive(Clone)]
pub struct MemoryResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    tx: UnboundedSender<Event<T>>,
}

impl<T> MemoryResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn new(tx: UnboundedSender<Event<T>>) -> Self {
        Self { tx }
    }

    pub fn init(&self, id: Uuid, init: resource::memory::Init) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Memory(resource::memory::MemoryEvent::Init(init)).into(),
            ),
        )
    }

    pub fn operating(&self, id: Uuid, operating: resource::memory::Operating) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Memory(resource::memory::MemoryEvent::Operating(
                    operating,
                ))
                .into(),
            ),
        )
    }

    pub fn resizing(&self, id: Uuid, resizing: resource::memory::Resizing) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Memory(resource::memory::MemoryEvent::Resizing(resizing))
                    .into(),
            ),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: resource::memory::Finalizing) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Memory(resource::memory::MemoryEvent::Finalizing(
                    finalizing,
                ))
                .into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid, exit: resource::memory::Exit) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Memory(resource::memory::MemoryEvent::Exit(exit)).into(),
            ),
        )
    }
}

#[derive(Clone)]
pub struct ProcessorResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    tx: UnboundedSender<Event<T>>,
}

impl<T> ProcessorResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn new(tx: UnboundedSender<Event<T>>) -> Self {
        Self { tx }
    }

    pub fn init(&self, id: Uuid, init: resource::processor::Init) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Processor(resource::processor::ProcessorEvent::Init(init))
                    .into(),
            ),
        )
    }

    pub fn operating(&self, id: Uuid, operating: resource::processor::Operating) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Processor(resource::processor::ProcessorEvent::Operating(
                    operating,
                ))
                .into(),
            ),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: resource::processor::Finalizing) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Processor(
                    resource::processor::ProcessorEvent::Finalizing(finalizing),
                )
                .into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid, exit: resource::processor::Exit) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Processor(resource::processor::ProcessorEvent::Exit(exit))
                    .into(),
            ),
        )
    }
}

#[derive(Clone)]
pub struct ChannelResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    tx: UnboundedSender<Event<T>>,
}

impl<T> ChannelResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn new(tx: UnboundedSender<Event<T>>) -> Self {
        Self { tx }
    }

    pub fn init(&self, id: Uuid, init: resource::channel::Init) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Channel(resource::channel::ChannelEvent::Init(init))
                    .into(),
            ),
        )
    }

    pub fn operating(&self, id: Uuid, operating: resource::channel::Operating) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Channel(resource::channel::ChannelEvent::Operating(
                    operating,
                ))
                .into(),
            ),
        )
    }

    pub fn finalizing(&self, id: Uuid, finalizing: resource::channel::Finalizing) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Channel(resource::channel::ChannelEvent::Finalizing(
                    finalizing,
                ))
                .into(),
            ),
        )
    }

    pub fn exit(&self, id: Uuid, exit: resource::channel::Exit) {
        Context::push_event(
            &self.tx,
            Event::new_now(
                id,
                resource::ResourceEvent::Channel(resource::channel::ChannelEvent::Exit(exit))
                    .into(),
            ),
        )
    }
}

#[derive(Clone)]
pub struct ResourceGroupObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    tx: UnboundedSender<Event<T>>,
}

impl<T> ResourceGroupObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn new(tx: UnboundedSender<Event<T>>) -> Self {
        Self { tx }
    }

    pub fn group(&self, id: Uuid, group: resource::GroupEvent) {
        Context::push_event(
            &self.tx,
            Event::new_now(id, resource::ResourceEvent::Group(group).into()),
        )
    }
}
