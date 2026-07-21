// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! A per-entity instance handle forwarding events to the observer's event
//! pipeline.

use std::sync::Arc;

use uuid::Uuid;

use crate::observer::Observer;

/// An error from emitting through a [`Handle`].
#[derive(Debug, thiserror::Error)]
pub enum HandleError {
    /// A once-cardinality event was emitted more than once for one entity
    /// instance.
    #[error("once-event `{event}` already emitted for this entity instance")]
    OnceAlreadyEmitted {
        /// Name of the event that was re-emitted.
        event: &'static str,
    },
}

/// A handle to one entity instance.
///
/// Exports this instance's events through an [`Observer`] shared with other
/// handles.
/// Enforces once-cardinality events are sent at most once.
#[doc(hidden)]
pub struct Handle<E> {
    id: Uuid,
    /// One bit per once-cardinality event, set once that event is emitted.
    once_flags: u64,
    observer: Arc<Observer<E>>,
}

impl<E> Handle<E> {
    /// Create a handle for a new entity instance, with a generated id.
    pub fn new(observer: Arc<Observer<E>>) -> Self {
        Self::with_id(Uuid::now_v7(), observer)
    }

    /// Create a handle for the entity instance identified by `id`.
    pub fn with_id(id: Uuid, observer: Arc<Observer<E>>) -> Self {
        Self {
            id,
            once_flags: 0,
            observer,
        }
    }

    /// The entity instance id this handle emits for.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Emit a multi-cardinality event for this instance.
    pub fn emit(&self, event: E) {
        self.observer.emit(self.id, event);
    }

    /// Emit a once-cardinality event.
    ///
    /// Returns [`HandleError::OnceAlreadyEmitted`] if this handle previously
    /// emitted an event with the same `INDEX`.
    pub fn emit_once<const INDEX: u32>(
        &mut self,
        event_name: &'static str,
        event: E,
    ) -> Result<(), HandleError> {
        const { assert!(INDEX < u64::BITS, "once-event bit index out of range") };
        let mask = 1u64 << INDEX;
        if self.once_flags & mask != 0 {
            return Err(HandleError::OnceAlreadyEmitted { event: event_name });
        }
        self.once_flags |= mask;
        self.observer.emit(self.id, event);
        Ok(())
    }

    /// Whether the once-cardinality event tracked by `INDEX` has already been
    /// emitted for this instance.
    pub fn is_emitted<const INDEX: u32>(&self) -> bool {
        const { assert!(INDEX < u64::BITS, "once-event bit index out of range") };
        self.once_flags & (1u64 << INDEX) != 0
    }
}
