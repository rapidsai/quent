// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Unbounded single-producer, single-consumer channel.
//!
//! The consumer receives values in the order they were sent and can drain them
//! in batches. Buffered values have no fixed capacity limit, so sends do not
//! need to wait for the consumer to free space. A successful send makes its
//! value available to the consumer immediately before returning.
//!
//! Values are stored in a chain of fixed-size ring buffers called segments.
//! When the current segment is full, the producer continues in the next segment,
//! reusing an empty spare segment or allocating a new one if no empty segment
//! is available. The consumer drains older segments first, then returns them
//! for reuse or releases them if enough spares are available.
//!
//! As the consumer drains values, it advances a read index that marks the free
//! slots. The producer keeps a local copy of that index, refreshing it only
//! when the ring appears full. A push into an available slot publishes its
//! value before returning, without a lock, compare-and-swap, or allocation.
//!
//! Using `rtrb` avoids implementing slot ownership and publication with unsafe
//! code here. A single bounded ring would reject sends when full.
//!
//! The configured spare limit bounds the number of empty segments retained
//! for reuse, so a temporary burst does not keep memory usage at its peak.
//!
//! The producer checks whether the consumer has closed only when it finds its
//! current segment full, avoiding an extra atomic read on every send. This
//! reduces per-send overhead, which matters when the producer sends a burst of
//! many small values.
//!
//! Until the segment fills up, sends can succeed even after the consumer closes
//! or drops. Draining can free slots and delay the check. A send can therefore
//! report success even though the consumer will never receive its value. To
//! receive every value, stop the producer and wait for any send in progress to
//! finish before closing and draining the consumer. Dropping the consumer
//! discards unread values. Closing limits subsequent draining to the buffered
//! values observed at closure plus one segment's capacity, so refilling cannot
//! prolong draining indefinitely.

use std::{
    num::NonZeroUsize,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use rtrb::{Consumer as RingConsumer, Producer as RingProducer, PushError, RingBuffer};

/// Segment capacity and spare retention for one SPSC channel.
///
/// In the MPSC channel, these settings apply separately to each sending thread.
/// Defaults to 256 slots per segment and two retained spare segments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Config {
    /// Number of event slots per segment.
    pub segment_capacity: NonZeroUsize,
    /// Maximum number of empty segments retained per channel, with zero
    /// disabling retention.
    pub spare_segments: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // safety: The fixed capacity is nonzero.
            segment_capacity: NonZeroUsize::new(256).unwrap(),
            spare_segments: 2,
        }
    }
}

/// Result of one drain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DrainResult {
    /// Number of values appended to the destination.
    pub drained: usize,
    /// Whether a live or unfinished channel may yield further values.
    pub pending: bool,
}

/// Tracks consumer closure for one SPSC channel's endpoints.
#[derive(Debug)]
struct ChannelState {
    /// Set when the consumer closes or drops.
    consumer_closed: AtomicBool,
}

/// Holds the old segment's writer and the next segment's reader and shared
/// state.
///
/// The producer moves both endpoints into this value when it switches segments.
/// The consumer takes the whole value after draining the old segment, pairs the
/// old writer with its existing reader for reuse, and continues reading from
/// the next segment.
struct SegmentTransition<T> {
    /// Writer tied to the old segment's ring buffer, no longer used by the
    /// producer.
    ///
    /// Retaining it allows writes into the same allocation when reused, since
    /// a writer cannot be recreated from the reader alone.
    retired_writer: RingProducer<T>,
    /// Reader paired with the producer's new writer, used only after the old
    /// segment is drained.
    next_reader: RingConsumer<T>,
    /// Next segment's successor and closure state, shared with the producer.
    next_node: Arc<SegmentState<T>>,
}

/// Records a segment's successor or the end of the producer's writes.
struct SegmentState<T> {
    /// Endpoints and shared state passed to the consumer when the producer
    /// moves to the next segment.
    transition: OnceLock<Mutex<SegmentTransition<T>>>,
    /// Set when the producer drops, after its final writes and release of this
    /// segment's writer.
    closed: AtomicBool,
}

impl<T> SegmentState<T> {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            transition: OnceLock::new(),
            closed: AtomicBool::new(false),
        })
    }

    fn take_transition(&mut self) -> Option<SegmentTransition<T>> {
        self.transition.take().map(|transition| {
            transition
                .into_inner()
                .unwrap_or_else(|poison| poison.into_inner())
        })
    }
}

