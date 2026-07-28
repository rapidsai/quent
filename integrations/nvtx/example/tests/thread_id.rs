// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! End-to-end proof that captured Push/Pop ranges carry a real OS thread id.
//!
//! Reuses the crate's [`run_capture`](nvtx_example::run_capture) with a
//! collecting callback exporter and asserts every captured `RangePush`/`RangePop`
//! carries a nonzero `thread_id`, and that Push/Pop emitted from the single
//! example thread share one id. No GPU, no subprocess, no files.
//!
//! This lives in its own test file (hence its own test binary/process) rather
//! than in `capture.rs`: `nvtx_injection::install_hook` is one-shot per process,
//! so two `run_capture` calls in the same binary would collide.

use std::sync::{Arc, Mutex};

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::NvtxEvent;
use quent_instrumentation::{Event, EventCallback};
use uuid::Uuid;

#[test]
fn pushpop_carry_thread_id() {
    // Collect every captured event in memory via a callback exporter.
    let collected: Arc<Mutex<Vec<NvtxEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let collected = Arc::clone(&collected);
        EventCallback::new(move |recorded| {
            if let Some(event) = recorded.event.downcast_ref::<Event<NvtxEventEntity>>() {
                collected.lock().unwrap().push(event.data.0.clone());
            }
        })
    };

    nvtx_example::run_capture(Uuid::now_v7(), sink).expect("capture");

    let events = collected.lock().unwrap();

    let mut thread_ids = Vec::new();
    let mut saw_push = false;
    let mut saw_pop = false;
    for event in events.iter() {
        match event {
            NvtxEvent::RangePush { thread_id, .. } => {
                assert_ne!(
                    *thread_id, 0,
                    "captured RangePush must carry a real OS thread id, not 0"
                );
                thread_ids.push(*thread_id);
                saw_push = true;
            }
            NvtxEvent::RangePop { thread_id, .. } => {
                assert_ne!(
                    *thread_id, 0,
                    "captured RangePop must carry a real OS thread id, not 0"
                );
                thread_ids.push(*thread_id);
                saw_pop = true;
            }
            _ => {}
        }
    }

    assert!(saw_push, "expected at least one captured RangePush");
    assert!(saw_pop, "expected at least one captured RangePop");

    // The example drives every Push/Pop from a single thread, so all stamped ids
    // must match — proving a push and its matching pop carry the same thread_id.
    let first = thread_ids[0];
    assert!(
        thread_ids.iter().all(|&t| t == first),
        "all Push/Pop from the single example thread must share one thread_id; got {thread_ids:?}"
    );
}
