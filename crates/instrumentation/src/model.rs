// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Instrumentation models and their contexts.

use crate::{ContextInner, ExporterOptions, InstrumentedEntity, Observer, Uuid, write_sidecar};

/// Provides typed access to an entity observer in a generated model.
///
/// Hidden because generated observer collections implement it; callers use
/// [`Context::observer`].
#[doc(hidden)]
pub trait ObserverProvider<E: InstrumentedEntity> {
    /// Returns the observer stored for `E`.
    fn observer(&self) -> Observer<E>;
}

/// Supplies schema-specific observers to an instrumentation context.
pub trait InstrumentedModel {
    /// Generated observers for this model.
    ///
    /// Hidden because callers access observers through [`Context::observer`].
    #[doc(hidden)]
    type Observers;

    /// Builds the observers for this model.
    ///
    /// `exporter` is `None` for a no-op context.
    ///
    /// Hidden because [`Context`] invokes it during construction.
    ///
    /// # Errors
    ///
    /// Returns an error when an observer or its exporter cannot be constructed.
    #[doc(hidden)]
    fn build_observers(
        context: &ContextInner,
        exporter: Option<&ExporterOptions>,
    ) -> Result<Self::Observers, Box<dyn std::error::Error>>;
}

/// Instrumentation context for a generated model.
pub struct Context<M: InstrumentedModel> {
    observers: M::Observers,
    inner: ContextInner,
}

impl<M: quent_events::Model + InstrumentedModel> Context<M> {
    /// Creates a context and builds every entity's exporter pipeline.
    ///
    /// Passing `None` creates a no-op context that discards events.
    pub fn try_new(exporter: Option<ExporterOptions>) -> Result<Self, Box<dyn std::error::Error>>
    where
        M: crate::build_info::ModelSource,
    {
        Self::try_with_id(Uuid::now_v7(), exporter)
    }

    /// Creates a context with the supplied ID.
    pub fn try_with_id(
        id: Uuid,
        exporter: Option<ExporterOptions>,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        M: crate::build_info::ModelSource,
    {
        let inner = if exporter.is_some() {
            ContextInner::try_new(id)?
        } else {
            ContextInner::noop(id)
        };
        if let Some(options) = &exporter {
            write_sidecar(options, id, M::model_info());
        }
        let observers = M::build_observers(&inner, exporter.as_ref())?;
        Ok(Self { observers, inner })
    }

    /// Returns the context ID.
    pub fn id(&self) -> Uuid {
        self.inner.id()
    }

    /// Returns the observer associated with entity marker `E`.
    pub fn observer<E>(&self) -> Observer<E>
    where
        E: InstrumentedEntity<Context = Self>,
        M::Observers: ObserverProvider<E>,
    {
        self.observers.observer()
    }
}
