// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `quent-nvtx`: the bridge from captured NVTX events into Quent's event
//! pipeline, plus the self-configuring capture cdylib NVTX loads via
//! `NVTX_INJECTION64_PATH`.
//!
//! # Two roles, one crate
//!
//! - **rlib (the [`install`] primitive):** fronts Quent's *unbounded*
//!   `EventSender` with a *bounded*, lock-free [`crossbeam_queue::ArrayQueue`]
//!   ring plus a drop-and-count overflow policy (CAP-05 / D-16). A dedicated
//!   drain thread pops the ring and forwards each event to the exporter, and the
//!   injection hook stamps every [`NvtxEvent`] with a capture timestamp via
//!   [`Event::new_now`] (CAP-04). This is the only Phase-1 crate that touches
//!   Quent internals (D-03).
//! - **cdylib (the capture library):** the same image also contains
//!   `quent-nvtx-injection` (its `InitializeInjectionNvtx2` entry and callback
//!   table). When NVTX `dlopen`s this `.so` via `NVTX_INJECTION64_PATH`, an ELF
//!   `.init_array` constructor reads `QUENT_NVTX_OUTPUT_DIR` (+ an optional
//!   `QUENT_NVTX_SESSION`), builds an ndjson pipeline, and calls [`install`] —
//!   so the hook is set in the *same* module whose callbacks NVTX invokes. A
//!   `.fini_array` destructor drops the pipeline at exit to flush the file.
//!
//! ## Why the cdylib self-configures (and the app does not call [`install`])
//!
//! NVTX resolves its injection library by `dlopen(NVTX_INJECTION64_PATH,
//! RTLD_LAZY)` and calls *that library's* `InitializeInjectionNvtx2`; the
//! callbacks that fire read *that library's* `HOOK`. Because `dlopen` uses
//! `RTLD_LOCAL`, a hook installed in a copy of the injection code linked into
//! the *application* is a different, unreachable static. The only module whose
//! hook the callbacks read is the loaded cdylib — so the sink must be installed
//! *inside* the cdylib, at load, from environment configuration.

