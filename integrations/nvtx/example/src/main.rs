// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! In-process NVTX capture, driven by the application.
//!
//! The program owns its Quent [`Context`] and exporter, annotates its code via
//! the NVTX Rust API, and links `nvtx-injection`'s `static-injection`
//! feature so NVTX initializes injection in-process — no cdylib, no
//! `NVTX_INJECTION64_PATH`.
//!
//! ```text
//! pixi run cargo run -p nvtx-example -- /tmp/nvtx-capture
//! ```
//!
//! Writes ndjson under `<dir>/<session>/NvtxEvent/`.

use std::path::PathBuf;

use nvtx_bridge::NvtxEventEntity;
use quent_instrumentation::Context;
use quent_io::{FileSystemExporterOptions, FileSystemFormat};
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Output root: first CLI argument, else a temp dir.
    let out_dir: PathBuf = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("nvtx-example"));

    // Session id: second CLI argument if given (lets a harness locate the
    // output deterministically), else a fresh one. A supplied-but-invalid id is
    // an error, not a silent fresh id.
    let session = match std::env::args_os().nth(2) {
        Some(arg) => {
            let text = arg.to_str().ok_or("session id must be valid UTF-8")?;
            Uuid::parse_str(text)?
        }
        None => Uuid::now_v7(),
    };

    // The app owns the context and picks the exporter (ndjson here; a real app
    // would use its configured pipeline).
    let entity_dir = out_dir.join(session.to_string()).join("NvtxEvent");
    let ctx = Context::try_new(session)?;
    let options = FileSystemExporterOptions::new(FileSystemFormat::Ndjson, out_dir);
    let observer = ctx.block_on(async { ctx.observer::<NvtxEventEntity>(options).await })?;

    // Forward captured NVTX events into the observer, before the first NVTX call
    // (the hook is one-shot); `static-injection` initializes injection in-process.
    let sender = observer.sender();
    nvtx_injection::install_hook(move |event| {
        sender.emit(session, event);
    })?;

    // Ordinary work, annotated with the NVTX Rust API — every call is captured.
    run_annotated_work();

    // Flush by dropping the observer.
    drop(observer);

    println!("captured NVTX events under {}", entity_dir.display());
    Ok(())
}

/// Exercise the core default-domain NVTX kinds the `nvtx` crate exposes: thread
/// naming, a mark, a push/pop range, and a start/end range guard.
fn run_annotated_work() {
    nvtx::name_thread!("nvtx-example/main");
    nvtx::mark!("startup");

    nvtx::range_push!("phase-1");
    nvtx::range_pop!();

    let phase2 = nvtx::range!("phase-2");
    drop(phase2);
}
