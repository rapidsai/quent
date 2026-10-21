// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Capture shutdown while other threads are emitting NVTX events.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

mod fixtures;

#[test]
fn concurrent_shutdown_flushes_queued_events_and_ignores_late_calls() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (context, capture) = fixtures::start({
        let calls = Arc::clone(&calls);
        move |_| {
            calls.fetch_add(1, Ordering::Relaxed);
        }
    });

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
    drop(context);
    drop(capture);
    producer.join().unwrap();

    let flushed = calls.load(Ordering::Relaxed);
    assert!(flushed >= 1);
    nvtx::mark(c"after shutdown");
    assert_eq!(calls.load(Ordering::Relaxed), flushed);
}
