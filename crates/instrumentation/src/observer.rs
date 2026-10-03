// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared event forwarding state for entity observers.

use crate::context::{Runtime, drive};
use quent_events::{Event, EventPayload};
use quent_io::Exporter;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[cfg(feature = "channel-spsc")]
use std::time::Duration;
#[cfg(not(feature = "channel-spsc"))]
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
#[cfg(feature = "channel-spsc")]
use tokio::time::{MissedTickBehavior, interval};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use uuid::Uuid;

/// Wrapper around an optional channel sender.
///
/// When the inner sender is `None` (i.e. the noop exporter is selected), `send`
/// is a no-op that avoids any channel or event-forwarding overhead. Active
/// senders follow the transport's shutdown contract; see `PERFORMANCE.md`.
pub struct EventSender<T> {
    tx: Option<TransportSender<T>>,
    /// Flag shared across clones to prevent potentially massive log spam from
    /// subseQUENT sender errors after the first.
    disable_error_log: Arc<AtomicBool>,
}

#[cfg(not(feature = "channel-spsc"))]
type TransportSender<T> = UnboundedSender<Event<T>>;

#[cfg(feature = "channel-spsc")]
// The function pointer keeps the public sender methods callable for generic
// `T`; construction proves `T: Send` once without adding a bound to callers.
type SpscSend<T> = fn(&quent_channel::Sender<Event<T>>, Event<T>) -> Result<(), Event<T>>;

#[cfg(feature = "channel-spsc")]
struct TransportSender<T> {
    tx: quent_channel::Sender<Event<T>>,
    send: SpscSend<T>,
}

#[cfg(feature = "channel-spsc")]
impl<T> Clone for TransportSender<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            send: self.send,
        }
    }
}

#[cfg(feature = "channel-spsc")]
impl<T> TransportSender<T> {
    fn send(&self, event: Event<T>) -> Result<(), Event<T>> {
        (self.send)(&self.tx, event)
    }
}

#[cfg(feature = "channel-spsc")]
fn send_spsc<T: Send + 'static>(
    tx: &quent_channel::Sender<Event<T>>,
    event: Event<T>,
) -> Result<(), Event<T>> {
    tx.send(event)
}

impl<T> std::fmt::Debug for EventSender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("EventSender<{}>", std::any::type_name::<T>()))
            .field("tx", &self.tx.as_ref().map(|_| ".."))
            .field("disable_error_log", &self.disable_error_log)
            .finish()
    }
}

impl<T> Clone for EventSender<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            disable_error_log: Arc::clone(&self.disable_error_log),
        }
    }
}