/// Groups a ring buffer's paired endpoints and state for allocation or reuse.
struct Segment<T> {
    /// Write endpoint paired with this segment's reader.
    writer: RingProducer<T>,
    /// Read endpoint for values written through this segment's writer.
    reader: RingConsumer<T>,
    /// Tracks this segment's successor or the end of the producer's writes.
    node: Arc<SegmentState<T>>,
}

impl<T> Segment<T> {
    fn new(capacity: NonZeroUsize) -> Self {
        let (writer, reader) = RingBuffer::new(capacity.get());
        Self {
            writer,
            reader,
            node: SegmentState::new(),
        }
    }
}

/// The only writer of a segmented channel.
pub(crate) struct Producer<T> {
    /// Write endpoint for the active segment, taken during producer teardown.
    current: Option<RingProducer<T>>,
    /// Receives empty segments supplied by the consumer for reuse.
    spare_reader: Option<RingConsumer<Segment<T>>>,
    /// Consumer closure state shared with the consumer.
    state: Arc<ChannelState>,
    /// Active segment's state, retained until the producer switches segments or
    /// drops.
    node: Arc<SegmentState<T>>,
    /// Number of value slots allocated in each new segment.
    capacity: NonZeroUsize,
}

/// The only reader of a segmented channel.
pub(crate) struct Consumer<T> {
    /// Read endpoint for the oldest unfinished segment, absent once the channel
    /// is fully drained and disconnected.
    current: Option<RingConsumer<T>>,
    /// Returns empty segments to the producer for reuse.
    spare_writer: Option<RingProducer<Segment<T>>>,
    /// Consumer closure state shared with the producer.
    state: Arc<ChannelState>,
    /// Current segment's state, absent once no segments remain to drain.
    node: Option<Arc<SegmentState<T>>>,
    /// Settings controlling retention of empty segments for reuse.
    config: Config,
    /// Values still eligible for draining after closure, including one segment
    /// of allowance for concurrent sends. Absent while open.
    shutdown_remaining: Option<usize>,
}

/// Create an unbounded single-producer, single-consumer channel.
pub(crate) fn spsc<T: Send + 'static>(config: Config) -> (Producer<T>, Consumer<T>) {
    let Segment {
        writer,
        reader,
        node,
    } = Segment::new(config.segment_capacity);
    let (spare_writer, spare_reader) = RingBuffer::new(config.spare_segments.max(1));
    let state = Arc::new(ChannelState {
        consumer_closed: AtomicBool::new(false),
    });
    (
        Producer {
            current: Some(writer),
            spare_reader: Some(spare_reader),
            state: Arc::clone(&state),
            node: Arc::clone(&node),
            capacity: config.segment_capacity,
        },
        Consumer {
            current: Some(reader),
            spare_writer: Some(spare_writer),
            state,
            node: Some(node),
            config,
            shutdown_remaining: None,
        },
    )
}

impl<T: Send + 'static> Producer<T> {
    /// Publish `value` before returning, growing the channel if needed.
    ///
    /// A push into the current segment neither allocates nor checks whether the
    /// consumer is gone. Growth may allocate and inherit allocator latency.
    /// On a full segment, disconnection returns `value`.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        // safety: The current writer is only removed during producer teardown.
        let value = match self.current.as_mut().unwrap().push(value) {
            Ok(()) => return Ok(()),
            Err(PushError::Full(value)) => value,
        };
        if self.state.consumer_closed.load(Ordering::Acquire) {
            return Err(value);
        }

        // safety: The spare reader is only removed during producer teardown.
        let spare = match self.spare_reader.as_mut().unwrap().pop() {
            Ok(spare) => spare,
            Err(_) => Segment::new(self.capacity),
        };
        let Segment {
            writer,
            reader,
            node,
        } = spare;
        // safety: The producer still owns its current writer when switching
        // segments.
        let old_writer = self.current.replace(writer).unwrap();
        let old_node = std::mem::replace(&mut self.node, Arc::clone(&node));
        if old_node
            .transition
            .set(Mutex::new(SegmentTransition {
                retired_writer: old_writer,
                next_reader: reader,
                next_node: node,
            }))
            .is_err()
        {
            unreachable!("a segment has exactly one successor");
        }
        drop(old_node);

        // safety: The replacement writer was installed above.
        match self.current.as_mut().unwrap().push(value) {
            Ok(()) => Ok(()),
            Err(PushError::Full(_)) => unreachable!("a new segment has free slots"),
        }
    }
}

