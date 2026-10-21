// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX emitted by an exporter returns through the bridge without blocking it.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

mod fixtures;

#[test]
fn exporter_emitted_nvtx_does_not_deadlock_shutdown() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (done, finished) = mpsc::channel();
    let (context, capture) = fixtures::start({
        let calls = Arc::clone(&calls);
        move |_| {
            if calls.fetch_add(1, Ordering::Relaxed) == 0 {
                nvtx::mark(c"exporter mark");
            } else {
                done.send(()).unwrap();
            }
        }
    });

    nvtx::mark(c"initial mark");
    finished
        .recv_timeout(Duration::from_secs(10))
        .expect("exporter NVTX did not return through the bridge");
    drop(context);
    drop(capture);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}
