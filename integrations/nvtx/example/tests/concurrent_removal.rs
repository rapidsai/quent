// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Idle thread caches must not retain a removed hook.

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

thread_local! {
    static BLOCK_INDEX: Cell<usize> = const { Cell::new(usize::MAX) };
}

#[test]
fn removal_releases_the_hook_after_the_last_concurrent_call() {
    const THREADS: usize = 4;
    const TIMEOUT: Duration = Duration::from_secs(10);
    let calls = Arc::new(AtomicUsize::new(0));
    let released = Arc::new(AtomicUsize::new(0));
    let retained = Arc::new(());
    let weak = Arc::downgrade(&retained);
    let (entered_tx, entered_rx) = mpsc::channel();
    let capture = nvtx_injection::install_hook({
        let calls = Arc::clone(&calls);
        let released = Arc::clone(&released);
        move |_| {
            std::hint::black_box(&retained);
            calls.fetch_add(1, Ordering::Relaxed);
            BLOCK_INDEX.with(|index| {
                let index = index.get();
                if index < THREADS {
                    entered_tx.send(index).unwrap();
                    while released.load(Ordering::Acquire) & (1 << index) == 0 {
                        std::thread::yield_now();
                    }
                }
            });
        }
    })
    .unwrap();

    // Keep an idle cache alive on the main thread and retire other producers.
    nvtx::mark(c"idle producer");
    for _ in 0..16 {
        std::thread::spawn(|| nvtx::mark(c"exiting producer"))
            .join()
            .unwrap();
    }

    let (finished_tx, finished_rx) = mpsc::channel();
    let mut workers = Vec::new();
    let mut exits = Vec::new();
    for index in 0..THREADS {
        let (exit_tx, exit_rx) = mpsc::channel();
        exits.push(exit_tx);
        let finished_tx = finished_tx.clone();
        workers.push(std::thread::spawn(move || {
            BLOCK_INDEX.with(|slot| slot.set(index));
            nvtx::mark(c"in flight");
            nvtx::mark(c"after removal on cached producer");
            finished_tx.send(index).unwrap();
            exit_rx.recv_timeout(TIMEOUT).unwrap();
        }));
    }
    for _ in 0..THREADS {
        entered_rx.recv_timeout(TIMEOUT).unwrap();
    }
    let before_removal = calls.load(Ordering::Relaxed);
    let (dropped_tx, dropped_rx) = mpsc::channel();
    let dropper = std::thread::spawn(move || {
        drop(capture);
        dropped_tx.send(()).unwrap();
    });
    dropped_rx
        .recv_timeout(TIMEOUT)
        .expect("guard drop waited for in-flight callbacks");
    dropper.join().unwrap();

    nvtx::mark(c"after removal on idle producer");
    std::thread::spawn(|| nvtx::mark(c"after removal on new producer"))
        .join()
        .unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), before_removal);

    released.store((1 << (THREADS - 1)) - 1, Ordering::Release);
    for _ in 0..THREADS - 1 {
        finished_rx.recv_timeout(TIMEOUT).unwrap();
    }
    assert!(weak.upgrade().is_some(), "last in-flight hook was released");
    released.store((1 << THREADS) - 1, Ordering::Release);
    finished_rx.recv_timeout(TIMEOUT).unwrap();
    // Every producer remains alive here, so TLS caches must hold only weak refs.
    assert!(weak.upgrade().is_none(), "idle caches retained the hook");
    assert_eq!(calls.load(Ordering::Relaxed), before_removal);
    assert!(nvtx_injection::install_hook(|_| unreachable!()).is_err());

    for exit in exits {
        exit.send(()).unwrap();
    }
    for worker in workers {
        worker.join().unwrap();
    }
}
