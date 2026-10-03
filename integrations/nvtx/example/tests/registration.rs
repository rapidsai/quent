// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! A failed hook installation must not replace the existing hook.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use quent_instrumentation::EventCallback;
use uuid::Uuid;

#[test]
fn failed_installation_does_not_replace_existing_hook() {
    let calls = Arc::new(AtomicUsize::new(0));
    let weak_calls = Arc::downgrade(&calls);
    nvtx_injection::install_hook({
        let calls = weak_calls.clone();
        move |_| {
            if let Some(calls) = calls.upgrade() {
                calls.fetch_add(1, Ordering::Relaxed);
            }
        }
    })
    .unwrap();
    nvtx::mark(c"before duplicate");
    assert_eq!(calls.load(Ordering::Relaxed), 1);

    let exporter_lifetime = Arc::new(());
    let weak_exporter = Arc::downgrade(&exporter_lifetime);
    let result = nvtx_example::run_capture(
        Uuid::now_v7(),
        EventCallback::new(move |_| {
            let _keep_alive = &exporter_lifetime;
        }),
    );
    assert!(matches!(
        result
            .unwrap_err()
            .downcast_ref::<nvtx_bridge::CaptureError>(),
        Some(nvtx_bridge::CaptureError::Hook(_))
    ));
    assert!(weak_exporter.upgrade().is_none());

    nvtx::mark(c"after duplicate");
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    drop(calls);
    nvtx::mark(c"after shutdown");
    assert!(weak_calls.upgrade().is_none());
}
