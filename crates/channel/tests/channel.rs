// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    cell::{Cell, RefCell},
    num::NonZeroUsize,
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
};

use quent_channel::{
    mpsc::{Receiver, Sender, unbounded_channel_with_config},
    spsc::Config,
};

fn config(capacity: usize, spares: usize) -> Config {
    Config {
        segment_capacity: NonZeroUsize::new(capacity).unwrap(),
        spare_segments: spares,
    }
}

/// Checks that draining preserves existing output, respects the limit, and
/// reports only newly appended values.
#[test]
fn drain_returns_appended_count_and_respects_limit() {
    let (sender, mut receiver) = unbounded_channel_with_config(config(2, 0));
    let mut output = vec![99];
    assert_eq!(
        receiver.drain_into(&mut output, NonZeroUsize::new(2).unwrap()),
        0
    );
    for value in 0..3 {
        sender.send(value).unwrap();
    }
    assert_eq!(output, [99]);
    receiver.close();
    assert_eq!(
        receiver.drain_into(&mut output, NonZeroUsize::new(2).unwrap()),
        2
    );
    assert_eq!(output, [99, 0, 1]);
    assert_eq!(
        receiver.drain_into(&mut output, NonZeroUsize::new(2).unwrap()),
        1
    );
    assert_eq!(output, [99, 0, 1, 2]);
    assert_eq!(
        receiver.drain_into(&mut output, NonZeroUsize::new(2).unwrap()),
        0
    );
}

/// Checks that draining across segment boundaries delivers every value in send
/// order.
#[test]
fn carries_owning_values_in_fifo_order_across_segments() {
    let (sender, mut receiver) = unbounded_channel_with_config::<String>(config(2, 0));
    for i in 0..100 {
        sender.send(i.to_string()).unwrap();
    }
    receiver.close();
    let mut values = Vec::new();
    while receiver.drain_into(&mut values, NonZeroUsize::new(3).unwrap()) != 0 {}
    assert_eq!(values, (0..100).map(|i| i.to_string()).collect::<Vec<_>>());
}

#[derive(Debug)]
struct Counted(Arc<AtomicUsize>);

