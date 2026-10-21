// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Convert and forward NVTX records to a Quent observer on a worker thread.
//!
//! The hook queues records while a worker owns the observer. This has several
//! benefits:
//!
//! - Text decoding and Quent event construction do not delay NVTX callers.
//! - Observer shutdown stays out of callbacks during process shutdown. A
//!   library may emit NVTX from a global destructor. If that callback held
//!   the last observer reference, it would block while the exporter flushes.
//! - In the unusual case where an exporter or a library it calls emits NVTX,
//!   the callback does not trigger observer shutdown. If it did, it would wait
//!   for the exporter that is executing it and deadlock.

use std::io;
use std::thread::{self, JoinHandle};

use nvtx_injection::Record;
use quent_events::{AnyEntity, EntityRef, Event};
use quent_instrumentation::{InstrumentedEntity, Observer};
use quent_nvtx_sink::quent::nvtx::SinkEvent;
use quent_time::{TimeUnixNanoSec, timestamp};
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

use crate::convert::{convert_with_thread_id, current_thread_id};

struct QueuedRecord {
    record: Record,
    timestamp: TimeUnixNanoSec,
    thread_id: Option<u32>,
}

enum Message {
    Record(QueuedRecord),
    Stop,
}

/// Error starting an NVTX capture.
#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("cannot start the NVTX capture worker: {0}")]
    Spawn(#[from] io::Error),
    #[error(transparent)]
    InstallHook(#[from] nvtx_injection::InstallHookError),
}

/// Forwards NVTX records through a regular observer on a dedicated worker.
///
/// Installation is one-shot per process, including after this value is dropped.
/// On normal completion, dropping the capture closes its queue, drains accepted
/// records, and releases its observer. The exporter flushes when the final
/// observer owner is dropped. Later calls and calls racing with shutdown may
/// be discarded.
pub struct Capture {
    sender: UnboundedSender<Message>,
    worker: Option<JoinHandle<()>>,
}

impl Capture {
    /// Creates an NVTX instance linked to `process_entity` and installs capture.
    ///
    /// Emits `Initialized` before captured events, which retain their callback
    /// timestamp and thread ID. The process reference is not validated. The capture
    /// owns `observer` until queued records have drained.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker cannot start or an NVTX hook was already installed.
    pub fn install<E>(
        observer: Observer<E>,
        process_entity: EntityRef<AnyEntity>,
    ) -> Result<Self, CaptureError>
    where
        E: InstrumentedEntity<Payload = SinkEvent> + 'static,
    {
        let instance = Uuid::now_v7();
        let initialized = Event::new(
            instance,
            timestamp(),
            SinkEvent::Initialized {
                process: process_entity,
            },
        );
        let (sender, receiver) = mpsc::unbounded_channel();
        let (start_sender, start_receiver) = std::sync::mpsc::channel();
        let worker = thread::Builder::new()
            .name("quent-nvtx-bridge".into())
            .spawn(move || {
                // Failed installations must not emit an initialized instance.
                if start_receiver.recv() == Ok(true) {
                    observer.forward(initialized);
                    forward(receiver, observer, instance);
                }
            })?;

        let capture = Self {
            sender: sender.clone(),
            worker: Some(worker),
        };
        let installed = nvtx_injection::install_hook(move |record| {
            if !sender.is_closed() {
                let timestamp = timestamp();
                let thread_id =
                    matches!(&record, Record::RangePush { .. } | Record::RangePop { .. })
                        .then(current_thread_id);
                let _ = sender.send(Message::Record(QueuedRecord {
                    record,
                    timestamp,
                    thread_id,
                }));
            }
        });
        let _ = start_sender.send(installed.is_ok());
        installed?;

        Ok(capture)
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Stop);
        if let Some(worker) = self.worker.take() {
            // Releasing the final observer owner on the worker flushes its exporter.
            // Joining from that exporter can deadlock. A worker panic is ignored
            // here and may leave records unflushed.
            let _ = worker.join();
        }
    }
}

fn forward<E>(mut receiver: UnboundedReceiver<Message>, observer: Observer<E>, instance: Uuid)
where
    E: InstrumentedEntity<Payload = SinkEvent>,
{
    let mut batch = Vec::new();
    loop {
        // Drain the current backlog without growing one batch indefinitely if
        // producers keep sending.
        let limit = receiver.len().max(1);
        if receiver.blocking_recv_many(&mut batch, limit) == 0 {
            break;
        }
        for message in batch.drain(..) {
            match message {
                Message::Record(queued) => {
                    observer.forward(Event::new(
                        instance,
                        queued.timestamp,
                        convert_with_thread_id(queued.record, queued.thread_id),
                    ));
                }
                Message::Stop => {
                    // The installed hook retains a sender for the process
                    // lifetime, so shutdown must close the receiver explicitly.
                    receiver.close();
                }
            }
        }
    }
}
