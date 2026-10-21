// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Capture shutdown on an NVTX-emitting current-thread runtime does not drop the observer there.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

mod fixtures;

#[test]
fn capture_drop_on_current_thread_runtime_flushes_on_worker() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (context, capture) = fixtures::start({
        let calls = Arc::clone(&calls);
        move |_| {
            calls.fetch_add(1, Ordering::Relaxed);
        }
    });

    drop(context);

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
