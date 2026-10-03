// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Segmented single-producer, single-consumer transport.

use std::{
    num::NonZeroUsize,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use rtrb::{Consumer as RingConsumer, Producer as RingProducer, PushError, RingBuffer};

/// Segment capacity and the number of empty segments kept for reuse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Config {
    /// Number of event slots per segment.
    pub segment_capacity: NonZeroUsize,
    /// Maximum number of empty segments retained per channel.
    pub spare_segments: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            segment_capacity: NonZeroUsize::new(256).unwrap(),
            spare_segments: 2,
        }
    }
}

/// A non-atomic snapshot of segment counts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metrics {
    /// Active segments, including the current segment.
    pub in_flight: usize,
    /// Allocated segments, including empty spares.
    pub allocated: usize,
    /// Empty segments available to the producer.
    pub spares: usize,
}

/// Result of one bounded drain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DrainReport {
    /// Number of values appended to the destination.
    pub drained: usize,
    /// Whether a live or unfinished channel may yield further values.
    pub pending: bool,
}

#[derive(Debug)]
struct State {
    consumer_gone: AtomicBool,
    in_flight: AtomicUsize,
    allocated: AtomicUsize,
    spares: AtomicUsize,
}

impl State {
    fn metrics(&self) -> Metrics {
        Metrics {
            in_flight: self.in_flight.load(Ordering::Relaxed),
            allocated: self.allocated.load(Ordering::Relaxed),
            spares: self.spares.load(Ordering::Relaxed),
        }
    }
}

struct Handoff<T> {
    retired_writer: RingProducer<T>,
    next_reader: RingConsumer<T>,
    next_node: Arc<Node<T>>,
}

struct Node<T> {
    handoff: OnceLock<Mutex<Handoff<T>>>,
    closed: AtomicBool,
}

impl<T> Node<T> {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            handoff: OnceLock::new(),
            closed: AtomicBool::new(false),
        })
    }

    fn take_handoff(&mut self) -> Option<Handoff<T>> {
        self.handoff.take().map(|handoff| {
            handoff
                .into_inner()
                .unwrap_or_else(|poison| poison.into_inner())
        })
    }
}

struct Spare<T> {
    writer: RingProducer<T>,
    reader: RingConsumer<T>,
    node: Arc<Node<T>>,
}

impl<T> Spare<T> {
    fn new(capacity: NonZeroUsize) -> Self {
        let (writer, reader) = RingBuffer::new(capacity.get());
        Self {
            writer,
            reader,
            node: Node::new(),
        }
    }
}

/// The only writer of a segmented channel.
pub(crate) struct Producer<T> {
    current: Option<RingProducer<T>>,
    spare_reader: Option<RingConsumer<Spare<T>>>,
    state: Arc<State>,
    node: Arc<Node<T>>,
    capacity: NonZeroUsize,
}

/// The only reader of a segmented channel.
pub(crate) struct Consumer<T> {
    current: Option<RingConsumer<T>>,
    spare_writer: Option<RingProducer<Spare<T>>>,
    state: Arc<State>,
    node: Option<Arc<Node<T>>>,
    config: Config,
}

