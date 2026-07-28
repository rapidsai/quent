// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The runtime host that observers of a model instance run on.

use crate::observer::{Observer, spawn_forwarder};
use quent_events::EntityEvent;
use quent_io::ExporterProvider;
use std::future::Future;
use std::sync::Arc;
use tokio::runtime::{Handle, Runtime};
use tracing::debug;
use uuid::Uuid;

/// The runtime an active context's observers run on.
#[derive(Clone)]
pub(crate) enum BackendRuntime {
    /// A handle to a runtime owned elsewhere (`#[tokio::main]`, a caller-managed
    /// one) and kept alive by that owner.
    Borrowed(Handle),
    /// The runtime this context spawned, shared by the context and every observer
    /// (hence `Arc`) and shut down by the last holder's `Drop`.
    Owned {
        handle: Handle,
        /// `Option` only so `Drop` can move the `Arc` out of `&mut self`; `Some`
        /// for the value's whole life until then.
        runtime: Option<Arc<Runtime>>,
    },
}

impl BackendRuntime {
    /// The handle observers spawn and block on.
    pub(crate) fn handle(&self) -> Handle {
        match self {
            Self::Borrowed(handle) | Self::Owned { handle, .. } => handle.clone(),
        }
    }
}

impl Drop for BackendRuntime {
    fn drop(&mut self) {
        // On the last holder of a spawned runtime, shut it down without blocking,
        // since a blocking `Runtime` drop panics on a runtime worker thread.
        // `into_inner` yields the `Runtime` only when this was the final `Arc`.
        // Safe to abandon tasks here: the observers' forwarders have already
        // flushed by the time the last holder drops.
        if let Self::Owned { runtime, .. } = self
            && let Some(runtime) = runtime.take().and_then(Arc::into_inner)
        {
            runtime.shutdown_background();
        }
    }
}

/// What a context does with events. `Noop` drops them; `Active` runs its
/// observers' forwarders on the carried runtime.
enum Backend {
    Noop,
    Active { runtime: BackendRuntime },
}

/// A context responsible for providing an asynchronous back-end to a
/// synchronous context generated from an application event model.
///
/// Instrumented application code should not interact with this type directly
/// unless there is a very special reason. Instead, it should interact with the
/// generated context only through a fully synchronous API.
///
/// What it is responsible for:
/// - Resolving the runtime its observers run on. It borrows an ambient one if
///   present, otherwise spawns its own (see [`BackendRuntime`]).
/// - Being the single sync→async bridge for async observer construction and
///   the drop-time flush.
///
/// # Panics
///
/// The blocking sync/async crossings work off a runtime or on a multi-threaded
/// one, but panic on a current-thread runtime.
#[doc(hidden)]
pub struct Context {
    /// Unique identifier of this context.
    id: Uuid,
    /// The asynchronous run-time the observers produced by this context operate
    /// on.
    backend: Backend,
}

impl Context {
    /// Construct an active context adopting `id`, with a runtime for its
    /// observers' forwarders.
    pub fn try_new(id: Uuid) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Context {
            id,
            backend: Backend::Active {
                runtime: resolve_runtime()?,
            },
        })
    }

    /// Construct a no-op context: observers built from it discard events.
    pub fn noop(id: Uuid) -> Self {
        debug!("using noop context");
        Context {
            id,
            backend: Backend::Noop,
        }
    }

    /// Return the universally unique identifier of this context.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Drive `fut` to completion on this context's runtime, blocking the
    /// calling thread.
    ///
    /// # Panics
    ///
    /// Panics on a current-thread runtime.
    pub fn block_on<F: Future>(&self, fut: F) -> F::Output {
        match self.runtime() {
            Some(runtime) => drive(&runtime.handle(), fut),
            // A noop context has no runtime, but its async work is immediately
            // ready, so poll once. Invariant: the noop `observer()` future
            // must never pend (it early-returns before any `.await`). The
            // `unreachable!` below enforces it.
            None => {
                let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
                match std::pin::pin!(fut).poll(&mut cx) {
                    std::task::Poll::Ready(v) => v,
                    std::task::Poll::Pending => {
                        unreachable!("noop context future is always ready")
                    }
                }
            }
        }
    }

    /// The runtime backing an active context; `None` for noop.
    fn runtime(&self) -> Option<&BackendRuntime> {
        match &self.backend {
            Backend::Active { runtime } => Some(runtime),
            Backend::Noop => None,
        }
    }

    /// Create an [`Observer`] of events of one *type* of entity `T`, building its
    /// exporter from `provider` bound to this context's id.
    ///
    /// The exporter is constructed here (so construction errors surface through
    /// this call) and only then moved into the spawned forwarder task. A noop
    /// context builds no exporter.
    pub async fn observer<T>(
        &self,
        provider: impl ExporterProvider<T>,
    ) -> Result<Observer<T>, Box<dyn std::error::Error>>
    where
        T: Send + EntityEvent + 'static,
    {
        let Some(runtime) = self.runtime() else {
            return Ok(Observer::noop());
        };
        let exporter = provider.create_exporter(self.id).await?;
        Ok(spawn_forwarder(runtime, exporter))
    }
}

/// Resolve the runtime observers run on: borrow an ambient one if present,
/// otherwise spawn a fresh owned runtime.
fn resolve_runtime() -> Result<BackendRuntime, Box<dyn std::error::Error>> {
    if let Ok(handle) = Handle::try_current() {
        debug!("using existing async runtime");
        Ok(BackendRuntime::Borrowed(handle))
    } else {
        debug!("spawning new async runtime");
        let runtime = Runtime::new().map_err(|e| format!("unable to spawn async runtime: {e}"))?;
        Ok(BackendRuntime::Owned {
            handle: runtime.handle().clone(),
            runtime: Some(Arc::new(runtime)),
        })
    }
}

/// Drive `fut` to completion on `handle`'s runtime, blocking the current thread.
///
/// Off a runtime, it blocks directly. On a multi-threaded runtime worker it
/// uses `block_in_place` so the scheduler keeps progressing.
///
/// # Panics
/// On a current-thread runtime, this panics.
pub(crate) fn drive<F: Future>(handle: &Handle, fut: F) -> F::Output {
    if Handle::try_current().is_ok() {
        tokio::task::block_in_place(|| handle.block_on(fut))
    } else {
        handle.block_on(fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_context_has_noop_backend() {
        let ctx = Context::noop(Uuid::now_v7());
        assert!(matches!(ctx.backend, Backend::Noop));
    }
}
