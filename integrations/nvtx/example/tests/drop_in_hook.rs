// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Dropping the capture guard from inside the hook must not wait on itself.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use nvtx_injection::CaptureGuard;

#[test]
fn guard_dropped_inside_hook_does_not_deadlock() {
    let calls = Arc::new(AtomicUsize::new(0));
    let slot: Arc<Mutex<Option<CaptureGuard>>> = Arc::new(Mutex::new(None));
    let capture = nvtx_injection::install_hook({
        let calls = Arc::clone(&calls);
        let slot = Arc::clone(&slot);
        move |_| {
            calls.fetch_add(1, Ordering::Relaxed);
            drop(slot.lock().unwrap().take());
        }
    })
    .unwrap();
    *slot.lock().unwrap() = Some(capture);

    // Run on another thread so a deadlock fails the test instead of hanging it.
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        nvtx::mark(c"drops the guard");
        done.send(()).unwrap();
    });
    finished
        .recv_timeout(Duration::from_secs(10))
        .expect("guard drop inside the hook deadlocked");
    assert_eq!(calls.load(Ordering::Relaxed), 1);

    nvtx::mark(c"after shutdown");
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}
