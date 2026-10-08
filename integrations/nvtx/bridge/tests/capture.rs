// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exercise the opt-in bridge converter with real NVTX calls and a Quent observer.

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::sync::{Arc, Barrier, Mutex};

use quent_instrumentation::{ContextInner, EventCallback};
use quent_nvtx_bridge::Capture;
use quent_nvtx_events::NvtxEvent;
use uuid::Uuid;

const N_THREADS: usize = 4;

#[test]
fn captures_core_events_and_preserves_calling_thread_ids() {
    let collected = Arc::new(Mutex::new(Vec::<NvtxEvent>::new()));
    let exporter = EventCallback::<NvtxEvent>::new({
        let collected = Arc::clone(&collected);
        move |event| collected.lock().unwrap().push(event.data.clone())
    });
    let session = Uuid::now_v7();
    let context = ContextInner::try_new(session).unwrap();
    let observer = context
        .block_on(async { context.observer::<NvtxEvent>(&exporter).await })
        .unwrap();
    let capture = Capture::install(session, observer).unwrap();

    // SAFETY: SYS_gettid takes no arguments and returns this thread's OS ID.
    let thread_id = unsafe { libc::syscall(libc::SYS_gettid) as u32 };
    nvtx::name_thread(thread_id, c"capture/main");
    nvtx::mark(c"startup");
    let range = nvtx::Range::new(c"phase");
    drop(range);

    let barrier = Arc::new(Barrier::new(N_THREADS));
    let workers: Vec<_> = (0..N_THREADS)
        .map(|i| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let name = CString::new(format!("thread-{i}")).unwrap();
                let range = nvtx::LocalRange::new(name);
                drop(range);
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }

    drop(capture);
    let captured = collected.lock().unwrap().len();
    nvtx::mark(c"after observer drop");
    let events = collected.lock().unwrap();
    assert_eq!(events.len(), captured);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, NvtxEvent::NameThread { .. })),
        "missing thread name"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, NvtxEvent::Mark { .. })),
        "missing mark"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, NvtxEvent::RangeStart { .. })),
        "missing range start"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, NvtxEvent::RangeEnd { .. })),
        "missing range end"
    );

    let mut pushes = HashMap::<u32, usize>::new();
    let mut pops = HashMap::<u32, usize>::new();
    for event in events.iter() {
        match event {
            NvtxEvent::RangePush { thread_id, .. } => {
                assert_ne!(*thread_id, 0);
                *pushes.entry(*thread_id).or_default() += 1;
            }
            NvtxEvent::RangePop { thread_id, .. } => {
                assert_ne!(*thread_id, 0);
                *pops.entry(*thread_id).or_default() += 1;
            }
            _ => {}
        }
    }
    let push_ids: HashSet<_> = pushes.keys().copied().collect();
    let pop_ids: HashSet<_> = pops.keys().copied().collect();
    assert_eq!(push_ids.len(), N_THREADS);
    assert_eq!(push_ids, pop_ids);
    assert!(pushes.values().all(|count| *count == 1));
    assert!(pops.values().all(|count| *count == 1));
}
