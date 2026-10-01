// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Routes NVTX events to a user hook and releases the hook when capture ends.
//! Callbacks already running keep the hook alive until they finish.
//!
//! # Mechanism
//!
//! A process that needs NVTX events to be injected into a Rust consumer calls
//! [`install_hook`] with a callback that forwards events to that consumer. The
//! process-wide [`HOOK`] static stores the callback in [`HookState`] behind a
//! [`Mutex`]. When a thread first captures an event, it creates a [`ThreadHook`]
//! that `HookState` owns through an [`Arc`]. The thread keeps a [`CachedHook`]
//! containing a [`Weak`] reference to that handle. For each event, [`dispatch`]
//! upgrades this reference to an `Arc`, keeping the callback and its captured
//! resources alive until the call finishes.
//!
//! Dropping [`CaptureGuard`] disables capture and drops [`HookState`]. Each
//! `CachedHook` keeps its `Weak`, but it can no longer be upgraded once the last
//! `Arc` is dropped. A callback already running keeps its `Arc` until it returns.
//! Nested NVTX calls do not invoke the hook again.
//!
//! When a thread exits, dropping its `CachedHook` removes its `ThreadHook` from
//! `HookState` if capture is still active. A `CachedHook` alone cannot keep the
//! hook's resources alive. [`install_hook`] can succeed only once per process.
//!
//! # Why this mechanism
//!
//! A hook kept in static storage forever would also keep its observer and exporter
//! alive. A strong reference in each thread's cache would let idle threads delay
//! their release after capture ends.
//!
//! Locking one shared hook slot for every event makes producer threads compete
//! for the same lock. Cloning one shared `Arc` on every event also makes them
//! update the same reference count. Separate `ThreadHook` handles let each thread
//! update its own reference count during dispatch.
//!
//! Holding a lock while calling the hook, or waiting for all callbacks during
//! guard drop, could deadlock when the hook drops its own [`CaptureGuard`].
//! Temporary strong references let those callbacks finish without either wait.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use nvtx_events::NvtxEvent;
use thiserror::Error;

/// The stored capture hook. Sink-agnostic: it depends only on [`NvtxEvent`].
type Hook = Arc<dyn Fn(NvtxEvent) + Send + Sync + 'static>;

/// Owns the installed hook and producer registrations until capture ends.
struct HookState {
    hook: Hook,
    threads: Vec<Arc<ThreadHook>>,
}

// Each producer updates only its own handle's reference count. Keep handles on
// separate cache lines even if the allocator places them next to each other.
#[repr(align(64))]
struct ThreadHook {
    hook: Hook,
}

/// Process-wide capture state that permits one installation and subsequent removal.
static HOOK: OnceLock<Mutex<Option<HookState>>> = OnceLock::new();

/// A thread's hook registration that does not keep capture resources alive.
/// Dropping it unregisters the thread.
struct CachedHook {
    hook: Weak<ThreadHook>,
}

impl Drop for CachedHook {
    fn drop(&mut self) {
        let removed = HOOK.get().and_then(|slot| {
            let mut state = slot.lock().unwrap();
            let state = state.as_mut()?;
            let index = state
                .threads
                .iter()
                .position(|hook| Arc::as_ptr(hook) == self.hook.as_ptr())?;
            Some(state.threads.swap_remove(index))
        });
        drop(removed);
    }
}

thread_local! {
    /// Hook registration scoped to the calling thread's lifetime.
    static CACHED_HOOK: std::cell::OnceCell<Option<CachedHook>> = const {
        std::cell::OnceCell::new()
    };
}

/// Acquire a hook handle that retains its captured resources until released.
///
/// Returns `None` if no hook is available. Supports calls after TLS teardown.
fn acquire_hook() -> Option<Arc<ThreadHook>> {
    CACHED_HOOK
        .try_with(|cache| {
            cache
                .get_or_init(|| {
                    let mut state = HOOK.get()?.lock().unwrap();
                    let state = state.as_mut()?;
                    let hook = Arc::new(ThreadHook {
                        hook: Arc::clone(&state.hook),
                    });
                    let cache = CachedHook {
                        hook: Arc::downgrade(&hook),
                    };
                    state.threads.push(hook);
                    Some(cache)
                })
                .as_ref()?
                .hook
                .upgrade()
        })
        .unwrap_or_else(|_| {
            // Late process cleanup can emit NVTX after this thread's cache has
            // been destroyed. It still needs capture if the guard remains alive.
            let state = HOOK.get()?.lock().unwrap();
            let state = state.as_ref()?;
            Some(Arc::new(ThreadHook {
                hook: Arc::clone(&state.hook),
            }))
        })
}

