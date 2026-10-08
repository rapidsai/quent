// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Instrumentation models and their contexts.

use crate::{ContextExporter, ContextInner, InstrumentedEntity, Observer, Uuid};
use std::num::NonZeroUsize;

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

/// Settings for an active context's asynchronous runtime.
///
/// Create options with [`Self::default`] and set the fields you want to change.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct RuntimeOptions {
    /// Uses the available CPU count when unset, falling back to one worker if
    /// that count cannot be determined.
    pub worker_threads: Option<NonZeroUsize>,
    /// Limits threads for blocking work, such as file writes in exporters,
    /// separately from worker threads.
    ///
    /// Defaults to 512.
    ///
    /// Blocking threads are created as needed. Leave this unset unless you
    /// really need to limit thread usage. Work waits when all threads are busy,
    /// so a lower limit may slow file writes and flushing.
    ///
    /// Custom exporters can deadlock if their blocking tasks wait for other
    /// work in the same pool and no thread is free to run it.
    pub max_blocking_threads: Option<NonZeroUsize>,
    /// Overrides the default runtime thread name, `quent-rt-worker`, when set.
    pub thread_name: Option<String>,
}

/// Creates and holds the runtime and observers for an application's events.
///
/// Each active context creates its own asynchronous runtime and worker threads.
/// Creating multiple active contexts therefore uses more threads and resources.
/// No-op contexts do not create a runtime.
///
/// Observers and handles keep the runtime alive after the context is dropped.
/// Dropping the last owner of an observer, including its handles, waits for
/// queued events to be exported and its exporter to flush.
/// Channel-specific shutdown guarantees are documented in `PERFORMANCE.md`.
pub struct Context<M: InstrumentedModel> {
    observers: M::Observers,
    inner: ContextInner,
}

impl<M: quent_events::EventModel + InstrumentedModel> Context<M> {
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
        Self::try_with_id_and_options(id, provider, RuntimeOptions::default())
    }

    /// Creates a context using the supplied runtime settings.
    ///
    /// Runtime settings are ignored for a no-op exporter.
    pub fn try_new_with_options<P>(
        provider: P,
        options: RuntimeOptions,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        M: crate::build_info::ModelSource + ObserverBuilder<P>,
        P: ContextExporter,
    {
        Self::try_with_id_and_options(Uuid::now_v7(), provider, options)
    }

    /// Creates a context using the supplied ID and runtime settings.
    ///
    /// Runtime settings are ignored for a no-op exporter.
    pub fn try_with_id_and_options<P>(
        id: Uuid,
        provider: P,
        options: RuntimeOptions,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        M: crate::build_info::ModelSource + ObserverBuilder<P>,
        P: ContextExporter,
    {
        let inner = if provider.is_noop() {
            ContextInner::noop(id)
        } else {
            ContextInner::try_new_with_options(id, options)?
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
