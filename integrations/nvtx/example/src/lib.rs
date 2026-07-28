// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! In-process NVTX capture, driven by the application.
//!
//! [`run_capture`] wires the NVTX injection hook into a Quent `Observer` built
//! on a caller-supplied exporter, runs a fixed set of NVTX annotations, and
//! flushes. The binary debug-prints captured events; the test reuses the same
//! routine with a collecting exporter — one code path, no subprocess or files.
//!
//! Capture is in-process: this crate links `nvtx-injection` with its
//! `static-injection` feature, so NVTX initializes injection at the first NVTX
//! call in whatever binary links the crate.

use nvtx_bridge::NvtxEventEntity;
use quent_instrumentation::{Context, EventCallback};
use uuid::Uuid;

/// Capture the NVTX events produced by the fixed annotation sequence into
/// `exporter`.
///
/// Builds a Quent context and observer on `exporter`, installs the injection
/// hook (one-shot per process) to forward each event, runs the annotations, and
/// drops the observer to flush.
pub fn run_capture(
    session: Uuid,
    exporter: EventCallback,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = Context::try_new(session)?;
    let observer = ctx.block_on(async { ctx.observer::<NvtxEventEntity>(exporter).await })?;

    // Forward each captured event into the observer, before the first NVTX call.
    let sender = observer.sender();
    nvtx_injection::install_hook(move |event| sender.emit(session, event))?;

    annotated_work();

    // Dropping the observer drains and flushes the exporter.
    drop(observer);
    Ok(())
}

/// Exercise the core default-domain NVTX kinds the `nvtx` crate exposes: thread
/// naming, a mark, a push/pop range, and a start/end range guard.
fn annotated_work() {
    nvtx::name_thread!("nvtx-example/main");
    nvtx::mark!("startup");

    nvtx::range_push!("phase-1");
    nvtx::range_pop!();

    let phase2 = nvtx::range!("phase-2");
    drop(phase2);
}
