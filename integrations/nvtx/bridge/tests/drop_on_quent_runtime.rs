// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Capture shutdown must allow its exporter task to finish, even with one worker.

use std::num::NonZeroUsize;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use quent_instrumentation::{ContextInner, EventCallback, RuntimeOptions};
use quent_nvtx_bridge::Capture;
use quent_nvtx_events::NvtxEvent;
use uuid::Uuid;

// Each capture needs a fresh process because the NVTX hook is one-shot. Kill
// and reap a stuck child so a deadlock fails the test instead of hanging CI.
fn isolated(name: &str, run: impl FnOnce()) {
    const CHILD: &str = "QUENT_NVTX_SHUTDOWN_TEST";
    if std::env::var(CHILD).as_deref() == Ok(name) {
        run();
        return;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env(CHILD, name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().unwrap().is_some() {
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "{name} failed: {}\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            return;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "{name} did not finish within 10 seconds:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn single_worker_context(session: Uuid) -> ContextInner {
    let mut options = RuntimeOptions::default();
    options.worker_threads = NonZeroUsize::new(1);
    ContextInner::try_new_with_options(session, options).unwrap()
}

#[test]
fn capture_drop_on_quent_worker_flushes() {
    isolated("capture_drop_on_quent_worker_flushes", || {
        let calls = Arc::new(AtomicUsize::new(0));
        let session = Uuid::now_v7();
        let context = single_worker_context(session);
        let exporter = EventCallback::<NvtxEvent>::new({
            let calls = Arc::clone(&calls);
            move |_| {
                calls.fetch_add(1, Ordering::Relaxed);
            }
        });
        let observer = context.block_on(context.observer(&exporter)).unwrap();
        let capture = Capture::install(session, observer).unwrap();

        context.block_on(async move {
            // block_on itself runs on the caller. Spawn onto the same runtime
            // as the exporter to exercise contention for its only worker.
            tokio::spawn(async move {
                nvtx::mark(c"before capture drop on Quent worker");
                drop(capture);
                assert_eq!(calls.load(Ordering::Relaxed), 1);
            })
            .await
            .unwrap();
        });
    });
}

struct NotifyOnDrop(mpsc::Sender<()>);

impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        self.0.send(()).unwrap();
    }
}

#[test]
fn capture_drop_in_exporter_drains_and_shuts_down() {
    isolated("capture_drop_in_exporter_drains_and_shuts_down", || {
        let calls = Arc::new(AtomicUsize::new(0));
        let slot = Arc::new(Mutex::new(None::<Capture>));
        let (done, finished) = mpsc::channel();
        let shutdown = NotifyOnDrop(done);
        let exporter = EventCallback::<NvtxEvent>::new({
            let calls = Arc::clone(&calls);
            let slot = Arc::clone(&slot);
            move |_| {
                // The final callback owner is dropped when the exporter shuts
                // down, after all queued events have been delivered.
                let _shutdown = &shutdown;
                if calls.fetch_add(1, Ordering::Relaxed) == 0 {
                    nvtx::mark(c"queued before capture drop in exporter");
                    let capture = slot.lock().unwrap().take().unwrap();
                    drop(capture);
                }
            }
        });
        let session = Uuid::now_v7();
        let context = single_worker_context(session);
        let observer = context.block_on(context.observer(&exporter)).unwrap();
        *slot.lock().unwrap() = Some(Capture::install(session, observer).unwrap());
        drop(exporter);
        // The capture's observer must keep its runtime alive through shutdown.
        drop(context);

        nvtx::mark(c"drop capture in exporter");
        finished.recv().unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        nvtx::mark(c"after exporter shutdown");
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    });
}