/// Create a segmented SPSC channel.
pub(crate) fn spsc<T: Send + 'static>(config: Config) -> (Producer<T>, Consumer<T>) {
    let Spare {
        writer,
        reader,
        node,
    } = Spare::new(config.segment_capacity);
    let (spare_writer, spare_reader) = RingBuffer::new(config.spare_segments.max(1));
    let state = Arc::new(State {
        consumer_gone: AtomicBool::new(false),
        in_flight: AtomicUsize::new(1),
        allocated: AtomicUsize::new(1),
        spares: AtomicUsize::new(0),
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
        let value = match self.current.as_mut().unwrap().push(value) {
            Ok(()) => return Ok(()),
            Err(PushError::Full(value)) => value,
        };
        if self.state.consumer_gone.load(Ordering::Acquire) {
            return Err(value);
        }

        let spare = match self.spare_reader.as_mut().unwrap().pop() {
            Ok(spare) => {
                self.state.spares.fetch_sub(1, Ordering::Relaxed);
                spare
            }
            Err(_) => {
                self.state.allocated.fetch_add(1, Ordering::Relaxed);
                Spare::new(self.capacity)
            }
        };
        let Spare {
            writer,
            reader,
            node,
        } = spare;
        self.state.in_flight.fetch_add(1, Ordering::Relaxed);
        let old_writer = self.current.replace(writer).unwrap();
        let old_node = std::mem::replace(&mut self.node, Arc::clone(&node));
        if old_node
            .handoff
            .set(Mutex::new(Handoff {
                retired_writer: old_writer,
                next_reader: reader,
                next_node: node,
            }))
            .is_err()
        {
            unreachable!("a segment has exactly one successor");
        }
        drop(old_node);

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
    /// Append at most `limit` published values to `output`.
    ///
    /// `pending` also remains true for an open but currently empty channel.
    pub fn drain_into(&mut self, output: &mut Vec<T>, limit: usize) -> DrainReport {
        let mut drained = 0;
        while drained < limit && self.current.is_some() {
            let available = self.current.as_ref().unwrap().slots();
            if available != 0 {
                let count = available.min(limit - drained);
                let chunk = self.current.as_mut().unwrap().read_chunk(count).unwrap();
                output.extend(chunk);
                drained += count;
                continue;
            }
            if !self.advance() {
                break;
            }
        }
        DrainReport {
            drained,
            pending: self.current.is_some(),
        }
    }

    /// Signal disconnection; already published values remain drainable.
    pub fn close(&mut self) {
        self.state.consumer_gone.store(true, Ordering::Release);
    }

    /// Prepare empty segments on the consumer for future producer bursts.
    ///
    /// Returns the number of new segments supplied.
    pub fn preallocate_spares(&mut self) -> usize {
        let mut supplied = 0;
        while self.state.spares.load(Ordering::Relaxed) < self.config.spare_segments {
            if self.spare_writer.as_ref().unwrap().is_abandoned() {
                break;
            }
            let spare = Spare::new(self.config.segment_capacity);
            if self.spare_writer.as_mut().unwrap().push(spare).is_err() {
                break;
            }
            self.state.allocated.fetch_add(1, Ordering::Relaxed);
            self.state.spares.fetch_add(1, Ordering::Relaxed);
            supplied += 1;
        }
        supplied
    }

    /// Return a sampled count of segments held by this channel.
    pub fn metrics(&self) -> Metrics {
        self.state.metrics()
    }

    pub(crate) fn has_published_work(&self) -> bool {
        let Some(current) = &self.current else {
            return false;
        };
        current.slots() != 0 || self.node.as_ref().unwrap().handoff.get().is_some()
    }

    fn advance(&mut self) -> bool {
        let node = self.node.as_mut().unwrap();
        let Some(inner) = Arc::get_mut(node) else {
            return false;
        };
        // The first empty check can race with the producer's final writes.
        // Once the producer has released this node, check again before recycling.
        if self.current.as_ref().unwrap().slots() != 0 {
            return false;
        }
        if let Some(handoff) = inner.take_handoff() {
            let old_reader = self.current.take().unwrap();
            let old_node = self.node.replace(handoff.next_node).unwrap();
            let spare = Spare {
                writer: handoff.retired_writer,
                reader: old_reader,
                node: old_node,
            };
            self.current = Some(handoff.next_reader);
            self.state.in_flight.fetch_sub(1, Ordering::Relaxed);
            if self.config.spare_segments != 0
                && !self.spare_writer.as_ref().unwrap().is_abandoned()
            {
                match self.spare_writer.as_mut().unwrap().push(spare) {
                    Ok(()) => {
                        self.state.spares.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(PushError::Full(spare)) => {
                        drop(spare);
                        self.state.allocated.fetch_sub(1, Ordering::Relaxed);
                    }
                }
            } else {
                drop(spare);
                self.state.allocated.fetch_sub(1, Ordering::Relaxed);
            }
            return true;
        }
        if inner.closed.load(Ordering::Acquire) {
            drop(self.current.take());
            drop(self.node.take());
            self.state.in_flight.fetch_sub(1, Ordering::Relaxed);
            self.state.allocated.fetch_sub(1, Ordering::Relaxed);
        }
        false
    }
}

impl<T> Drop for Consumer<T> {
    fn drop(&mut self) {
        self.state.consumer_gone.store(true, Ordering::Release);
        while let Some(mut node) = self.node.take() {
            let Some(inner) = Arc::get_mut(&mut node) else {
                drop(node);
                break;
            };
            let handoff = inner.take_handoff();
            drop(self.current.take());
            self.state.in_flight.fetch_sub(1, Ordering::Relaxed);
            self.state.allocated.fetch_sub(1, Ordering::Relaxed);
            drop(node);
            let Some(handoff) = handoff else {
                break;
            };
            drop(handoff.retired_writer);
            self.current = Some(handoff.next_reader);
            self.node = Some(handoff.next_node);
        }
    }
}