impl Drop for Counted {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

/// Checks that teardown retains no unread values across segments, whether the
/// producer thread exits or the receiver drops first.
#[test]
fn both_handle_drop_orders_destroy_unread_values_once() {
    for consumer_first in [false, true] {
        let count = Arc::new(AtomicUsize::new(0));
        let (sender, receiver) = unbounded_channel_with_config(config(2, 1));
        let sent = Arc::new(Barrier::new(2));
        let exit = Arc::new(Barrier::new(2));
        let producer = thread::spawn({
            let count = Arc::clone(&count);
            let sent = Arc::clone(&sent);
            let exit = Arc::clone(&exit);
            move || {
                for _ in 0..5 {
                    assert!(sender.send(Counted(Arc::clone(&count))).is_ok());
                }
                sent.wait();
                exit.wait();
            }
        });
        sent.wait();
        if consumer_first {
            drop(receiver);
            exit.wait();
            producer.join().unwrap();
        } else {
            exit.wait();
            producer.join().unwrap();
            drop(receiver);
        }
        assert_eq!(count.load(Ordering::Relaxed), 5);
    }
}

/// Checks that dropping the receiver reclaims queued values and returns
/// ownership of a rejected value.
#[test]
fn early_consumer_drop_reclaims_every_payload_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = unbounded_channel_with_config(config(2, 0));
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

/// Checks at compile time that non-`Sync` payloads support sending and both
/// endpoints remain `Send`.
const _: fn() = || {
    fn assert_send<T: Send>() {}

    assert_send::<Sender<Cell<u32>>>();
    assert_send::<Receiver<Cell<u32>>>();
    let (sender, _receiver) = unbounded_channel_with_config(config(1, 0));
    sender.send(Cell::new(42_u32)).unwrap();
};

/// Checks registration and per-thread ordering for multiple producers, draining
/// only after all producer threads exit.
#[test]
fn registry_drains_each_thread_without_losing_sequences() {
    let (sender, mut receiver) = unbounded_channel_with_config(config(8, 2));
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
    while receiver.drain_into(&mut output, NonZeroUsize::new(37).unwrap()) != 0 {}
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

/// Checks that events emitted during thread-local destruction are delivered in
/// fallback order without requiring cross-queue ordering.
#[test]
fn thread_local_destructor_emission_reaches_the_receiver() {
    let (sender, mut receiver) = unbounded_channel_with_config(config(1, 0));
    thread::spawn(move || {
        TEARDOWN_EMITTER.with(|holder| *holder.0.borrow_mut() = Some(sender.clone()));
        sender.send(1).unwrap();
    })
    .join()
    .unwrap();
    receiver.close();
    let mut output = Vec::new();
    while receiver.drain_into(&mut output, NonZeroUsize::new(1).unwrap()) != 0 {}
    assert_eq!(output.len(), 3);
    assert_eq!(output.iter().filter(|&&value| value == 1).count(), 1);
    assert_eq!(
        output
            .into_iter()
            .filter(|&value| value != 1)
            .collect::<Vec<_>>(),
        [99, 100]
    );
}

/// Checks that closure preserves buffered values and a rejected send returns
/// its value.
#[test]
fn closure_preserves_buffered_values_and_returns_rejected_values() {
    let (sender, mut receiver) = unbounded_channel_with_config(config(2, 0));
    sender.send(1).unwrap();
    sender.send(2).unwrap();
    receiver.close();
    assert_eq!(sender.send(3), Err(3));
    let mut output = Vec::new();
    while receiver.drain_into(&mut output, NonZeroUsize::new(4).unwrap()) != 0 {}
    assert_eq!(output, [1, 2]);
}

/// Checks that closure preserves buffered segments but bounds later refills,
/// including when closure is repeated or queues were already collected.
#[test]
fn shutdown_budget_is_fixed_per_queue() {
    for collect_first in [false, true] {
        let (sender, mut receiver) = unbounded_channel_with_config(config(2, 0));
        for value in 0..5 {
            sender.send(value).unwrap();
        }
        let mut output = Vec::new();
        let limit = NonZeroUsize::new(2).unwrap();
        if collect_first {
            assert_eq!(receiver.drain_into(&mut output, limit), 2);
        }
        receiver.close();
        while output.len() < 5 {
            assert_ne!(receiver.drain_into(&mut output, limit), 0);
        }
        assert_eq!(output, [0, 1, 2, 3, 4]);
        for value in 5..7 {
            sender.send(value).unwrap();
            receiver.close();
            assert_eq!(receiver.drain_into(&mut output, limit), 1);
        }
        let _ = sender.send(7);
        receiver.close();
        assert_eq!(receiver.drain_into(&mut output, limit), 0);
        assert_eq!(output, [0, 1, 2, 3, 4, 5, 6]);
    }
}

/// Checks that final draining finishes without waiting for a live producer to
/// drop.
#[test]
fn final_drain_finishes_with_a_live_main_thread_producer() {
    let (sender, mut receiver) = unbounded_channel_with_config(config(2, 0));
    sender.send(1).unwrap();
    receiver.close();
    let mut output = Vec::new();
    while receiver.drain_into(&mut output, NonZeroUsize::new(1).unwrap()) != 0 {}
    assert_eq!(output, [1]);
    drop(receiver);
    drop(sender);
}

/// Checks complete, ordered delivery with one producer and a concurrently
/// draining consumer as segments grow and are reused.
#[test]
fn concurrent_growth_and_collection_preserve_all_values() {
    let (sender, mut receiver) = unbounded_channel_with_config(config(4, 2));
    let finished = Arc::new(AtomicBool::new(false));
    let producer_finished = Arc::clone(&finished);
    let collector = thread::spawn(move || {
        let mut output = Vec::new();
        while !producer_finished.load(Ordering::Acquire) {
            receiver.drain_into(&mut output, NonZeroUsize::new(31).unwrap());
            thread::yield_now();
        }
        receiver.close();
        while receiver.drain_into(&mut output, NonZeroUsize::new(31).unwrap()) != 0 {}
        output
    });
    for value in 0..100_000 {
        sender.send(value).unwrap();
        if value % 17 == 0 {
            thread::yield_now();
        }
    }
    drop(sender);
    finished.store(true, Ordering::Release);
    assert_eq!(collector.join().unwrap(), (0..100_000).collect::<Vec<_>>());
}

/// Checks that dropping a long segment chain destroys every unread value
/// without overflowing the stack.
#[test]
fn dropping_a_long_undrained_chain_is_iterative() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = unbounded_channel_with_config(config(1, 0));
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

/// Checks that receiver teardown racing with further sends from registered
/// producers retains no payloads.
#[test]
fn receiver_drop_during_emission_preserves_payload_ownership() {
    let drops = Arc::new(AtomicUsize::new(0));
    let attempts = Arc::new(AtomicUsize::new(0));
    // Wait for all four producers to queue a value before receiver teardown can
    // race with further sends.
    let ready = Arc::new(Barrier::new(5));
    let (sender, receiver) = unbounded_channel_with_config(config(2, 0));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let sender = sender.clone();
            let drops = Arc::clone(&drops);
            let attempts = Arc::clone(&attempts);
            let ready = Arc::clone(&ready);
            thread::spawn(move || {
                attempts.fetch_add(1, Ordering::Relaxed);
                sender.send(Counted(Arc::clone(&drops))).unwrap();
                ready.wait();
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
    ready.wait();
    drop(receiver);
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(
        drops.load(Ordering::Relaxed),
        attempts.load(Ordering::Relaxed)
    );
}

/// Checks that channels carrying different value types do not interfere when
/// used on the same thread.
#[test]
fn different_pipeline_types_remain_independent_on_one_thread() {
    let (text_sender, mut text_receiver) = unbounded_channel_with_config(config(1, 0));
    let (number_sender, mut number_receiver) = unbounded_channel_with_config(config(1, 0));
    text_sender.send(String::from("one")).unwrap();
    number_sender.send(2_u64).unwrap();
    text_receiver.close();
    number_receiver.close();
    let mut text = Vec::new();
    let mut numbers = Vec::new();
    assert_eq!(
        text_receiver.drain_into(&mut text, NonZeroUsize::new(1).unwrap()),
        1
    );
    assert_eq!(
        text_receiver.drain_into(&mut text, NonZeroUsize::new(1).unwrap()),
        0
    );
    assert_eq!(
        number_receiver.drain_into(&mut numbers, NonZeroUsize::new(1).unwrap()),
        1
    );
    assert_eq!(
        number_receiver.drain_into(&mut numbers, NonZeroUsize::new(1).unwrap()),
        0
    );
    assert_eq!(text, ["one"]);
    assert_eq!(numbers, [2]);
}
