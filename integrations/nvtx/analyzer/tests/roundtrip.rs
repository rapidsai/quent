// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! End-to-end proof against a **real** NVTX capture.
//!
//! Every other test feeds the builder a hand-built stream, so they all agree
//! with each other about what a capture looks like. This one closes that loop by
//! running the actual injection layer and reconstructing whatever comes out.
//!
//! Gated behind `real-capture-tests` because it links the real injection layer,
//! whose `nvtx-sys` dependency runs bindgen and native C compilation.
#![cfg(feature = "real-capture-tests")]

use std::sync::{Arc, Mutex};

use quent_instrumentation::{ContextInner, Event, EventCallback};
use quent_nvtx_analyzer::{NvtxModelBuilder, SpanKind, StatsKey};
use quent_nvtx_bridge::Capture;
use quent_nvtx_events::NvtxEvent;
use uuid::Uuid;

/// The default (NULL) NVTX domain used by the test annotations.
const DEFAULT_DOMAIN: u64 = 0;

#[test]
fn real_capture_roundtrip() {
    // Collect the full envelope, not just the inner event: the builder orders by
    // `timestamp`, so dropping it would make this test prove nothing about the
    // real capture's ordering.
    let collected: Arc<Mutex<Vec<Event<NvtxEvent>>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let collected = Arc::clone(&collected);
        EventCallback::<NvtxEvent>::new(move |event| {
            collected.lock().expect("collector poisoned").push(event);
        })
    };

    let session = Uuid::now_v7();
    let context = ContextInner::try_new(session).expect("context");
    let observer = context
        .block_on(async { context.observer::<NvtxEvent>(&sink).await })
        .expect("observer");
    let capture = Capture::install(session, observer).expect("install capture");

    // SAFETY: SYS_gettid takes no arguments and returns this thread's OS ID.
    let thread_id = unsafe { libc::syscall(libc::SYS_gettid) as u32 };
    nvtx::name_thread(thread_id, c"analyzer-roundtrip/main");
    nvtx::mark(c"startup");
    let local_range = nvtx::LocalRange::new(c"phase-1");
    drop(local_range);
    let range = nvtx::Range::new(c"phase-2");
    drop(range);
    drop(capture);

    // Read through the `Arc` rather than `try_unwrap`ing it: the sink's closure
    // holds a second clone. Unwrapping would fail for a reason unrelated to
    // reconstruction.
    let events = std::mem::take(&mut *collected.lock().expect("collector poisoned"));
    assert!(!events.is_empty(), "no NVTX events captured");

    let model = NvtxModelBuilder::build(events);

    // `nvtx::name_thread` — the name must reach the thread view.
    assert!(
        model
            .threads()
            .iter()
            .any(|thread| thread.name == "analyzer-roundtrip/main"),
        "named thread missing; threads: {:?}",
        model.threads()
    );

    // `nvtx::mark` — an instant, not a zero-length span.
    assert!(
        model.marks().iter().any(|mark| mark.name == "startup"),
        "mark \"startup\" missing; marks: {:?}",
        model.marks()
    );
    assert!(
        !model.spans().iter().any(|span| span.name == "startup"),
        "the mark must not have been reconstructed as a span"
    );

    // `nvtx::LocalRange` — a per-thread nested range.
    let phase1 = model
        .spans()
        .iter()
        .find(|span| span.name == "phase-1")
        .expect("span \"phase-1\" missing");
    assert!(
        matches!(phase1.kind, SpanKind::PushPop { .. }),
        "a push/pop range carries the OS thread it ran on; got {:?}",
        phase1.kind
    );
    assert!(phase1.end.is_some(), "the pop was observed");
    assert!(phase1.duration().is_some());

    // `nvtx::Range` — a process-wide start/end range.
    let phase2 = model
        .spans()
        .iter()
        .find(|span| span.name == "phase-2")
        .expect("span \"phase-2\" missing");
    assert_eq!(phase2.kind, SpanKind::StartEnd);
    assert!(phase2.end.is_some(), "the end was observed");
    assert!(phase2.duration().is_some());

    // Both ranges reach the statistics, one group each.
    let stats = model.range_statistics();
    for name in ["phase-1", "phase-2"] {
        let group = stats
            .get(&StatsKey {
                name: name.to_owned(),
                domain: DEFAULT_DOMAIN,
                category: None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "no statistics for {name:?}; groups: {:?}",
                    stats.keys().collect::<Vec<_>>()
                )
            });
        assert_eq!(group.count, 1, "one occurrence of {name:?}");
        assert_eq!(group.observed_count, 1, "{name:?} closed for real");
        assert_eq!(
            group.min_duration, group.max_duration,
            "a single occurrence bounds itself"
        );
        assert_eq!(group.avg_duration, group.total_duration);
    }
}