impl<T> EventSender<T> {
    /// Returns a noop sender that silently drops all events.
    pub fn noop() -> Self {
        Self {
            tx: None,
            disable_error_log: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn send(&self, event: Event<T>) {
        if let Some(tx) = &self.tx
            && tx.send(event).is_err()
            && !self.disable_error_log.swap(true, Ordering::Relaxed)
        {
            tracing::error!("unable to send event, suppressing further errors");
        }
    }

    /// Emit an event, converting it into the target type via `Into`.
    pub fn emit(&self, id: Uuid, event: impl Into<T>) {
        self.send(Event::new_now(id, event.into()));
    }
}

/// Backs an entity observer with event forwarding and exporter lifecycle management.
///
/// A context creates one instance per entity type and shares it among that
/// entity's observer and handles. Dropping the final shared owner cancels the
/// forwarder and flushes the exporter.
///
/// Hidden because generated `Observer<E>` and `Handle<E>` types expose this
/// lifecycle.
#[doc(hidden)]
pub struct ObserverInner<T> {
    events_sender: EventSender<T>,
    cancellation_token: CancellationToken,
    forwarder_handle: Option<JoinHandle<()>>,
    /// The runtime this pipeline's forwarder runs on; `None` for a no-op
    /// pipeline. An `Owned` runtime is kept alive here for the pipeline's
    /// lifetime, so its drop flush is valid even after the [`Context`] is gone.
    ///
    /// [`Context`]: crate::Context
    runtime: Option<Runtime>,
}

impl<T> ObserverInner<T> {
    /// Construct a no-op pipeline that discards events and holds no runtime
    /// resources whatsoever.
    pub fn noop() -> Self {
        Self {
            events_sender: EventSender::noop(),
            cancellation_token: CancellationToken::new(),
            forwarder_handle: None,
            runtime: None,
        }
    }

    /// Send a pre-built event into this stream.
    pub fn send(&self, event: Event<T>) {
        self.events_sender.send(event);
    }

    /// Emit an event for entity `id`, converting it into the stream type.
    pub fn emit(&self, id: Uuid, event: impl Into<T>) {
        self.events_sender.emit(id, event);
    }
}

impl<T> Drop for ObserverInner<T> {
    fn drop(&mut self) {
        self.cancellation_token.cancel();

        let (Some(runtime), Some(forwarder_handle)) = (&self.runtime, self.forwarder_handle.take())
        else {
            return;
        };

        // The forwarder drains remaining events and flushes the exporter on
        // cancellation; joining waits for that to finish. `drive` blocks here
        // whether dropped off a runtime or on a multi-threaded worker.
        if let Err(e) = drive(&runtime.handle(), forwarder_handle) {
            warn!("forwarder task failed: {e}");
        }
    }
}

/// Spawn the forwarder task for `exporter` on `runtime` and wrap it in an
/// [`ObserverInner`]. The task drains and flushes the exporter on cancellation.
pub(crate) fn spawn_forwarder<T>(
    runtime: &Runtime,
    mut exporter: Box<dyn Exporter<T>>,
) -> ObserverInner<T>
where
    T: Send + EventPayload + 'static,
{
    let cancellation_token = CancellationToken::new();
    let cloned_token = cancellation_token.clone();
    #[cfg(not(feature = "channel-spsc"))]
    let (events_sender, mut events_receiver) = unbounded_channel();
    #[cfg(feature = "channel-spsc")]
    let (events_sender, mut events_receiver) =
        quent_channel::unbounded_channel(quent_channel::Config::default());
    #[cfg(feature = "channel-spsc")]
    let events_sender = TransportSender {
        tx: events_sender,
        send: send_spsc::<T>,
    };

    let forwarder_handle = runtime.handle().spawn(async move {
        #[cfg(not(feature = "channel-spsc"))]
        forward_tokio(&mut events_receiver, &mut exporter, &cloned_token).await;
        #[cfg(feature = "channel-spsc")]
        forward_spsc(&mut events_receiver, &mut exporter, &cloned_token).await;
        // Tear down once, however the loop exited.
        if let Err(e) = exporter.shutdown().await {
            warn!("failed to shut down exporter: {e}");
        }
    });

    ObserverInner {
        events_sender: EventSender {
            tx: Some(events_sender),
            disable_error_log: Arc::new(AtomicBool::new(false)),
        },
        cancellation_token,
        forwarder_handle: Some(forwarder_handle),
        runtime: Some(runtime.clone()),
    }
}

async fn export_buffer<T: Send + 'static>(
    exporter: &mut Box<dyn Exporter<T>>,
    buffer: &mut Vec<Event<T>>,
) {
    if let Err(e) = exporter.drain_events(buffer).await {
        warn!("unable to export events: {e}");
    }
    debug_assert!(
        buffer.is_empty(),
        "drain_events must leave the buffer empty"
    );
}

#[cfg(not(feature = "channel-spsc"))]
async fn forward_tokio<T: Send + 'static>(
    receiver: &mut UnboundedReceiver<Event<T>>,
    exporter: &mut Box<dyn Exporter<T>>,
    cancellation: &CancellationToken,
) {
    let mut buffer = Vec::new();
    loop {
        let limit = exporter.batch_size_hint().get();
        buffer.reserve(limit);
        tokio::select! {
            // Cancellation leaves `buffer` untouched; the shutdown branch drains it.
            n = receiver.recv_many(&mut buffer, limit) => {
                if n == 0 {
                    break;
                }
                export_buffer(exporter, &mut buffer).await;
            },
            () = cancellation.cancelled() => {
                receiver.close();
                loop {
                    let limit = exporter.batch_size_hint().get();
                    buffer.reserve(limit);
                    if receiver.recv_many(&mut buffer, limit).await == 0 {
                        break;
                    }
                    export_buffer(exporter, &mut buffer).await;
                }
                break;
            },
        }
    }
}

