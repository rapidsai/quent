// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Completed sends are flushed even when other producers race with shutdown.

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use nvtx_bridge::{NvtxCapture, NvtxEventEntity};
use nvtx_events::{NvtxEvent, NvtxMessage};
use quent_instrumentation::{ContextInner, EventCallback};
use uuid::Uuid;

#[test]
fn completed_sends_flush_and_late_calls_are_harmless() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let exporter_lifetime = Arc::new(());
    let weak_exporter = Arc::downgrade(&exporter_lifetime);
    let exporter = EventCallback::<NvtxEventEntity>::new({
        let events = Arc::clone(&events);
        move |event| {
            let _keep_alive = &exporter_lifetime;
            events
                .lock()
                .unwrap()
                .push((event.id, event.timestamp, event.data.0.clone()));
        }
    });
    let session = Uuid::now_v7();
    let context = ContextInner::try_new(session).unwrap();
    let observer = context
        .block_on(async { context.observer::<NvtxEventEntity>(&exporter).await })
        .unwrap();
    let capture = NvtxCapture::new(session, observer).unwrap();
    drop(exporter);
    drop(context);

    let before = quent_time::timestamp();
    let producers: Vec<_> = (0..4)
        .map(|thread| {
            std::thread::spawn(move || {
                for sequence in 0..1000 {
                    let message = CString::new(format!("{thread}:{sequence}")).unwrap();
                    nvtx::mark(message.as_c_str());
                }
            })
        })
        .collect();
    for producer in producers {
        producer.join().unwrap();
    }
    let after = quent_time::timestamp();

    let stop = Arc::new(AtomicBool::new(false));
    let (started_tx, started_rx) = mpsc::channel();
    let racing = std::thread::spawn({
        let stop = Arc::clone(&stop);
        move || {
            nvtx::mark(c"racing");
            started_tx.send(()).unwrap();
            while !stop.load(Ordering::Relaxed) {
                nvtx::mark(c"racing");
            }
            nvtx::mark(c"late on registered thread");
        }
    });
    started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let (done_tx, done_rx) = mpsc::channel();
    let shutdown = std::thread::spawn(move || {
        drop(capture);
        done_tx.send(()).unwrap();
    });
    let result = done_rx.recv_timeout(Duration::from_secs(10));
    stop.store(true, Ordering::Relaxed);
    racing.join().unwrap();
    result.expect("capture shutdown did not finish");
    shutdown.join().unwrap();
    assert!(weak_exporter.upgrade().is_none());

    let captured = events.lock().unwrap().len();
    nvtx::mark(c"late on unregistered thread");
    let events = events.lock().unwrap();
    assert_eq!(events.len(), captured);
    let mut next = [0; 4];
    for (id, timestamp, event) in events.iter() {
        assert_eq!(*id, session);
        let NvtxEvent::Mark { attributes, .. } = event else {
            panic!("unexpected event")
        };
        let Some(NvtxMessage::String(message)) = &attributes.message else {
            panic!("unexpected message")
        };
        if message == "racing" {
            continue;
        }
        let (thread, sequence) = message.split_once(':').unwrap();
        let thread: usize = thread.parse().unwrap();
        let sequence: usize = sequence.parse().unwrap();
        assert_eq!(sequence, next[thread]);
        next[thread] += 1;
        assert!(
            (before..=after).contains(timestamp),
            "timestamp was assigned after capture"
        );
    }
    assert_eq!(next, [1000; 4]);
    assert!(nvtx_injection::install_hook(|_| unreachable!()).is_err());
}
