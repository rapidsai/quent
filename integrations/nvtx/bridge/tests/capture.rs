// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Barrier, Mutex};

use quent_nvtx_sink::quent::nvtx::SinkEvent;

mod fixtures;

#[test]
fn queued_pushes_and_pops_preserve_the_emitting_threads_ids() {
    const N_THREADS: usize = 4;
    let collected = Arc::new(Mutex::new(Vec::<SinkEvent>::new()));
    let (context, capture) = fixtures::start({
        let collected = Arc::clone(&collected);
        move |event| collected.lock().unwrap().push(event.data)
    });
    let barrier = Arc::new(Barrier::new(N_THREADS));
    let workers: Vec<_> = (0..N_THREADS)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let range = nvtx::LocalRange::new(c"worker");
                drop(range);
                // SAFETY: SYS_gettid takes no arguments and returns this thread's OS ID.
                unsafe { libc::syscall(libc::SYS_gettid) as u32 }
            })
        })
        .collect();
    let mut expected: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    drop(context);
    drop(capture);

    let mut pushes = Vec::new();
    let mut pops = Vec::new();
    for event in collected.lock().unwrap().iter() {
        match event {
            SinkEvent::RangePush { data } => pushes.push(data.thread_id),
            SinkEvent::RangePop { data } => pops.push(data.thread_id),
            _ => {}
        }
    }
    expected.sort_unstable();
    pushes.sort_unstable();
    pops.sort_unstable();
    assert_eq!(pushes, expected);
    assert_eq!(pops, expected);
}
