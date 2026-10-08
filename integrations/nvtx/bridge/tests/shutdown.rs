// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Capture shutdown while other threads are emitting NVTX events.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

use quent_instrumentation::{ContextInner, EventCallback};
use quent_nvtx_bridge::Capture;
use quent_nvtx_events::NvtxEvent;
use uuid::Uuid;

#[test]
fn concurrent_shutdown_flushes_queued_events_and_ignores_late_calls() {
    let calls = Arc::new(AtomicUsize::new(0));
    let exporter = EventCallback::<NvtxEvent>::new({
        let calls = Arc::clone(&calls);
        move |_| {
            calls.fetch_add(1, Ordering::Relaxed);
        }
    });
    let session = Uuid::now_v7();
    let context = ContextInner::try_new(session).unwrap();
    let observer = context
        .block_on(async { context.observer::<NvtxEvent>(&exporter).await })
        .unwrap();
    let capture = Capture::install(session, observer).unwrap();

    nvtx::mark(c"before shutdown");
    let barrier = Arc::new(Barrier::new(2));
    let producer = std::thread::spawn({
        let barrier = Arc::clone(&barrier);
        move || {
            barrier.wait();
            for _ in 0..1_000 {
                nvtx::mark(c"concurrent");
            }
        }
    });
    barrier.wait();
    drop(capture);
    producer.join().unwrap();

    let flushed = calls.load(Ordering::Relaxed);
    assert!(flushed >= 1);
    nvtx::mark(c"after shutdown");
    assert_eq!(calls.load(Ordering::Relaxed), flushed);
}
