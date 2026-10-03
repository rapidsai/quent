// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    num::NonZeroUsize,
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use quent_channel::{Config, unbounded_channel};

const EVENTS: usize = 100_000;
const PRODUCERS: usize = 4;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn config(capacity: usize, spares: usize) -> Config {
    Config {
        segment_capacity: NonZeroUsize::new(capacity).unwrap(),
        spare_segments: spares,
    }
}

fn report(scenario: &str, name: &str, times: &mut [u128], elapsed_ns: u128, allocations: usize) {
    times.sort_unstable();
    let at =
        |numerator: usize, denominator: usize| times[(times.len() - 1) * numerator / denominator];
    println!(
        "{scenario},{name},{},{},{},{},{:.3},{:.0},{}",
        at(1, 2),
        at(99, 100),
        at(999, 1000),
        times[times.len() - 1],
        elapsed_ns as f64 / 1_000_000.0,
        times.len() as f64 * 1_000_000_000.0 / elapsed_ns as f64,
        allocations
    );
}

fn steady(name: &str, mut push: impl FnMut(usize), mut collect: impl FnMut() -> usize) {
    let mut times = Vec::with_capacity(EVENTS);
    let allocations_before = ALLOCATIONS.load(Ordering::Relaxed);
    let overall = Instant::now();
    for value in 0..EVENTS {
        let start = Instant::now();
        push(black_box(value));
        times.push(start.elapsed().as_nanos());
        assert_eq!(collect(), value);
    }
    let elapsed_ns = overall.elapsed().as_nanos();
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - allocations_before;
    report("steady", name, &mut times, elapsed_ns, allocations);
}

fn burst(name: &str, mut push: impl FnMut(usize), mut drain: impl FnMut() -> usize) {
    let mut times = Vec::with_capacity(EVENTS);
    let allocations_before = ALLOCATIONS.load(Ordering::Relaxed);
    let overall = Instant::now();
    for value in 0..EVENTS {
        let start = Instant::now();
        push(black_box(value));
        times.push(start.elapsed().as_nanos());
    }
    let elapsed_ns = overall.elapsed().as_nanos();
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - allocations_before;
    for value in 0..EVENTS {
        assert_eq!(drain(), value);
    }
    report("burst", name, &mut times, elapsed_ns, allocations);
}

fn steady_scenarios() {
    let (mut writer, mut reader) = rtrb::RingBuffer::new(256);
    steady(
        "rtrb-256",
        |value| writer.push(value).unwrap(),
        || reader.pop().unwrap(),
    );

    for capacity in [64, 256, 1024] {
        let (sender, mut receiver) = unbounded_channel(config(capacity, 2));
        let mut output = Vec::with_capacity(1);
        sender.send(usize::MAX).unwrap();
        receiver.drain_into(&mut output, 1);
        output.clear();
        receiver.preallocate_spares();
        steady(
            &format!("quent-tls-{capacity}"),
            |value| sender.send(value).unwrap(),
            || {
                receiver.drain_into(&mut output, 1);
                output.pop().unwrap()
            },
        );
    }

    let (sender, receiver) = std::sync::mpsc::channel();
    steady(
        "std-mpsc",
        |value| sender.send(value).unwrap(),
        || receiver.try_recv().unwrap(),
    );

    let (sender, receiver) = crossbeam_channel::unbounded();
    steady(
        "crossbeam",
        |value| sender.send(value).unwrap(),
        || receiver.try_recv().unwrap(),
    );

    let (sender, receiver) = flume::unbounded();
    steady(
        "flume",
        |value| sender.send(value).unwrap(),
        || receiver.try_recv().unwrap(),
    );

    let (sender, receiver) = kanal::unbounded();
    steady(
        "kanal",
        |value| sender.send(value).unwrap(),
        || receiver.try_recv().unwrap().unwrap(),
    );

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    steady(
        "tokio",
        |value| sender.send(value).unwrap(),
        || receiver.try_recv().unwrap(),
    );
}

fn burst_scenarios() {
    let (mut writer, mut reader) = rtrb::RingBuffer::new(EVENTS);
    burst(
        "rtrb-preallocated",
        |value| writer.push(value).unwrap(),
        || reader.pop().unwrap(),
    );

    for capacity in [64, 256, 1024] {
        let (sender, mut receiver) = unbounded_channel(config(capacity, 0));
        let mut output = Vec::with_capacity(EVENTS);
        sender.send(usize::MAX).unwrap();
        receiver.drain_into(&mut output, 1);
        output.clear();
        burst(
            &format!("quent-tls-{capacity}"),
            |value| sender.send(value).unwrap(),
            || {
                if output.is_empty() {
                    receiver.drain_into(&mut output, EVENTS);
                    output.reverse();
                }
                output.pop().unwrap()
            },
        );
    }

    let (sender, receiver) = std::sync::mpsc::channel();
    burst(
        "std-mpsc",
        |value| sender.send(value).unwrap(),
        || receiver.try_recv().unwrap(),
    );

    let (sender, receiver) = crossbeam_channel::unbounded();
    burst(
        "crossbeam",
        |value| sender.send(value).unwrap(),
        || receiver.try_recv().unwrap(),
    );

    let (sender, receiver) = flume::unbounded();
    burst(
        "flume",
        |value| sender.send(value).unwrap(),
        || receiver.try_recv().unwrap(),
    );

    let (sender, receiver) = kanal::unbounded();
    burst(
        "kanal",
        |value| sender.send(value).unwrap(),
        || receiver.try_recv().unwrap().unwrap(),
    );

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    burst(
        "tokio",
        |value| sender.send(value).unwrap(),
        || receiver.try_recv().unwrap(),
    );
}

