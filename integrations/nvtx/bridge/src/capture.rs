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
use quent_events::Event;
use quent_instrumentation::ObserverInner;
use quent_nvtx_events::NvtxEvent;
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

/// Forwards NVTX records and flushes the observer on a dedicated worker.
///
/// Installation is one-shot per process, including after this value is dropped.
/// On normal completion, dropping the capture closes its queue, drains accepted
/// records, and waits for the observer's exporter to flush. Later calls and
/// calls racing with shutdown may be discarded.
pub struct Capture {
    sender: UnboundedSender<Message>,
    worker: Option<JoinHandle<()>>,
}

impl Capture {
    /// Install the NVTX hook and start forwarding records to `observer`.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker cannot start or an NVTX hook was already installed.
    pub fn install(
        session: Uuid,
        observer: ObserverInner<NvtxEvent>,
    ) -> Result<Self, CaptureError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let worker = thread::Builder::new()
            .name("quent-nvtx-bridge".into())
            .spawn(move || forward(receiver, observer, session))?;

        let capture = Self {
            sender: sender.clone(),
            worker: Some(worker),
        };
        nvtx_injection::install_hook(move |record| {
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
        })?;

        Ok(capture)
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Stop);
        if let Some(worker) = self.worker.take() {
            // Joining also waits for the worker-owned observer to flush. Doing
            // so on its exporter or Quent runtime worker can deadlock. A worker
            // panic is ignored here and may leave records unflushed.
            let _ = worker.join();
        }
    }
}

fn forward(
    mut receiver: UnboundedReceiver<Message>,
    observer: ObserverInner<NvtxEvent>,
    session: Uuid,
) {
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
                    observer.send(Event::new(
                        session,
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

#[cfg(test)]
mod tests {
    use std::sync::mpsc as std_mpsc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn shutdown_does_not_wait_for_the_hook_sender() {
        let (sender, receiver) = mpsc::unbounded_channel();
        let hook_sender = sender.clone();
        let worker = thread::spawn(move || {
            forward(receiver, ObserverInner::noop(), Uuid::now_v7());
        });
        let capture = Capture {
            sender,
            worker: Some(worker),
        };
        let (done, finished) = std_mpsc::channel();
        let shutdown = thread::spawn(move || {
            drop(capture);
            done.send(()).unwrap();
        });
        finished
            .recv_timeout(Duration::from_secs(10))
            .expect("shutdown waited for the hook sender");
        shutdown.join().unwrap();
        assert!(hook_sender.send(Message::Stop).is_err());
    }
}
