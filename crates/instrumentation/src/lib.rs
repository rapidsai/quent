// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Quent Instrumentation API
//!
use quent_events::Event;
use quent_exporter::{ExporterOptions, create_exporter};
use quent_exporter_types::Exporter;
use serde::Serialize;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};
use tokio::{
    runtime::{Handle, Runtime},
    sync::mpsc::{UnboundedSender, unbounded_channel},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use uuid::Uuid;

type ExporterHandle<T> = Arc<dyn Exporter<T>>;

/// Wrapper around an optional channel sender. When the inner sender is `None`
/// (i.e. the noop exporter is selected), `send` is a no-op that avoids any
/// channel or event-forwarding overhead.
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

impl<T> Default for EventSender<T> {
    fn default() -> Self {
        Self::noop()
    }
}

impl<T> EventSender<T> {
    /// Returns a noop sender that silently drops all events.
    pub fn noop() -> Self {
        Self {
            tx: None,
            disable_error_log: Arc::new(AtomicBool::new(false)),
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

pub struct Context<T>
where
    T: Serialize + Send + 'static,
{
    handle: Option<Handle>,
    events_sender: EventSender<T>,
    exporter: Option<ExporterHandle<T>>,
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
    T: Serialize + Send + 'static,
{
    pub fn try_new(
        id: Uuid,
        exporter: Option<ExporterOptions>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let kind = match exporter {
            None => {
                debug!("using noop exporter");
                return Ok(Context {
                    handle: None,
                    events_sender: EventSender {
                        tx: None,
                        disable_error_log: Arc::new(AtomicBool::new(false)),
                    },
                    exporter: None,
                    cancellation_token: CancellationToken::new(),
                    forwarder_handle: None,
                    _runtime: None,
                });
            }
            Some(kind) => kind,
        };

        let (runtime, handle) = if let Ok(handle) = Handle::try_current() {
            debug!("using existing async runtime");
            (None, handle)
        } else {
            debug!("spawning async runtime");
            let runtime =
                Runtime::new().map_err(|e| format!("unable to spawn async runtime: {e}"))?;
            let handle = runtime.handle().clone();
            (Some(runtime), handle)
        };

        let (events_sender, mut events_receiver) = unbounded_channel();

        debug!("constructing exporter");
        let exporter = create_exporter_on_handle(&handle, kind, id)?;

        let cancellation_token = CancellationToken::new();
        let cloned_token = cancellation_token.clone();

        let forwarder_handle = handle.spawn({
            let exporter: ExporterHandle<T> = Arc::clone(&exporter);
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
            handle: Some(handle),
            events_sender: EventSender {
                tx: Some(events_sender),
                disable_error_log: Arc::new(AtomicBool::new(false)),
            },
            exporter: Some(exporter),
            cancellation_token,
            forwarder_handle: Some(forwarder_handle),
            _runtime: runtime,
        })
    }

    pub fn events_sender(&self) -> EventSender<T> {
        self.events_sender.clone()
    }
}

impl<T> Drop for Context<T>
where
    T: Serialize + Send + 'static,
{
    fn drop(&mut self) {
        self.cancellation_token.cancel();

        if let Some(handle) = self.handle.take() {
            shutdown_runtime(
                handle,
                self.forwarder_handle.take(),
                self.exporter.take(),
                self._runtime.take(),
            );
        }
    }
}

fn create_exporter_on_handle<T>(
    handle: &Handle,
    kind: ExporterOptions,
    id: Uuid,
) -> Result<ExporterHandle<T>, Box<dyn std::error::Error>>
where
    T: Serialize + Send + 'static,
{
    // `Handle::block_on` panics from a Tokio worker thread, so keep the sync
    // constructor API by doing the blocking exporter setup on a helper thread.
    if Handle::try_current().is_ok() {
        let handle = handle.clone();
        let join_result = thread::spawn(move || {
            handle
                .block_on(create_exporter(kind, id))
                .map_err(|e| e.to_string())
        })
        .join();

        return match join_result {
            Ok(Ok(exporter)) => Ok(exporter),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err("exporter construction thread panicked".into()),
        };
    }

    Ok(handle.block_on(create_exporter(kind, id))?)
}

fn shutdown_runtime<T>(
    handle: Handle,
    forwarder_handle: Option<JoinHandle<()>>,
    exporter: Option<ExporterHandle<T>>,
    runtime: Option<Runtime>,
) where
    T: Serialize + Send + 'static,
{
    // `Drop` is synchronous too; use the same helper-thread escape hatch when
    // teardown happens from inside a Tokio runtime.
    if Handle::try_current().is_ok() {
        if thread::spawn(move || {
            shutdown_runtime_on_current_thread(handle, forwarder_handle, exporter, runtime)
        })
        .join()
        .is_err()
        {
            warn!("runtime shutdown thread panicked");
        }
        return;
    }

    shutdown_runtime_on_current_thread(handle, forwarder_handle, exporter, runtime);
}

fn shutdown_runtime_on_current_thread<T>(
    handle: Handle,
    forwarder_handle: Option<JoinHandle<()>>,
    exporter: Option<ExporterHandle<T>>,
    runtime: Option<Runtime>,
) where
    T: Serialize + Send + 'static,
{
    // Wait for the forwarder to finish processing remaining events.
    if let Some(forwarder_handle) = forwarder_handle
        && let Err(e) = handle.block_on(forwarder_handle)
    {
        warn!("forwarder task failed: {e}");
    }

    // Flush the exporter to ensure all events are sent.
    if let Some(exporter) = exporter {
        if let Err(e) = handle.block_on(exporter.force_flush()) {
            warn!("failed to flush exporter: {e}");
        }
        drop(exporter);
    }

    drop(runtime);
}

#[cfg(test)]
mod tests {
    use super::*;
    use quent_exporter::NdjsonExporterOptions;
    use tempfile::TempDir;

    #[derive(Debug, serde::Serialize)]
    struct TestEvent;

    fn ndjson_exporter(output: &TempDir) -> ExporterOptions {
        ExporterOptions::Ndjson(NdjsonExporterOptions {
            output_dir: output.path().to_path_buf(),
        })
    }

    #[test]
    fn noop_exporter() {
        let ctx = Context::<TestEvent>::try_new(Uuid::now_v7(), None).unwrap();
        assert!(ctx.exporter.is_none());
        assert!(ctx.forwarder_handle.is_none());
        assert!(ctx._runtime.is_none());

        let sender = ctx.events_sender();
        assert!(sender.tx.is_none());

        sender.send(Event::new_now(Uuid::now_v7(), TestEvent));
        sender.send(Event::new_now(Uuid::now_v7(), TestEvent));
        drop(ctx);
    }

    #[test]
    fn exporter_context_can_be_created_inside_existing_runtime() {
        let output = TempDir::new().unwrap();
        let runtime = Runtime::new().unwrap();
        let context_id = Uuid::now_v7();
        let event_id = Uuid::now_v7();
        let output_file = output.path().join(format!("{context_id}.ndjson"));

        runtime.block_on(async {
            let ctx =
                Context::<TestEvent>::try_new(context_id, Some(ndjson_exporter(&output))).unwrap();
            assert!(ctx._runtime.is_none());
            ctx.events_sender().emit(event_id, TestEvent);
            drop(ctx);
        });

        let contents = std::fs::read_to_string(output_file).unwrap();
        assert!(contents.contains(&event_id.to_string()));
    }

    #[test]
    fn exporter_context_can_be_dropped_inside_existing_runtime() {
        let output = TempDir::new().unwrap();
        let runtime = Runtime::new().unwrap();
        let context_id = Uuid::now_v7();
        let event_id = Uuid::now_v7();
        let output_file = output.path().join(format!("{context_id}.ndjson"));
        let ctx =
            Context::<TestEvent>::try_new(context_id, Some(ndjson_exporter(&output))).unwrap();

        assert!(ctx._runtime.is_some());
        ctx.events_sender().emit(event_id, TestEvent);
        runtime.block_on(async move {
            drop(ctx);
        });

        let contents = std::fs::read_to_string(output_file).unwrap();
        assert!(contents.contains(&event_id.to_string()));
    }
}