fn parallel<S: Clone + Send>(
    name: &str,
    sender: S,
    push: impl Fn(&S, usize) + Copy + Send,
    mut collect: impl FnMut() -> usize,
) {
    let barrier = Arc::new(Barrier::new(PRODUCERS + 1));
    let mut times = Vec::with_capacity(EVENTS);
    let allocations_before = ALLOCATIONS.load(Ordering::Relaxed);
    std::thread::scope(|scope| {
        let mut workers = Vec::with_capacity(PRODUCERS);
        for worker in 0..PRODUCERS {
            let sender = sender.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(scope.spawn(move || {
                let mut samples = Vec::with_capacity(EVENTS / PRODUCERS);
                barrier.wait();
                for index in 0..EVENTS / PRODUCERS {
                    let start = Instant::now();
                    push(&sender, worker * (EVENTS / PRODUCERS) + index);
                    samples.push(start.elapsed().as_nanos());
                }
                samples
            }));
        }
        barrier.wait();
        let overall = Instant::now();
        let mut received = 0;
        while received < EVENTS {
            let count = collect();
            received += count;
            if count == 0 {
                std::hint::spin_loop();
            }
        }
        let elapsed_ns = overall.elapsed().as_nanos();
        for worker in workers {
            times.extend(worker.join().unwrap());
        }
        let allocations = ALLOCATIONS.load(Ordering::Relaxed) - allocations_before;
        report("parallel-4p", name, &mut times, elapsed_ns, allocations);
    });
}

fn parallel_scenarios() {
    let (sender, mut receiver) = unbounded_channel(config(256, 2));
    let mut output = Vec::with_capacity(256);
    parallel(
        "quent-tls-256",
        sender,
        |sender, value| sender.send(value).unwrap(),
        || {
            output.clear();
            receiver.drain_into(&mut output, 256).drained
        },
    );

    let (sender, receiver) = std::sync::mpsc::channel();
    parallel(
        "std-mpsc",
        sender,
        |sender, value| sender.send(value).unwrap(),
        || (0..256).map_while(|_| receiver.try_recv().ok()).count(),
    );

    let (sender, receiver) = crossbeam_channel::unbounded();
    parallel(
        "crossbeam",
        sender,
        |sender, value| sender.send(value).unwrap(),
        || (0..256).map_while(|_| receiver.try_recv().ok()).count(),
    );

    let (sender, receiver) = flume::unbounded();
    parallel(
        "flume",
        sender,
        |sender, value| sender.send(value).unwrap(),
        || (0..256).map_while(|_| receiver.try_recv().ok()).count(),
    );

    let (sender, receiver) = kanal::unbounded();
    parallel(
        "kanal",
        sender,
        |sender, value| sender.send(value).unwrap(),
        || {
            (0..256)
                .map_while(|_| receiver.try_recv().ok().flatten())
                .count()
        },
    );

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    parallel(
        "tokio",
        sender,
        |sender, value| sender.send(value).unwrap(),
        || (0..256).map_while(|_| receiver.try_recv().ok()).count(),
    );
}

fn first_use_scenario() {
    let mut times = Vec::with_capacity(1_000);
    let allocations_before = ALLOCATIONS.load(Ordering::Relaxed);
    let overall = Instant::now();
    for value in 0..1_000 {
        let (sender, mut receiver) = unbounded_channel(config(256, 2));
        let start = Instant::now();
        sender.send(value).unwrap();
        times.push(start.elapsed().as_nanos());
        let mut output = Vec::new();
        receiver.drain_into(&mut output, 1);
        assert_eq!(output, [value]);
    }
    let elapsed_ns = overall.elapsed().as_nanos();
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - allocations_before;
    report(
        "registration",
        "quent-tls-256",
        &mut times,
        elapsed_ns,
        allocations,
    );
}

fn main() {
    println!(
        "scenario,channel,p50_ns,p99_ns,p999_ns,max_ns,total_ms,events_per_second,allocations"
    );
    steady_scenarios();
    burst_scenarios();
    parallel_scenarios();
    first_use_scenario();
}
