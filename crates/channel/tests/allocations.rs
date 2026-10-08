// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    alloc::{GlobalAlloc, Layout, System},
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};

use quent_channel::{mpsc::unbounded_channel_with_config, spsc::Config};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// Checks that bounded bursts and drains allocate nothing after recycling
/// warmup with sufficient output capacity.
#[test]
fn warmed_pushes_and_recycled_switches_do_not_allocate() {
    let config = Config {
        segment_capacity: NonZeroUsize::new(2).unwrap(),
        spare_segments: 2,
    };
    let (sender, mut receiver) = unbounded_channel_with_config(config);
    for value in 0..6 {
        sender.send(value).unwrap();
    }
    let mut output = Vec::with_capacity(2_001);
    while receiver.drain_into(&mut output, NonZeroUsize::new(2).unwrap()) != 0 {}
    output.clear();
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    for batch in 0..500 {
        for offset in 0..4 {
            sender.send(batch * 4 + offset).unwrap();
        }
        assert_eq!(
            receiver.drain_into(&mut output, NonZeroUsize::new(2).unwrap()),
            2
        );
        assert_eq!(
            receiver.drain_into(&mut output, NonZeroUsize::new(2).unwrap()),
            2
        );
    }
    let after = ALLOCATIONS.load(Ordering::Relaxed);
    assert_eq!(after, before);
    assert_eq!(output, (0..2_000).collect::<Vec<_>>());
}