impl<T> Drop for Producer<T> {
    fn drop(&mut self) {
        // Closure follows the last ring publication and release of its writer.
        drop(self.current.take());
        drop(self.spare_reader.take());
        self.node.closed.store(true, Ordering::Release);
    }
}

impl<T: Send + 'static> Consumer<T> {
    /// Returns whether no segments remain or the shutdown budget is exhausted.
    pub(crate) fn is_finished(&self) -> bool {
        self.current.is_none() || self.shutdown_remaining == Some(0)
    }

    /// Append at most `limit` published values to `output`.
    ///
    /// `pending` also remains true for an open but currently empty channel.
    pub fn drain_into(&mut self, output: &mut Vec<T>, limit: NonZeroUsize) -> DrainResult {
        let limit = self
            .shutdown_remaining
            .map_or(limit.get(), |remaining| limit.get().min(remaining));
        let mut drained = 0;
        while drained < limit && self.current.is_some() {
            // safety: The loop condition established that a current reader
            // exists.
            let available = self.current.as_ref().unwrap().slots();
            if available != 0 {
                let count = available.min(limit - drained);
                // safety: The reader is still present, and the sole consumer
                // cannot lose available slots to another reader.
                let chunk = self.current.as_mut().unwrap().read_chunk(count).unwrap();
                output.extend(chunk);
                drained += count;
                continue;
            }
            if !self.advance() {
                break;
            }
        }
        if let Some(remaining) = &mut self.shutdown_remaining {
            *remaining -= drained;
        }
        DrainResult {
            drained,
            pending: !self.is_finished(),
        }
    }

    /// Signals disconnection and bounds draining to buffered values plus one
    /// segment's capacity. Repeated calls do not replenish the budget.
    pub fn close(&mut self) {
        if self.shutdown_remaining.is_some() {
            return;
        }
        self.state.consumer_closed.store(true, Ordering::Release);
        let mut remaining = self.current.as_ref().map_or(0, RingConsumer::slots);
        let mut node = self.node.clone();
        while let Some(current) = node {
            node = current.transition.get().map(|transition| {
                let transition = transition.lock().unwrap_or_else(|error| error.into_inner());
                remaining = remaining.saturating_add(transition.next_reader.slots());
                Arc::clone(&transition.next_node)
            });
        }
        self.shutdown_remaining =
            Some(remaining.saturating_add(self.config.segment_capacity.get()));
    }

    fn advance(&mut self) -> bool {
        // safety: Only draining an existing reader calls advance, and that
        // reader has a segment state.
        let node = self.node.as_mut().unwrap();
        let Some(inner) = Arc::get_mut(node) else {
            return false;
        };
        // The first empty check can race with the producer's final writes.
        // Once the producer has released this node, check again before
        // recycling.
        // safety: The caller's current reader has not been removed.
        if self.current.as_ref().unwrap().slots() != 0 {
            return false;
        }
        if let Some(transition) = inner.take_transition() {
            // safety: The current reader remains present until this take.
            let old_reader = self.current.take().unwrap();
            // safety: The segment state borrowed above remains installed until
            // this replacement.
            let old_node = self.node.replace(transition.next_node).unwrap();
            let spare = Segment {
                writer: transition.retired_writer,
                reader: old_reader,
                node: old_node,
            };
            self.current = Some(transition.next_reader);
            // safety: The spare writer remains present throughout the
            // consumer's lifetime.
            if self.config.spare_segments != 0
                && !self.spare_writer.as_ref().unwrap().is_abandoned()
            {
                // safety: Checking whether the spare writer is abandoned does
                // not remove it.
                if let Err(PushError::Full(spare)) = self.spare_writer.as_mut().unwrap().push(spare)
                {
                    drop(spare);
                }
            } else {
                drop(spare);
            }
            return true;
        }
        if inner.closed.load(Ordering::Acquire) {
            drop(self.current.take());
            drop(self.node.take());
        }
        false
    }
}

impl<T> Drop for Consumer<T> {
    fn drop(&mut self) {
        self.state.consumer_closed.store(true, Ordering::Release);
        while let Some(mut node) = self.node.take() {
            let Some(inner) = Arc::get_mut(&mut node) else {
                drop(node);
                break;
            };
            let transition = inner.take_transition();
            drop(self.current.take());
            drop(node);
            let Some(transition) = transition else {
                break;
            };
            drop(transition.retired_writer);
            self.current = Some(transition.next_reader);
            self.node = Some(transition.next_node);
        }
    }
}
