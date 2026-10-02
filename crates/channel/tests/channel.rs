// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    cell::{Cell, RefCell},
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

use quent_channel::{Config, Receiver, Sender, unbounded_channel};

fn config(capacity: usize, spares: usize) -> Config {
    Config {
        segment_capacity: NonZeroUsize::new(capacity).unwrap(),
        spare_segments: spares,
    }
}

fn drain_n<T: Send + 'static>(receiver: &mut Receiver<T>, output: &mut Vec<T>, count: usize) {
    let target = output.len() + count;
    while output.len() < target {
        assert_ne!(
            receiver.drain_into(output, target - output.len()).drained,
            0
        );
    }
}

#[test]
fn carries_owning_values_in_fifo_order_across_segments() {
    let (sender, mut receiver) = unbounded_channel::<String>(config(2, 0));
    for i in 0..100 {
        sender.send(i.to_string()).unwrap();
    }
    receiver.close();
    let mut values = Vec::new();
    while receiver.drain_into(&mut values, 3).pending {}
    assert_eq!(values, (0..100).map(|i| i.to_string()).collect::<Vec<_>>());
    assert_eq!(receiver.metrics().in_flight, 1);
}

#[test]
fn recycles_preallocated_segments_and_trims_excess() {
    let (sender, mut receiver) = unbounded_channel(config(2, 2));
    sender.send(usize::MAX).unwrap();
    let mut output = Vec::new();
    receiver.drain_into(&mut output, 1);
    output.clear();
    assert_eq!(receiver.preallocate_spares(), 2);
    assert_eq!(receiver.metrics().allocated, 3);
    for value in 0..6 {
        sender.send(value).unwrap();
    }
    drain_n(&mut receiver, &mut output, 6);
    assert_eq!(receiver.metrics().spares, 2);
    assert_eq!(receiver.metrics().allocated, 3);
    for value in 6..10 {
        sender.send(value).unwrap();
    }
    assert_eq!(receiver.metrics().allocated, 3);
    drain_n(&mut receiver, &mut output, 4);
    assert_eq!(output, (0..10).collect::<Vec<_>>());

    for value in 10..30 {
        sender.send(value).unwrap();
    }
    assert!(receiver.metrics().allocated > 3);
    drain_n(&mut receiver, &mut output, 20);
    assert_eq!(receiver.metrics().allocated, 3);
    assert_eq!(receiver.metrics().spares, 2);
}

#[derive(Debug)]
struct Counted(Arc<AtomicUsize>);

impl Drop for Counted {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn early_consumer_drop_reclaims_every_payload_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = unbounded_channel(config(2, 0));
    for _ in 0..100 {
        sender.send(Counted(Arc::clone(&drops))).unwrap();
    }
    drop(receiver);
    let undeliverable = loop {
        match sender.send(Counted(Arc::clone(&drops))) {
            Ok(()) => continue,
            Err(value) => break value,
        }
    };
    drop(undeliverable);
    drop(sender);
    assert_eq!(drops.load(Ordering::Relaxed), 101);
}

#[test]
fn supports_send_but_not_sync_payloads() {
    let (sender, mut receiver) = unbounded_channel(config(1, 0));
    let handle = thread::spawn(move || {
        sender.send(Cell::new(42)).unwrap();
    });
    handle.join().unwrap();
    receiver.close();
    let mut output = Vec::new();
    while receiver.drain_into(&mut output, 1).pending {}
    assert_eq!(output[0].get(), 42);
}

