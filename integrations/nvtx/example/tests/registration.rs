// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! A failed capture owner must not disable the process's successful owner.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use quent_instrumentation::EventCallback;
use uuid::Uuid;

#[test]
fn failed_capture_does_not_disable_existing_owner() {
    let calls = Arc::new(AtomicUsize::new(0));
    let capture = nvtx_injection::install_hook({
        let calls = Arc::clone(&calls);
        move |_| {
            calls.fetch_add(1, Ordering::Relaxed);
        }
    })
    .unwrap();
    nvtx::mark(c"before duplicate");
    assert_eq!(calls.load(Ordering::Relaxed), 1);

    let result = nvtx_example::run_capture(Uuid::now_v7(), EventCallback::new(|_| {}));
    assert!(result.unwrap_err().is::<nvtx_injection::InstallHookError>());

    nvtx::mark(c"after duplicate");
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    drop(capture);
    nvtx::mark(c"after shutdown");
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}
