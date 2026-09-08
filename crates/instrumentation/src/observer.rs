// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared event forwarding state for entity observers.

use crate::context::{Runtime, drive};
use quent_events::{EntityEvent, Event};
use quent_io::Exporter;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    sync::mpsc::{UnboundedSender, unbounded_channel},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use uuid::Uuid;

/// Wrapper around an optional channel sender.
///
/// When the inner sender is `None` (i.e. the noop exporter is selected), `send`
/// is a no-op that avoids any channel or event-forwarding overhead.
pub struct EventSender<T> {
    tx: Option<UnboundedSender<Event<T>>>,
    /// Flag shared across clones to prevent potentially massive log spam from
    /// subseQUENT sender errors after the first.
    disable_error_log: Arc<AtomicBool>,
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

    /// A cloned [`EventSender`] feeding this pipeline.
    ///
    /// Lets a `'static` producer emit into the pipeline while the caller keeps
    /// ownership (and still flushes on drop). The sender does not keep the
    /// pipeline alive; sends after it is dropped are discarded (the first logs
    /// an error via `tracing`, then further ones are suppressed).
    pub fn sender(&self) -> EventSender<T> {
        self.events_sender.clone()
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
    T: Send + EntityEvent + 'static,
{
    let cancellation_token = CancellationToken::new();
    let cloned_token = cancellation_token.clone();
    let (events_sender, mut events_receiver) = unbounded_channel();

    let forwarder_handle = runtime.handle().spawn(async move {
        // Reused across batches; `drain_events` leaves it empty and it is reserved
        // to a full batch before each receive.
        let mut buffer = Vec::new();
        loop {
            let limit = exporter.batch_size_hint().get();
            buffer.reserve(limit);
            tokio::select! {
                // Cancel-safe: if the cancellation branch wins, `buffer` is left
                // untouched (no events are lost).
                n = events_receiver.recv_many(&mut buffer, limit) => {
                    // 0 means the channel is closed and drained.
                    if n == 0 {
                        break;
                    }
                    if let Err(e) = exporter.drain_events(&mut buffer).await {
                        warn!("unable to export events: {e}");
                    }
                    debug_assert!(buffer.is_empty(), "drain_events must leave the buffer empty");
                },
                () = cloned_token.cancelled() => {
                    events_receiver.close();
                    // drain events that are buffered
                    loop {
                        let limit = exporter.batch_size_hint().get();
                        buffer.reserve(limit);
                        if events_receiver.recv_many(&mut buffer, limit).await == 0 {
                            break;
                        }
                        if let Err(e) = exporter.drain_events(&mut buffer).await {
                            warn!("unable to export events: {e}");
                        }
                        debug_assert!(buffer.is_empty(), "drain_events must leave the buffer empty");
                    }
                    break
                },
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEvent;
    impl EntityEvent for TestEvent {
        const NAME: &'static str = "TestEvent";
    }

    #[test]
    fn noop_observer_holds_no_sender_and_discards_events() {
        let observer = ObserverInner::<TestEvent>::noop();
        assert!(observer.events_sender.tx.is_none());
        // Emitting is a silent no-op.
        observer.emit(Uuid::now_v7(), TestEvent);
    }
}