#[test]
fn registry_drains_each_thread_without_losing_sequences() {
    let (sender, mut receiver) = unbounded_channel(config(8, 2));
    let handles: Vec<_> = (0..8)
        .map(|thread_id| {
            let sender = sender.clone();
            thread::spawn(move || {
                for sequence in 0..1_000 {
                    sender.send((thread_id, sequence)).unwrap();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    receiver.close();
    let mut output = Vec::new();
    while receiver.drain_into(&mut output, 37).pending {}
    assert_eq!(output.len(), 8_000);
    let mut per_thread = vec![Vec::new(); 8];
    for (thread_id, sequence) in output {
        per_thread[thread_id].push(sequence);
    }
    for sequences in per_thread {
        assert_eq!(sequences, (0..1_000).collect::<Vec<_>>());
    }
}

thread_local! {
    static TEARDOWN_EMITTER: TeardownEmitter = const { TeardownEmitter(RefCell::new(None)) };
}

struct TeardownEmitter(RefCell<Option<Sender<usize>>>);

impl Drop for TeardownEmitter {
    fn drop(&mut self) {
        if let Some(sender) = self.0.get_mut().take() {
            sender.send(99).unwrap();
            sender.send(100).unwrap();
        }
    }
}

#[test]
fn thread_local_destructor_emission_reaches_the_receiver() {
    let (sender, mut receiver) = unbounded_channel(config(1, 0));
    thread::spawn(move || {
        TEARDOWN_EMITTER.with(|holder| *holder.0.borrow_mut() = Some(sender.clone()));
        sender.send(1).unwrap();
    })
    .join()
    .unwrap();
    receiver.close();
    let mut output = Vec::new();
    while receiver.drain_into(&mut output, 1).pending {}
    assert_eq!(output, [1, 99, 100]);
}

#[test]
fn close_is_observed_at_a_segment_switch() {
    let (sender, mut receiver) = unbounded_channel(config(2, 0));
    sender.send(1).unwrap();
    receiver.close();
    assert!(sender.send(2).is_ok());
    assert_eq!(sender.send(3), Err(3));
    let mut output = Vec::new();
    while receiver.drain_into(&mut output, 4).pending {}
    assert_eq!(output, [1, 2]);
}

#[test]
fn final_drain_finishes_with_a_live_main_thread_producer() {
    let (sender, mut receiver) = unbounded_channel(config(2, 0));
    sender.send(1).unwrap();
    receiver.close();
    let mut output = Vec::new();
    while receiver.drain_into(&mut output, 1).pending {}
    assert_eq!(output, [1]);
    drop(receiver);
    assert_eq!(sender.send(2), Ok(()));
    assert_eq!(sender.send(3), Ok(()));
    assert_eq!(sender.send(4), Err(4));
}

#[test]
fn concurrent_growth_and_collection_preserve_all_values() {
    let (sender, mut receiver) = unbounded_channel(config(4, 2));
    let collector = thread::spawn(move || {
        let mut output = Vec::new();
        while output.len() < 100_000 {
            receiver.drain_into(&mut output, 31);
            thread::yield_now();
        }
        output
    });
    for value in 0..100_000 {
        sender.send(value).unwrap();
        if value % 17 == 0 {
            thread::yield_now();
        }
    }
    drop(sender);
    assert_eq!(collector.join().unwrap(), (0..100_000).collect::<Vec<_>>());
}

#[test]
fn dropping_a_long_undrained_chain_is_iterative() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = unbounded_channel(config(1, 0));
    let producer = thread::spawn({
        let drops = Arc::clone(&drops);
        move || {
            for _ in 0..20_000 {
                sender.send(Counted(Arc::clone(&drops))).unwrap();
            }
        }
    });
    producer.join().unwrap();
    drop(receiver);
    assert_eq!(drops.load(Ordering::Relaxed), 20_000);
}

#[test]
fn receiver_drop_during_emission_preserves_payload_ownership() {
    let drops = Arc::new(AtomicUsize::new(0));
    let attempts = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = unbounded_channel(config(2, 0));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let sender = sender.clone();
            let drops = Arc::clone(&drops);
            let attempts = Arc::clone(&attempts);
            thread::spawn(move || {
                for _ in 0..1_000 {
                    attempts.fetch_add(1, Ordering::Relaxed);
                    if let Err(value) = sender.send(Counted(Arc::clone(&drops))) {
                        drop(value);
                        break;
                    }
                }
            })
        })
        .collect();
    drop(receiver);
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(
        drops.load(Ordering::Relaxed),
        attempts.load(Ordering::Relaxed)
    );
}

#[test]
fn different_pipeline_types_remain_independent_on_one_thread() {
    let (text_sender, mut text_receiver) = unbounded_channel(config(1, 0));
    let (number_sender, mut number_receiver) = unbounded_channel(config(1, 0));
    text_sender.send(String::from("one")).unwrap();
    number_sender.send(2_u64).unwrap();
    text_receiver.close();
    number_receiver.close();
    let mut text = Vec::new();
    let mut numbers = Vec::new();
    assert!(!text_receiver.drain_into(&mut text, 1).pending);
    assert!(!number_receiver.drain_into(&mut numbers, 1).pending);
    assert_eq!(text, ["one"]);
    assert_eq!(numbers, [2]);
}
