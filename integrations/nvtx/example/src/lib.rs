// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! In-process NVTX capture, driven by the application.
//!
//! [`run_capture`] wires the NVTX injection hook into a Quent event pipeline
//! built on a caller-supplied exporter, runs a fixed set of NVTX annotations,
//! and flushes. The binary debug-prints captured events; the test reuses the
//! same routine with a collecting exporter — one code path, no subprocess or
//! files.
//!
//! Capture is in-process: this crate links `nvtx-injection` with its
//! `static-injection` feature, so NVTX initializes injection at the first NVTX
//! call in whatever binary links the crate.

use std::ffi::CString;
use std::sync::{Arc, Barrier};

use nvtx_bridge::NvtxEventEntity;
use quent_instrumentation::{ContextInner, EventCallback};
use uuid::Uuid;

/// Capture the NVTX events produced by the fixed annotation sequence into
/// `exporter`.
///
/// Builds a Quent context and event pipeline on `exporter`, installs the injection
/// hook (one-shot per process) to forward each event, runs the annotations, and
/// drops the pipeline to flush.
pub fn run_capture(
    session: Uuid,
    exporter: EventCallback<NvtxEventEntity>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_capture_n_threads(1, session, exporter)
}

/// Like [`run_capture`] but spawns `n` threads inside the observer window, each
/// doing a push/pop range, so callers can assert per-thread identity.
///
/// A [`Barrier`] synchronizes all threads before their first NVTX call so their
/// push/pop events are interleaved in real time rather than serialised.
pub fn run_capture_n_threads(
    n: usize,
    session: Uuid,
    exporter: EventCallback<NvtxEventEntity>,
) -> Result<(), Box<dyn std::error::Error>> {
    let context = ContextInner::try_new(session)?;
    let pipeline =
        context.block_on(async { context.observer::<NvtxEventEntity>(&exporter).await })?;

    // Forward each captured event into the pipeline, before the first NVTX call.
    let sender = pipeline.sender();
    nvtx_injection::install_hook(move |event| sender.emit(session, event))?;

    annotated_work_n_threads(n);

    // Dropping the pipeline drains and flushes the exporter.
    drop(pipeline);
    Ok(())
}

/// Exercise the core default-domain NVTX kinds the `nvtx` crate exposes: thread
/// naming, a mark, a push/pop range, and a start/end range guard.
///
/// When `n == 1` this runs on the calling thread. When `n > 1`, the push/pop
/// work is distributed across `n` threads that synchronise at a barrier so their
/// events interleave in time.
fn annotated_work_n_threads(n: usize) {
    if n == 1 {
        nvtx::name_thread(current_thread_id(), c"nvtx-example/main");
        nvtx::mark(c"startup");
        let phase1 = nvtx::LocalRange::new(c"phase-1");
        drop(phase1);
        let phase2 = nvtx::Range::new(c"phase-2");
        drop(phase2);
        return;
    }

    // Barrier ensures all threads reach their first NVTX call before any one
    // of them begins, so push/pop events from different threads interleave.
    let barrier = Arc::new(Barrier::new(n));
    let handles: Vec<_> = (0..n)
        .map(|i| {
            let b = Arc::clone(&barrier);
            std::thread::spawn(move || {
                b.wait();
                let name = CString::new(format!("thread-{i}"))
                    .expect("generated thread range name contains no NUL");
                let range = nvtx::LocalRange::new(name);
                drop(range);
            })
        })
        .collect();
    for h in handles {
        h.join().expect("worker thread panicked");
    }
}

fn current_thread_id() -> u32 {
    // The glibc `gettid()` wrapper requires glibc >= 2.30, but Quent's Conda
    // toolchain supports an older sysroot. The Linux syscall is stable across
    // every architecture supported by this Linux-64-only integration.
    // SAFETY: `gettid` takes no arguments and returns the caller's kernel task id.
    unsafe { libc::syscall(libc::SYS_gettid) as u32 }
}
