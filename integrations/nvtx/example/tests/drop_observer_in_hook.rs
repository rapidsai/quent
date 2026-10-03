// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Dropping the observer's owner inside the hook must not prevent its final flush.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use nvtx_bridge::NvtxEventEntity;
use quent_instrumentation::{ContextInner, EventCallback};
use uuid::Uuid;

#[test]
fn observer_owner_can_be_dropped_inside_hook() {
    let calls = Arc::new(AtomicUsize::new(0));
    let exporter = EventCallback::<NvtxEventEntity>::new({
        let calls = Arc::clone(&calls);
        move |_| {
            calls.fetch_add(1, Ordering::Relaxed);
        }
    });
    let session = Uuid::now_v7();
    let context = ContextInner::try_new(session).unwrap();
    let observer = Arc::new(
        context
            .block_on(async { context.observer::<NvtxEventEntity>(&exporter).await })
            .unwrap(),
    );
    let weak_observer = Arc::downgrade(&observer);
    let owner = Arc::new(Mutex::new(Some(observer)));
    nvtx_injection::install_hook({
        let observer = weak_observer.clone();
        let owner = Arc::downgrade(&owner);
        move |event| {
            if let Some(observer) = observer.upgrade() {
                let owner = owner.upgrade().unwrap();
                drop(owner.lock().unwrap().take());
                observer.emit(session, event);
            }
        }
    })
    .unwrap();

    let (done, finished) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        nvtx::mark(c"drops the observer owner");
        done.send(()).unwrap();
    });
    finished
        .recv_timeout(Duration::from_secs(10))
        .expect("observer drop inside hook deadlocked");
    worker.join().unwrap();
    assert!(weak_observer.upgrade().is_none());
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    nvtx::mark(c"after shutdown");
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}