#[cfg(feature = "channel-spsc")]
async fn forward_spsc<T: Send + 'static>(
    receiver: &mut quent_channel::Receiver<Event<T>>,
    exporter: &mut Box<dyn Exporter<T>>,
    cancellation: &CancellationToken,
) {
    let mut buffer = Vec::new();
    // Idle polling leaves the producer's ordinary push path free of wake-ups.
    let mut ticker = interval(Duration::from_millis(1));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        if cancellation.is_cancelled() {
            break;
        }
        let limit = exporter.batch_size_hint().get();
        buffer.reserve(limit);
        if receiver.drain_into(&mut buffer, limit).drained != 0 {
            export_buffer(exporter, &mut buffer).await;
            continue;
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            _ = ticker.tick() => {},
        }
    }

    // Completed pre-shutdown sends are published, but concurrent producers may
    // keep writing until their next segment switch.
    receiver.close();
    loop {
        let limit = exporter.batch_size_hint().get();
        buffer.reserve(limit);
        if receiver.drain_into(&mut buffer, limit).drained == 0 {
            break;
        }
        export_buffer(exporter, &mut buffer).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use quent_io::{ExporterProvider, ExporterResult};

    struct TestEvent;
    impl EventPayload for TestEvent {
        const NAME: &'static str = "TestEvent";
    }

    #[test]
    fn noop_observer_holds_no_sender_and_discards_events() {
        let observer = ObserverInner::<TestEvent>::noop();
        assert!(observer.events_sender.tx.is_none());
        // Emitting is a silent no-op.
        observer.emit(Uuid::now_v7(), TestEvent);
    }

    struct SequenceEvent(usize);

    impl EventPayload for SequenceEvent {
        const NAME: &'static str = "SequenceEvent";
    }

    struct RecordingProvider(Arc<Mutex<Vec<usize>>>);

    struct RecordingExporter(Arc<Mutex<Vec<usize>>>);

    #[async_trait::async_trait]
    impl ExporterProvider<SequenceEvent> for RecordingProvider {
        async fn create_exporter(
            &self,
            _context_id: Uuid,
        ) -> ExporterResult<Box<dyn Exporter<SequenceEvent>>> {
            Ok(Box::new(RecordingExporter(Arc::clone(&self.0))))
        }
    }

    #[async_trait::async_trait]
    impl Exporter<SequenceEvent> for RecordingExporter {
        async fn push(&mut self, event: Event<SequenceEvent>) -> ExporterResult<()> {
            self.0.lock().unwrap().push(event.data.0);
            Ok(())
        }

        async fn shutdown(self: Box<Self>) -> ExporterResult<()> {
            Ok(())
        }
    }

    #[test]
    fn final_owner_drop_exports_all_completed_sends() {
        const THREADS: usize = 4;
        const PER_THREAD: usize = 1_000;

        let recorded = Arc::new(Mutex::new(Vec::new()));
        let provider = RecordingProvider(Arc::clone(&recorded));
        let context = crate::ContextInner::try_new(Uuid::now_v7()).unwrap();
        let observer = Arc::new(
            context
                .block_on(async { context.observer::<SequenceEvent>(&provider).await })
                .unwrap(),
        );
        let mut workers = Vec::new();
        for thread in 0..THREADS {
            let observer = Arc::clone(&observer);
            workers.push(std::thread::spawn(move || {
                for index in 0..PER_THREAD {
                    observer.send(Event::new_now(
                        Uuid::nil(),
                        SequenceEvent(thread * PER_THREAD + index),
                    ));
                }
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        drop(observer);

        let mut values = recorded.lock().unwrap().clone();
        values.sort_unstable();
        assert_eq!(values, (0..THREADS * PER_THREAD).collect::<Vec<_>>());
    }
}
