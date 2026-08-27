// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Instrumented entity markers used by generated instrumentation libraries.

use std::sync::Arc;

use crate::{HandleInner, ObserverInner};

/// Adds instrumentation context and handle types to an entity marker.
pub trait InstrumentedEntity: quent_events::Entity + Sized {
    /// Instrumentation context containing this entity.
    type Context;

    /// Generated handle for this entity.
    type Handle: From<HandleInner<Self>>;
}

/// Provides handles for an entity type through its shared event observer.
pub struct Observer<E: InstrumentedEntity> {
    inner: Arc<ObserverInner<E::Event>>,
}

impl<E: InstrumentedEntity> Clone for Observer<E> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<E: InstrumentedEntity> Observer<E> {
    /// Creates an observer backed by `inner`.
    ///
    /// Hidden because generated models construct observers; callers obtain them
    /// through their model context.
    #[doc(hidden)]
    pub fn new(inner: Arc<ObserverInner<E::Event>>) -> Self {
        Self { inner }
    }

    /// Creates a handle for a fresh entity instance.
    pub fn handle(&self) -> E::Handle {
        HandleInner::new(Arc::clone(&self.inner)).into()
    }

    /// Creates a handle for the entity instance identified by `id`.
    pub fn handle_with_id(&self, id: crate::Uuid) -> E::Handle {
        HandleInner::with_id(id, Arc::clone(&self.inner)).into()
    }
}
