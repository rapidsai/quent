// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Capture shutdown on an NVTX-emitting current-thread runtime does not drop the observer there.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use quent_instrumentation::{ContextInner, EventCallback};
use quent_nvtx_bridge::Capture;
use quent_nvtx_events::NvtxEvent;
use uuid::Uuid;

#[test]
fn capture_drop_on_current_thread_runtime_flushes_on_worker() {
    let calls = Arc::new(AtomicUsize::new(0));
    let session = Uuid::now_v7();
    let context = ContextInner::try_new(session).unwrap();
    let exporter = EventCallback::<NvtxEvent>::new({
        let calls = Arc::clone(&calls);
        move |_| {
            calls.fetch_add(1, Ordering::Relaxed);
        }
    });
    let observer = context
        .block_on(async { context.observer::<NvtxEvent>(&exporter).await })
        .unwrap();
    let capture = Capture::install(session, observer).unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        nvtx::mark(c"current-thread callback");
        drop(capture);
    });
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}
