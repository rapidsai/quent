// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! An unbounded channel optimized for low producer latency with any number of
//! sending threads.
//!
//! [`unbounded_channel`] creates an MPSC queue designed for low send latency.
//! It gives each sending thread its own SPSC queue, built from a chain of
//! fixed-size ring buffers. Producers therefore do not compete for one shared
//! queue. The tradeoff is memory for each thread's queue and reusable segments,
//! plus a weaker shutdown cutoff than Quent's default Tokio transport. After
//! the receiver closes, an existing sender can still accept values until it
//! needs another segment. The receiver can still drain that segment, including
//! values sent after closure.
//!
//! 1. Give each thread using the shareable sender its own queue on first send.
//! 2. Publish values into fixed-size segments, adding segments as needed.
//! 3. Drain queues in batches, then reuse or release empty segments.
//! 4. Route sends during thread-local teardown through a fallback queue.
//!
//! ## Per-thread queues
//!
//! The shareable sender registers one SPSC queue per sending thread on its first
//! send. Values stay in order within a queue; there is no ordering guarantee
//! across threads or between a queue and the fallback.
//!
//! Quent events carry timestamps. Moving one entity handle between threads
//! normally takes longer than a send or obtaining a timestamp, so its events
//! are usually easy to order. If several handles for the same entity emit
//! concurrently, callers must synchronize them when order matters. Whether
//! timestamps alone distinguish those events depends on the configured clock.
//! (Re-)consider that case carefully before using this channel.
//!
//! ## Publication and growth
//!
//! Each segment is a bounded `rtrb` ring. As the consumer drains values, it
//! advances a read index that marks the free slots. The producer keeps a local
//! copy of that index, refreshing it only when the ring appears full. An
//! ordinary push publishes its value before returning, without a lock,
//! compare-and-swap, or allocation.
//!
//! Using `rtrb` avoids implementing slot ownership and publication with unsafe
//! code here. A single bounded ring would reject sends when full.
//! General-purpose unbounded MPSC channels coordinate concurrent producers
//! that a per-thread queue does not have.
//!
//! When a segment fills, the producer takes an empty one returned by the
//! receiver or allocates a new one.
//!
//! ## Collection and reuse
//!
//! The receiver visits registered queues and drains values in batches. It
//! returns empty segments for reuse and discards any beyond the configured
//! spare limit, so a temporary burst does not keep memory usage at its peak.
//!
//! ## Teardown and shutdown
//!
//! Sends during thread-local destruction use a separate fallback queue if the
//! thread's queue has already been destroyed. The channel does not wake an
//! async task.
//! After stopping sends, drain before dropping the receiver; values still
//! buffered when it is dropped are destroyed.
//!
//! ## Related designs
//!
//! [Quill](https://github.com/odygrd/quill/blob/master/include/quill/core/ThreadContextManager.h),
//! [ticklog](https://github.com/tensorbinge/ticklog),
//! [fmtlog](https://github.com/MengRao/fmtlog/blob/main/fmtlog.h), and
//! [fastrace](https://github.com/fast/fastrace/blob/main/crates/fastrace/src/util/command_bus.rs)
//! also collect from per-thread producer queues. They informed this channel's
//! layout, though their capacity and overflow policies differ.

// TODO(johanpel): Benchmark refreshing the consumer index before a segment
// appears full. A configurable threshold inside rtrb would require adding that
// option to rtrb; this crate can also refresh early through Producer::slots().
mod registry;
mod spsc;

pub use registry::{Receiver, Sender, unbounded_channel};
pub use spsc::{Config, DrainReport, Metrics};
