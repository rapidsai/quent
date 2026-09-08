// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Instrumentation models and their contexts.

use crate::{ContextExporter, ContextInner, InstrumentedEntity, Observer, Uuid};

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
}

/// Builds a model's observers from an exporter provider.
///
/// Generated implementations require `P` to provide an exporter for every
/// entity event type in the model.
#[doc(hidden)]
pub trait ObserverBuilder<P>: InstrumentedModel {
    /// Builds every observer from `provider`.
    ///
    /// # Errors
    ///
    /// Returns an error when an observer or exporter cannot be constructed.
    #[doc(hidden)]
    fn build_observers(
        context: &ContextInner,
        provider: &P,
    ) -> Result<Self::Observers, Box<dyn std::error::Error>>;
}

/// Instrumentation context for a generated model.
pub struct Context<M: InstrumentedModel> {
    observers: M::Observers,
    inner: ContextInner,
}

impl<M: quent_events::Model + InstrumentedModel> Context<M> {
    /// Creates a context and builds every entity's exporter pipeline.
    pub fn try_new<P>(provider: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        M: crate::build_info::ModelSource + ObserverBuilder<P>,
        P: ContextExporter,
    {
        Self::try_with_id(Uuid::now_v7(), provider)
    }

    /// Creates a context with the supplied ID.
    pub fn try_with_id<P>(id: Uuid, provider: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        M: crate::build_info::ModelSource + ObserverBuilder<P>,
        P: ContextExporter,
    {
        let inner = if provider.is_noop() {
            ContextInner::noop(id)
        } else {
            ContextInner::try_new(id)?
        };
        provider.prepare_context(id, M::model_info());
        let observers = M::build_observers(&inner, &provider)?;
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
