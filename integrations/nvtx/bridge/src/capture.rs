// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Queued NVTX capture with an application-owned lifetime.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use nvtx_injection::RawEvent;
use quent_channel::{Config, unbounded_channel};
use quent_events::Event;
use quent_instrumentation::ObserverInner;
use quent_time::{TimeUnixNanoSec, timestamp};
use uuid::Uuid;

use crate::NvtxEventEntity;

struct Record {
    timestamp: TimeUnixNanoSec,
    event: RawEvent,
}

/// An error starting NVTX capture.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error(transparent)]
    Hook(#[from] nvtx_injection::InstallHookError),
    #[error("could not start NVTX worker: {0}")]
    Worker(#[from] std::io::Error),
}

/// Owns queued NVTX forwarding and flushes its observer when dropped.
///
/// Idle delivery polls every millisecond. Drop blocks until completed sends
/// have been forwarded and the observer has been released. Events racing with
/// shutdown may be discarded. Do not drop this from its exporter's callbacks.
pub struct NvtxCapture {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl NvtxCapture {
    /// Install capture for one entity instance, taking ownership of its observer.
    ///
    /// Callbacks copy NVTX data and record timestamps on the emitting thread.
    /// A worker converts records into Quent events. Installation remains one-shot
    /// after shutdown, and late callbacks cannot retain the observer or exporter.
    ///
    /// # Errors
    /// Returns an error if a hook is already installed or the worker cannot start.
    pub fn new(id: Uuid, observer: ObserverInner<NvtxEventEntity>) -> Result<Self, CaptureError> {
        let (sender, mut receiver) = unbounded_channel::<Record>(Config::default());
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("nvtx-forwarder".into())
            .spawn(move || {
                let mut buffer = Vec::with_capacity(256);
                loop {
                    let stopping = worker_stop.load(Ordering::Acquire);
                    if stopping {
                        receiver.close();
                    }
                    let report = receiver.drain_into(&mut buffer, 256);
                    for record in buffer.drain(..) {
                        observer.send(Event::new(id, record.timestamp, record.event.into()));
                    }
                    if stopping && !report.pending {
                        break;
                    }
                    if report.drained == 0 && !stopping {
                        thread::park_timeout(Duration::from_millis(1));
                    }
                }
                // Flush only after every completed send has reached the observer.
                drop(receiver);
                drop(observer);
            })?;
        let capture = Self {
            stop,
            worker: Some(worker),
        };
        nvtx_injection::install_hook(move |event| {
            let _ = sender.send(Record {
                timestamp: timestamp(),
                event,
            });
        })?;
        Ok(capture)
    }
}

impl Drop for NvtxCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}
