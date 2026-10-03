// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    alloc::{GlobalAlloc, Layout, System},
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};

use quent_channel::{Config, unbounded_channel};

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

#[test]
fn warmed_pushes_and_recycled_switches_do_not_allocate() {
    let config = Config {
        segment_capacity: NonZeroUsize::new(2).unwrap(),
        spare_segments: 2,
    };
    let (sender, mut receiver) = unbounded_channel(config);
    sender.send(usize::MAX).unwrap();
    let mut output = Vec::with_capacity(2_001);
    receiver.drain_into(&mut output, 1);
    output.clear();
    assert_eq!(receiver.preallocate_spares(), 2);
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    for value in 0..2_000 {
        sender.send(value).unwrap();
        receiver.drain_into(&mut output, 2);
    }
    let after = ALLOCATIONS.load(Ordering::Relaxed);
    assert_eq!(after, before);
    assert_eq!(output, (0..2_000).collect::<Vec<_>>());
}