// Ordinary static storage outlives both the capture owner and Rust TLS, so
// callbacks can consult it during late process cleanup.
// The hook slot controls dispatch admission; this flag avoids event conversion.
static CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Error returned by [`install_hook`].
#[derive(Debug, Error)]
pub enum InstallHookError {
    /// A hook was already installed; installation is one-shot per process.
    #[error("an NVTX capture hook is already installed (install_hook is one-shot per process)")]
    AlreadyInstalled,
}

/// Install the process-global, sink-agnostic capture hook.
///
/// The hook receives converted [`NvtxEvent`]s while capture is active.
/// Installation is one-shot per process, even after capture ends. The first
/// captured event on each thread allocates a dispatch handle.
///
/// Capture is active until the returned [`CaptureGuard`] is dropped. The hook
/// can own its sink directly:
///
/// ```ignore
/// let pipeline = /* the sink the hook forwards into */;
/// let _capture = nvtx_injection::install_hook(move |event| pipeline.emit(event))?;
/// // ... annotated work ...
/// // `_capture` disables capture and releases the hook and its sink.
/// ```
///
/// # Errors
/// Returns [`InstallHookError::AlreadyInstalled`] if a hook was already set. A
/// failed caller receives no guard, so it cannot end another owner's capture.
pub fn install_hook<F>(hook: F) -> Result<CaptureGuard, InstallHookError>
where
    F: Fn(NvtxEvent) + Send + Sync + 'static,
{
    HOOK.set(Mutex::new(Some(HookState {
        hook: Arc::new(hook),
        threads: Vec::new(),
    })))
    .map_err(|_| InstallHookError::AlreadyInstalled)?;
    CAPTURE_ACTIVE.store(true, Ordering::Relaxed);
    Ok(CaptureGuard { _private: () })
}

/// Ownership of the active NVTX capture, returned by a successful
/// [`install_hook`].
///
/// Dropping the guard disables capture and removes the hook without waiting for
/// callbacks that already acquired it. Those callbacks may still invoke the hook
/// and retain its captured resources until they finish. Stop and join
/// NVTX-producing threads before ending capture if the sink must flush by then.
/// The guard may be dropped from inside the hook. Removal takes `O(t)` work for
/// `t` live producer threads, excluding destruction of captured resources.
///
/// Callback pointers remain installed: callbacks keep
/// synthesizing handles, ids, and nesting levels for the app. Capture cannot be
/// restarted.
///
/// If the guard is never dropped (`std::mem::forget`, `std::process::exit`),
/// capture stays active until the process ends.
#[must_use = "capture stops as soon as the guard is dropped; bind it to a named variable"]
#[derive(Debug)]
pub struct CaptureGuard {
    _private: (),
}

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        CAPTURE_ACTIVE.store(false, Ordering::Relaxed);
        let hook = HOOK.get().and_then(|slot| slot.lock().unwrap().take());
        // Captured resources can run user code on drop, including NVTX callbacks.
        // Release them outside the slot lock, just as for hook invocation.
        drop(hook);
    }
}

thread_local! {
    /// Whether this thread is inside [`dispatch`]. Const-initialized and
    /// drop-free, so it stays accessible during late process cleanup.
    static IN_DISPATCH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether callbacks should build an event for capture.
///
/// Hook acquisition in [`dispatch`] handles capture ending during conversion.
#[inline]
pub(crate) fn capture_active() -> bool {
    CAPTURE_ACTIVE.load(Ordering::Relaxed)
}

/// Dispatch a converted event to the installed hook while capture is active.
/// A call that acquires the hook may still invoke it after the
/// [`CaptureGuard`] is dropped.
pub(crate) fn dispatch(event: NvtxEvent) {
    // Guard against hook-induced re-entry: if the hook (or code it calls) emits
    // NVTX, it would recurse into this synchronous dispatch path and overflow
    // the stack, bypassing the callbacks' panic barriers. Drop nested events.
    if IN_DISPATCH.with(|g| g.replace(true)) {
        return;
    }
    // RAII exit so the reentry flag is cleared even if the hook unwinds.
    struct Exit;
    impl Drop for Exit {
        fn drop(&mut self) {
            IN_DISPATCH.with(|g| g.set(false));
        }
    }
    let _exit = Exit;

    // Only this thread can upgrade its cached handle. Reentry is suppressed,
    // so removal drops the last idle owner; an acquired handle lasts only until
    // this invocation returns. Idle TLS caches hold no captured resources.
    if let Some(hook) = acquire_hook() {
        (hook.hook)(event);
    }
}