// The capture cdylib and its injection dependency are Linux 64-bit only (D-04);
// `quent-nvtx-injection` enforces this with its own `compile_error!`.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_queue::ArrayQueue;
use quent_events::{EntityEvent, Event};
use quent_instrumentation::{Context, Observer};
use quent_io::{FileSystemExporterOptions, FileSystemFormat};
use quent_nvtx_events::{NvtxEvent, NvtxEventKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The `Event<T>` payload carried through Quent's pipeline for the NVTX stream.
///
/// A `#[serde(transparent)]` newtype over [`NvtxEventKind`] that implements the
/// *real* [`quent_events::EntityEvent`] the pipeline requires. `NvtxEventKind`
/// itself only implements the vocabulary crate's local `EntityEvent` (to keep
/// `quent-nvtx-events` free of Quent-internal dependencies, D-03); the orphan
/// rule blocks implementing the real trait for it here, so this bridge-local
/// newtype adapts it. Being transparent, its ndjson encoding is identical to a
/// bare [`NvtxEvent`], and its [`EntityEvent::NAME`] keeps the `"NvtxEvent"`
/// entity directory name.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NvtxEventEntity(pub NvtxEventKind);

impl EntityEvent for NvtxEventEntity {
    const NAME: &'static str = "NvtxEvent";
}

impl From<NvtxEvent> for NvtxEventEntity {
    fn from(event: NvtxEvent) -> Self {
        Self(NvtxEventKind::from(event))
    }
}

/// Fixed capacity of the bounded hot-path ring (D-16). Large enough that the
/// deterministic test-app never overflows, small relative to sustained
/// overload where drop-and-count (CAP-05) takes over.
const RING_CAPACITY: usize = 1 << 16;

/// Backstop poll interval for the drain thread when the ring is momentarily
/// empty. The producer never signals the drain (that would add work to the hot
/// path); a short `park_timeout` keeps latency low without a busy `pop` loop.
const DRAIN_POLL: Duration = Duration::from_millis(1);

/// Bound on how long [`Capture::drop`] waits for the drain thread to finish its
/// shutdown flush before abandoning it (WR-01). Teardown runs from a
/// `.fini_array` finalizer, possibly under the dynamic-loader lock, so an
/// unbounded join risks wedging process exit; this deadline caps that risk.
const TEARDOWN_JOIN_TIMEOUT: Duration = Duration::from_secs(2);

/// Process-global count of events dropped because the ring was full (D-07).
///
/// This is a process-cumulative counter that is never reset (IN-02); it counts
/// every drop across the process lifetime, not per-[`Capture`].
static DROPPED: AtomicU64 = AtomicU64::new(0);

/// The bounded, lock-free hand-off between the injection hook (producer, app
/// thread) and the drain thread (consumer).
type Ring = Arc<ArrayQueue<Event<NvtxEventEntity>>>;

/// Total number of events dropped so far because the ring was full.
///
/// The count is **process-cumulative** and never reset (IN-02): it reflects
/// every drop since the process started, across all [`Capture`] instances.
pub fn dropped() -> u64 {
    DROPPED.load(Ordering::Relaxed)
}

/// Non-blocking push with drop-and-count overflow (CAP-05 / D-07).
///
/// Returns immediately whether the event was enqueued or dropped; the producing
/// (application) thread is never blocked.
#[inline]
fn push_or_drop(ring: &ArrayQueue<Event<NvtxEventEntity>>, event: Event<NvtxEventEntity>) {
    if ring.push(event).is_err() {
        DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

/// Drain the ring into `observer` until `shutdown`, then drain the remainder.
///
/// Models the drain/shutdown-drain discipline of `spawn_forwarder`
/// (`crates/instrumentation/src/observer.rs`): forward everything available,
/// and on shutdown drain what is left before returning. `observer.send`
/// forwards through the unbounded `EventSender::send` (non-blocking).
fn drain_loop(ring: &Ring, observer: &Observer<NvtxEventEntity>, shutdown: &AtomicBool) {
    loop {
        while let Some(event) = ring.pop() {
            observer.send(event);
        }
        if shutdown.load(Ordering::Acquire) {
            // Final drain: catch anything pushed between the last `pop` and the
            // shutdown store.
            while let Some(event) = ring.pop() {
                observer.send(event);
            }
            break;
        }
        std::thread::park_timeout(DRAIN_POLL);
    }
}

/// A running capture pipeline. Dropping it stops the drain thread, drains the
/// remainder, and drops the exporter (flushing it).
#[must_use = "dropping the Capture immediately tears down the pipeline"]
pub struct Capture {
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Capture {
    /// Events dropped so far because the ring was full.
    ///
    /// Reads the same **process-cumulative** counter as the free [`dropped`]
    /// function (IN-02): despite the `&self` receiver this is not a per-instance
    /// count — it aggregates drops across every `Capture` in the process.
    pub fn dropped(&self) -> u64 {
        DROPPED.load(Ordering::Relaxed)
    }
}

impl Drop for Capture {
    /// Bounded, non-wedging teardown (WR-01).
    ///
    /// Joining the drain thread runs its owned tokio runtime shutdown and the
    /// ndjson exporter flush. On the cdylib path this `Drop` fires from a
    /// `.fini_array` finalizer, which on `dlclose` runs under the dynamic-loader
    /// lock — the very context [`install`] carefully keeps heavy work out of. An
    /// unbounded `handle.join()` here could therefore hang process exit if the
    /// exporter flush stalls, breaking the instrumented app's ability to exit
    /// cleanly (a hard project constraint). To stay non-wedging we run the
    /// blocking join on a short-lived detached watchdog thread and wait on a
    /// channel with [`TEARDOWN_JOIN_TIMEOUT`]; if the flush does not complete in
    /// time we log and abandon it (leaking the drain handle), preferring a lost
    /// tail-flush over a wedged process.
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        let Some(handle) = self.handle.take() else {
            return;
        };
        handle.thread().unpark();

        // Surface silent capture degradation at teardown (WR-05): if events were
        // dropped, the ring overflowed and the capture is incomplete.
        let dropped = DROPPED.load(Ordering::Relaxed);
        if dropped > 0 {
            eprintln!(
                "quent-nvtx: {dropped} event(s) were dropped (ring overflow); capture is incomplete"
            );
        }

        // Move the blocking join onto a detached watchdog so a stuck exporter
        // flush cannot wedge process exit; wait on the channel with a deadline.
        let (tx, rx) = std::sync::mpsc::sync_channel::<()>(1);
        if std::thread::Builder::new()
            .name("quent-nvtx-teardown".to_owned())
            .spawn(move || {
                let _ = handle.join();
                // The receiver may have already timed out and gone away.
                let _ = tx.send(());
            })
            .is_err()
        {
            // Spawn failed: the drain thread's handle was moved into (and
            // dropped by) the failed closure, so it is already detached. Do not
            // block the loader waiting on a join we can no longer observe.
            eprintln!(
                "quent-nvtx: could not spawn teardown watchdog; abandoning drain-thread flush"
            );
            return;
        }
        if rx.recv_timeout(TEARDOWN_JOIN_TIMEOUT).is_err() {
            eprintln!(
                "quent-nvtx: capture drain thread did not finish flushing within \
                 {TEARDOWN_JOIN_TIMEOUT:?}; abandoning flush to avoid wedging process exit"
            );
        }
    }
}

/// Error returned by [`install`].
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    /// The drain thread could not be spawned.
    #[error("failed to spawn the capture drain thread: {0}")]
    Spawn(String),
    /// The injection hook was already installed (one-shot per process).
    #[error(transparent)]
    Hook(#[from] quent_nvtx_injection::InstallHookError),
}

/// Install the capture pipeline: a bounded ring drained to an ndjson (or any)
/// `Observer`, fronted by the injection hook that stamps and enqueues events.
///
/// `make_observer` is invoked **on the drain thread**, not on the caller's
/// thread. This is deliberate: the cdylib calls `install` from an ELF
/// `.init_array` constructor while the dynamic-loader lock may be held, and
/// building the tokio runtime / exporter there could deadlock. Deferring it to
/// the drain thread runs that work after the constructor returns and the loader
/// lock is released. Events captured before the observer is ready buffer in the
/// bounded ring. If `make_observer` returns `None`, capture is disabled and
/// events drop.
///
/// # Errors
/// Returns [`InstallError::Spawn`] if the drain thread cannot start, or
/// [`InstallError::Hook`] if a hook is already installed.
pub fn install<F>(session: Uuid, make_observer: F) -> Result<Capture, InstallError>
where
    F: FnOnce() -> Option<Observer<NvtxEventEntity>> + Send + 'static,
{
    let ring: Ring = Arc::new(ArrayQueue::new(RING_CAPACITY));
    let shutdown = Arc::new(AtomicBool::new(false));

    let drain_ring = Arc::clone(&ring);
    let drain_shutdown = Arc::clone(&shutdown);
    let handle = std::thread::Builder::new()
        .name("quent-nvtx-drain".to_owned())
        .spawn(move || {
            let Some(observer) = make_observer() else {
                tracing::error!("quent-nvtx: exporter unavailable; capture disabled");
                // The cdylib installs no tracing subscriber, so the message
                // above may go nowhere; emit an unconditional diagnostic too
                // (WR-05) — without a drain the ring is never emptied and every
                // captured event silently drop-and-counts forever.
                eprintln!("quent-nvtx: exporter unavailable; capture disabled (all events will be dropped)");
                return;
            };
            drain_loop(&drain_ring, &observer, &drain_shutdown);
            // `observer` drops here → cancels the forwarder, drains the channel,
            // and flushes the exporter.
        })
        .map_err(|e| InstallError::Spawn(e.to_string()))?;

    // Sink-agnostic injection hook: stamp the capture timestamp (CAP-04) and
    // hand off to the bounded ring (CAP-05). The application thread never blocks.
    quent_nvtx_injection::install_hook(move |event: NvtxEvent| {
        push_or_drop(&ring, Event::new_now(session, NvtxEventEntity::from(event)));
    })?;

    Ok(Capture {
        shutdown,
        handle: Some(handle),
    })
}

/// Build an ndjson `Observer` writing under `dir/<session>/NvtxEvent/`.
///
/// Runs on the drain thread (see [`install`]). With no ambient tokio runtime it
/// spawns an owned one, which the returned `Observer` keeps alive for its
/// lifetime.
fn build_ndjson_observer(
    session: Uuid,
    dir: PathBuf,
) -> Result<Observer<NvtxEventEntity>, Box<dyn std::error::Error>> {
    let ctx = Context::try_new(session)?;
    let options = FileSystemExporterOptions::new(FileSystemFormat::Ndjson, dir);
    // `Context::observer` builds the exporter itself (via the provider) on the
    // context's runtime, so construction errors surface here on the drain thread.
    ctx.block_on(async { ctx.observer::<NvtxEventEntity>(options).await })
}

/// The self-configuring capture cdylib: install an ndjson pipeline at load and
/// flush it at exit. Linux-only (ELF `.init_array` / `.fini_array`, D-04).
#[cfg(target_os = "linux")]
mod cdylib {
    use std::sync::Mutex;

    use super::{Capture, build_ndjson_observer, install};
    use uuid::Uuid;

    /// The live pipeline, so the `.fini_array` destructor can drop (flush) it.
    static CAPTURE: Mutex<Option<Capture>> = Mutex::new(None);

    /// Runs at `dlopen`, before NVTX calls `InitializeInjectionNvtx2`. Kept
    /// minimal: read env, spawn the drain thread, install the hook. The runtime
    /// and exporter are built on the drain thread (after this returns), avoiding
    /// a dynamic-loader-lock deadlock.
    extern "C" fn init() {
        let Ok(dir) = std::env::var("QUENT_NVTX_OUTPUT_DIR") else {
            // No capture configured: leave the injection library dormant.
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        let session = std::env::var("QUENT_NVTX_SESSION")
            .ok()
            .and_then(|s| Uuid::parse_str(&s).ok())
            .unwrap_or_else(Uuid::now_v7);

        match install(session, move || build_ndjson_observer(session, dir).ok()) {
            Ok(capture) => match CAPTURE.lock() {
                Ok(mut slot) => *slot = Some(capture),
                Err(_) => {
                    // The slot mutex is poisoned. Do NOT let `capture` drop here:
                    // its `Drop` joins the drain thread (tokio shutdown + flush),
                    // and we are running inside the `.init_array` constructor
                    // under the dynamic-loader lock — exactly the context
                    // `install` is designed to keep heavy work out of (WR-04).
                    // Deliberately leak the pipeline instead; the process is
                    // already degraded and leaking is safe at load time.
                    std::mem::forget(capture);
                    eprintln!(
                        "quent-nvtx: capture slot mutex poisoned; leaking pipeline to avoid a loader-lock join"
                    );
                }
            },
            Err(e) => {
                // A cdylib must never panic across the C ABI / loader; log only.
                eprintln!("quent-nvtx: capture install failed: {e}");
            }
        }
    }

    /// Runs at `dlclose` / process exit: drop the pipeline to drain and flush.
    extern "C" fn fini() {
        if let Ok(mut slot) = CAPTURE.lock() {
            slot.take();
        }
    }

    #[used]
    #[unsafe(link_section = ".init_array")]
    static INIT: extern "C" fn() = init;

    #[used]
    #[unsafe(link_section = ".fini_array")]
    static FINI: extern "C" fn() = fini;
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use quent_io::{FileSystemFormat, ImporterOptions, ImporterProvider, filesystem};
    use quent_nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage};

    use super::*;

    fn range_push(domain: u64, label: &str) -> NvtxEvent {
        NvtxEvent::RangePush {
            domain,
            attributes: NvtxEventAttributes {
                category: 0,
                color: None,
                message: Some(NvtxMessage::String(label.to_owned())),
                payload: None,
            },
        }
    }

    fn read_events(entity_dir: &std::path::Path) -> Vec<Event<NvtxEventEntity>> {
        let importer = <ImporterOptions as ImporterProvider<NvtxEventEntity>>::create_importer(
            &ImporterOptions::FileSystem(filesystem::importer::Options {
                format: FileSystemFormat::Ndjson,
                path: entity_dir.to_path_buf(),
            }),
        )
        .expect("importer");
        importer.collect()
    }

    /// CAP-05 / D-08: a full ring drops-and-counts and the producer never blocks.
    #[test]
    fn ring_drops_and_counts_when_full() {
        let ring = ArrayQueue::<Event<NvtxEventEntity>>::new(2);
        let session = Uuid::now_v7();

        let before = DROPPED.load(Ordering::Relaxed);
        let start = Instant::now();
        for i in 0..5u64 {
            push_or_drop(
                &ring,
                Event::new_now(
                    session,
                    NvtxEventEntity::from(NvtxEvent::RangePop { domain: i }),
                ),
            );
        }
        let elapsed = start.elapsed();

        // Non-blocking: five pushes over a capacity-2 ring return effectively
        // instantly rather than parking the producer.
        assert!(
            elapsed < Duration::from_millis(100),
            "producer blocked: {elapsed:?}"
        );
        assert_eq!(ring.len(), 2, "ring holds exactly its capacity");
        assert_eq!(
            DROPPED.load(Ordering::Relaxed) - before,
            3,
            "the three overflow pushes are dropped and counted"
        );
    }

    /// The drain thread forwards every ring event, in order, through
    /// `EventSender::send` to a real ndjson exporter, with no drops under
    /// capacity.
    #[test]
    fn drain_forwards_events_in_order_to_ndjson() {
        let dir = tempfile::tempdir().unwrap();
        let session = Uuid::now_v7();
        let ring: Ring = Arc::new(ArrayQueue::new(1024));
        let shutdown = Arc::new(AtomicBool::new(false));

        // Buffer events in the ring before the drain observer is even built.
        let n = 8u64;
        for i in 0..n {
            ring.push(Event::new_now(
                session,
                NvtxEventEntity::from(range_push(i, "quent-nvtx-unit")),
            ))
            .expect("ring under capacity");
        }

        let drain_ring = Arc::clone(&ring);
        let drain_shutdown = Arc::clone(&shutdown);
        let dir_path = dir.path().to_path_buf();
        let handle = std::thread::spawn(move || {
            let observer = build_ndjson_observer(session, dir_path).expect("ndjson observer");
            drain_loop(&drain_ring, &observer, &drain_shutdown);
        });

        shutdown.store(true, Ordering::Release);
        handle.thread().unpark();
        handle.join().unwrap();

        let entity_dir = dir.path().join(session.to_string()).join("NvtxEvent");
        let events = read_events(&entity_dir);
        assert_eq!(
            events.len(),
            n as usize,
            "every buffered event was forwarded"
        );
        for (i, event) in events.iter().enumerate() {
            assert!(event.timestamp > 0, "capture timestamp populated (CAP-04)");
            match &event.data.0.0 {
                NvtxEvent::RangePush { domain, .. } => {
                    assert_eq!(*domain, i as u64, "FIFO order preserved");
                }
                other => panic!("unexpected event: {other:?}"),
            }
        }
    }
}
