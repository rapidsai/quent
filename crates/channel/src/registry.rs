// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Per-thread producer registration and thread-local teardown fallback.

use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use crate::spsc::{Config, Consumer, DrainReport, Metrics, Producer, spsc};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

struct LocalEntry {
    producer: Box<dyn Any>,
    closed: Arc<AtomicBool>,
}

thread_local! {
    static LOCAL: RefCell<HashMap<usize, LocalEntry>> = RefCell::new(HashMap::new());
}

struct Registry<T> {
    pending: Vec<Consumer<T>>,
    fallback: VecDeque<T>,
    closed: bool,
}

struct Shared<T> {
    id: usize,
    config: Config,
    closed: Arc<AtomicBool>,
    registry: Mutex<Registry<T>>,
}

/// A cloneable, synchronous sender with one SPSC producer per calling thread.
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

/// The single collector of a thread registry.
pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
    channels: Vec<Consumer<T>>,
    cursor: usize,
    fallback_first: bool,
}

/// Create an unbounded channel for a fixed value type.
pub fn unbounded_channel<T: Send + 'static>(config: Config) -> (Sender<T>, Receiver<T>) {
    let id = NEXT_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .expect("channel identity space exhausted");
    let shared = Arc::new(Shared {
        id,
        config,
        closed: Arc::new(AtomicBool::new(false)),
        registry: Mutex::new(Registry {
            pending: Vec::new(),
            fallback: VecDeque::new(),
            closed: false,
        }),
    });
    (
        Sender {
            shared: Arc::clone(&shared),
        },
        Receiver {
            shared,
            channels: Vec::new(),
            cursor: 0,
            fallback_first: false,
        },
    )
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T: Send + 'static> Sender<T> {
    /// Publish an event from the calling thread.
    ///
    /// The first send on a thread registers one channel. If thread-local
    /// storage has already been destroyed, the event uses the registry fallback.
    /// A detected closed receiver returns ownership of the event. Registration
    /// and fallback may lock and allocate; ordinary sends do neither.
    pub fn send(&self, value: T) -> Result<(), T> {
        let mut value = Some(value);
        let mut removed = Vec::new();
        let result = LOCAL.try_with(|local| {
            let mut entries = local.borrow_mut();
            if let Some(entry) = entries.get_mut(&self.shared.id) {
                let producer = entry.producer.downcast_mut::<Producer<T>>().unwrap();
                let result = producer.push(value.take().unwrap());
                if result.is_err() {
                    removed.push(entries.remove(&self.shared.id).unwrap());
                }
                return result;
            }

            removed.extend(
                entries
                    .extract_if(|_, entry| entry.closed.load(Ordering::Acquire))
                    .take(8)
                    .map(|(_, entry)| entry),
            );
            let (mut producer, consumer) = spsc(self.shared.config);
            let mut registry = self
                .shared
                .registry
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if registry.closed {
                drop(registry);
                drop(producer);
                drop(consumer);
                return Err(value.take().unwrap());
            }
            registry.pending.push(consumer);
            drop(registry);
            let result = producer.push(value.take().unwrap());
            entries.insert(
                self.shared.id,
                LocalEntry {
                    producer: Box::new(producer),
                    closed: Arc::clone(&self.shared.closed),
                },
            );
            result
        });
        drop(removed);
        match result {
            Ok(result) => result,
            Err(_) => {
                let mut registry = self
                    .shared
                    .registry
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                if registry.closed {
                    Err(value.take().unwrap())
                } else {
                    registry.fallback.push_back(value.take().unwrap());
                    Ok(())
                }
            }
        }
    }
}

impl<T: Send + 'static> Receiver<T> {
    /// Append up to `limit` published values across registered channels.
    ///
    /// The fallback has no ordering guarantee relative to a thread's channel.
    /// After closure, `pending` reflects published work remaining. To finish
    /// delivery, first stop and synchronize all emission, then call [`Self::close`]
    /// and drain until `pending` is false. Live TLS handles do not prevent this.
    pub fn drain_into(&mut self, output: &mut Vec<T>, limit: usize) -> DrainReport {
        self.collect_registrations();
        if limit == 0 {
            return DrainReport {
                drained: 0,
                pending: self.pending(),
            };
        }
        let start = output.len();
        if self.fallback_first {
            self.drain_fallback(output, limit.div_ceil(2));
        }
        let visits = self.channels.len();
        for _ in 0..visits {
            let remaining = limit - (output.len() - start);
            if remaining == 0 || self.channels.is_empty() {
                break;
            }
            self.cursor %= self.channels.len();
            let budget = remaining.min(self.shared.config.segment_capacity.get());
            let report = self.channels[self.cursor].drain_into(output, budget);
            if !report.pending {
                self.channels.swap_remove(self.cursor);
            } else {
                self.cursor += 1;
            }
        }
        if !self.fallback_first {
            self.drain_fallback(output, limit - (output.len() - start));
        }
        self.fallback_first = !self.fallback_first;
        DrainReport {
            drained: output.len() - start,
            pending: self.pending(),
        }
    }

    /// Stop accepting registration and teardown fallback events.
    ///
    /// Already registered channels remain drainable. Senders notice closure
    /// when they need to switch segments.
    pub fn close(&mut self) {
        let mut registry = self
            .shared
            .registry
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        registry.closed = true;
        self.shared.closed.store(true, Ordering::Release);
        for channel in &mut registry.pending {
            channel.close();
        }
        drop(registry);
        for channel in &mut self.channels {
            channel.close();
        }
    }

    /// Supply spare segments to currently registered channels.
    pub fn preallocate_spares(&mut self) -> usize {
        self.collect_registrations();
        self.channels
            .iter_mut()
            .map(Consumer::preallocate_spares)
            .sum()
    }

    /// Return approximate segment totals for currently registered channels.
    pub fn metrics(&mut self) -> Metrics {
        self.collect_registrations();
        self.channels.iter().fold(
            Metrics {
                in_flight: 0,
                allocated: 0,
                spares: 0,
            },
            |mut total, channel| {
                let metrics = channel.metrics();
                total.in_flight += metrics.in_flight;
                total.allocated += metrics.allocated;
                total.spares += metrics.spares;
                total
            },
        )
    }

    fn collect_registrations(&mut self) {
        let mut registry = self
            .shared
            .registry
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.channels.append(&mut registry.pending);
    }

    fn drain_fallback(&mut self, output: &mut Vec<T>, limit: usize) {
        if limit == 0 {
            return;
        }
        let mut values = Vec::new();
        let mut registry = self
            .shared
            .registry
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for _ in 0..limit {
            let Some(value) = registry.fallback.pop_front() else {
                break;
            };
            values.push(value);
        }
        drop(registry);
        output.extend(values);
    }

    fn pending(&self) -> bool {
        let registry = self
            .shared
            .registry
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        !registry.closed
            || !registry.pending.is_empty()
            || !registry.fallback.is_empty()
            || self.channels.iter().any(Consumer::has_published_work)
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let mut registry = self
            .shared
            .registry
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        registry.closed = true;
        self.shared.closed.store(true, Ordering::Release);
        let pending = std::mem::take(&mut registry.pending);
        let fallback = std::mem::take(&mut registry.fallback);
        drop(registry);
        drop(pending);
        drop(fallback);
    }
}
