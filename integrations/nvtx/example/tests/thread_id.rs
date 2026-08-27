// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! End-to-end proof that captured Push/Pop ranges carry real, per-thread OS ids.
//!
//! Spawns four threads via [`nvtx_example::run_capture_n_threads`], each
//! emitting one `RangePush`/`RangePop` pair, and asserts:
//!
//! 1. Every stamped `thread_id` is nonzero.
//! 2. Each thread's Push and Pop share the same `thread_id`.
//! 3. All four threads produce **distinct** `thread_id`s — proving that
//!    `gettid` returns per-thread values, not a single global constant.
//!
//! This lives in its own test file (hence its own binary/process) because
//! `nvtx_injection::install_hook` is one-shot per process.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::NvtxEvent;
use quent_instrumentation::EventCallback;
use uuid::Uuid;

const N_THREADS: usize = 4;

#[test]
fn pushpop_four_threads_get_distinct_ids() {
    let collected: Arc<Mutex<Vec<NvtxEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let collected = Arc::clone(&collected);
        EventCallback::<NvtxEventEntity>::new(move |event| {
            collected.lock().unwrap().push(event.data.0.clone());
        })
    };

    nvtx_example::run_capture_n_threads(N_THREADS, Uuid::now_v7(), sink).expect("capture");

    let events = collected.lock().unwrap();

    // Count Push and Pop events keyed by thread_id.
    let mut pushes: HashMap<u32, usize> = HashMap::new();
    let mut pops: HashMap<u32, usize> = HashMap::new();

    for event in events.iter() {
        match event {
            NvtxEvent::RangePush { thread_id, .. } => {
                assert_ne!(*thread_id, 0, "RangePush must carry a nonzero OS thread id");
                *pushes.entry(*thread_id).or_insert(0) += 1;
            }
            NvtxEvent::RangePop { thread_id, .. } => {
                assert_ne!(*thread_id, 0, "RangePop must carry a nonzero OS thread id");
                *pops.entry(*thread_id).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    // Each of the N threads emits exactly one Push and one Pop.
    assert_eq!(
        pushes.len(),
        N_THREADS,
        "expected {N_THREADS} distinct thread_ids on RangePush events, got {:?}",
        pushes.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        pops.len(),
        N_THREADS,
        "expected {N_THREADS} distinct thread_ids on RangePop events, got {:?}",
        pops.keys().collect::<Vec<_>>()
    );

    // The set of Push thread_ids must equal the set of Pop thread_ids — each
    // thread's push and pop carry the same id.
    let push_ids: HashSet<u32> = pushes.keys().copied().collect();
    let pop_ids: HashSet<u32> = pops.keys().copied().collect();
    assert_eq!(
        push_ids, pop_ids,
        "Push and Pop thread_ids must come from the same {N_THREADS} threads"
    );
}
