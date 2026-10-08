// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The runtime host that observers of a model instance run on.

use crate::RuntimeOptions;
use crate::observer::{ObserverInner, spawn_forwarder};
use quent_events::EventPayload;
use quent_io::ExporterProvider;
use std::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::runtime::{Handle, Runtime as TokioRuntime};
use tracing::debug;
use uuid::Uuid;

/// An owned runtime shared by an active context and its observers.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) struct Runtime {
    // Present until `Drop` takes ownership for non-blocking shutdown.
    runtime: Option<TokioRuntime>,
}

impl Runtime {
    /// The handle observers spawn and block on.
    pub(crate) fn handle(&self) -> &Handle {
        // The runtime is always present until `Drop` takes it, after which this
        // object is no longer accessible.
        self.runtime.as_ref().unwrap().handle()
    }

    /// Drive `fut` to completion, blocking the calling thread.
    ///
    /// # Panics
    ///
    /// Panics on a current-thread runtime.
    pub(crate) fn block_on<F: Future>(&self, fut: F) -> F::Output {
        #[cfg(not(target_arch = "wasm32"))]
        if Handle::try_current().is_ok() {
            return tokio::task::block_in_place(|| self.handle().block_on(fut));
        }
        self.handle().block_on(fut)
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        // Observers flush before releasing their runtime ownership. Shut down
        // without blocking because the final owner may drop in an async context.
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

/// The runtime host for a synchronous context generated from an application
/// event model.
///
/// Instrumented application code should not interact with this type directly
/// unless there is a very special reason. Instead, it should interact with the
/// generated context only through a fully synchronous API.
///
/// Hidden because [`crate::Context`] provides the model-level API.
///
/// What it is responsible for:
/// - Initializing and retaining the runtime used by its observers, which share
///   ownership until the final owner is dropped (see [`Runtime`]).
/// - Constructing observers retained by [`crate::Context`] and providing the
///   synchronous bridge for their construction and drop-time flush.
///
/// # Panics
///
/// The blocking sync/async crossings work off a runtime or on a multi-threaded
/// one, but panic on a current-thread runtime.
#[doc(hidden)]
pub struct ContextInner {
    /// Unique identifier of this context.
    id: Uuid,
    /// The asynchronous runtime used by active observers.
    runtime: Option<Arc<Runtime>>,
}

impl ContextInner {
    /// Construct an active context adopting `id`, with a runtime for its
    /// observers' forwarders.
    ///
    /// Initializes the timestamp clock, which may block during its first calibration.
    pub fn try_new(id: Uuid) -> Result<Self, Box<dyn std::error::Error>> {
        Self::try_new_with_options(id, RuntimeOptions::default())
    }

    /// Constructs an active context with the supplied ID and runtime settings.
    ///
    /// Initializes the timestamp clock, which may block during its first calibration.
    pub fn try_new_with_options(
        id: Uuid,
        options: RuntimeOptions,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        quent_time::initialize_clock();
        Ok(Self {
            id,
            runtime: Some(Arc::new(create_runtime(options)?)),
        })
    }

    /// Construct a no-op context: observers built from it discard events.
    ///
    /// Initializes the timestamp clock, which may block during its first calibration.
    pub fn noop(id: Uuid) -> Self {
        quent_time::initialize_clock();
        debug!("using noop context");
        Self { id, runtime: None }
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
            Some(runtime) => runtime.block_on(fut),
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
    fn runtime(&self) -> Option<&Arc<Runtime>> {
        self.runtime.as_ref()
    }

    /// Creates an [`ObserverInner`] for one entity event type `T`, building its
    /// exporter from `provider` bound to this context's id.
    ///
    /// The exporter is constructed here (so construction errors surface through
    /// this call) and only then moved into the spawned forwarder task. A noop
    /// context builds no exporter.
    pub async fn observer<T>(
        &self,
        provider: &impl ExporterProvider<T>,
    ) -> Result<ObserverInner<T>, Box<dyn std::error::Error>>
    where
        T: Send + EventPayload + 'static,
    {
        let Some(runtime) = self.runtime() else {
            return Ok(ObserverInner::noop());
        };
        let exporter = provider.create_exporter(self.id).await?;
        Ok(spawn_forwarder(runtime, exporter))
    }
}

/// Create an owned runtime for the context and its observers.
fn create_runtime(options: RuntimeOptions) -> Result<Runtime, Box<dyn std::error::Error>> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = options;
        Err("active instrumentation contexts are unsupported on wasm32".into())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        debug!("spawning new async runtime");
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder.enable_all();
        // Set the worker count explicitly so `TOKIO_WORKER_THREADS`, which users
        // may set for their application's runtime, does not also affect this runtime.
        let worker_threads = options
            .worker_threads
            .or_else(|| std::thread::available_parallelism().ok())
            .map_or(1, NonZeroUsize::get);
        builder.worker_threads(worker_threads);
        builder.max_blocking_threads(options.max_blocking_threads.map_or(512, NonZeroUsize::get));
        builder.thread_name(
            options
                .thread_name
                .unwrap_or_else(|| "quent-rt-worker".to_owned()),
        );
        let runtime = builder
            .build()
            .map_err(|e| format!("unable to spawn async runtime: {e}"))?;
        Ok(Runtime {
            runtime: Some(runtime),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_context_has_no_runtime() {
        let ctx = ContextInner::noop(Uuid::now_v7());
        assert!(ctx.runtime.is_none());
    }

    #[test]
    fn runtime_uses_default_thread_name() {
        let ctx = ContextInner::try_new(Uuid::now_v7()).unwrap();
        let runtime = ctx.runtime().unwrap();
        assert_eq!(
            runtime.handle().metrics().num_workers(),
            std::thread::available_parallelism().map_or(1, NonZeroUsize::get)
        );
        let thread_name = ctx.block_on(
            runtime
                .handle()
                .spawn(async { std::thread::current().name().map(str::to_owned) }),
        );
        assert_eq!(thread_name.unwrap().as_deref(), Some("quent-rt-worker"));
    }

    #[test]
    fn default_runtime_ignores_tokio_worker_threads() {
        // Run in a separate process to avoid changing other tests' environment.
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "context::tests::runtime_uses_default_thread_name",
            ])
            .env("TOKIO_WORKER_THREADS", "0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "default runtime failed with TOKIO_WORKER_THREADS=0:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    fn runtime_uses_configured_workers_and_thread_name() {
        let ctx = ContextInner::try_new_with_options(
            Uuid::now_v7(),
            RuntimeOptions {
                worker_threads: NonZeroUsize::new(1),
                thread_name: Some("quent-test-worker".to_owned()),
                ..RuntimeOptions::default()
            },
        )
        .unwrap();
        let runtime = ctx.runtime().unwrap();
        assert_eq!(runtime.handle().metrics().num_workers(), 1);
        let thread_name = ctx.block_on(
            runtime
                .handle()
                .spawn(async { std::thread::current().name().map(str::to_owned) }),
        );
        assert_eq!(thread_name.unwrap().as_deref(), Some("quent-test-worker"));
    }
}
