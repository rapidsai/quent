// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Full-coverage end-to-end capture proof (CAP-01, CAP-02, CAP-03, CAP-04,
//! VAL-01, VAL-02, D-11).
//!
//! Spawns the deterministic, multi-threaded `nvtx_test_app` as a **subprocess**
//! with `NVTX_INJECTION64_PATH` pointing at the `quent-nvtx` capture cdylib and
//! `QUENT_NVTX_OUTPUT_DIR` at a temp dir, then reads back the ndjson the cdylib
//! wrote and asserts that:
//!
//! * at least one event of EVERY core NVTX kind was captured (CAP-02);
//! * the CORE payload union round-trips verbatim on the payload-carrying mark
//!   (CAP-03 / D-12);
//! * a cross-thread `RangeStart`/`RangeEnd` pair with the same range id both
//!   appear (D-11);
//! * a distinct `NameThread` was captured for the main thread and each worker.
//!
//! Requires no GPU. Needs the `e2e` feature (which builds the test-app + its
//! NVTX client shim). Because the capture cdylib (`libquent_nvtx.so`) must exist
//! next to the test-app binary before the subprocess runs, build it first:
//!
//! ```text
//! cargo build -p quent-nvtx --features e2e
//! cargo test  -p quent-nvtx --features e2e --test capture_e2e
//! ```

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use quent_events::Event;
use quent_io::{FileSystemFormat, ImporterOptions, ImporterProvider, filesystem};
use quent_nvtx_events::{NvtxEvent, NvtxEventKind, NvtxPayload, NvtxPayloadValue};
use uuid::Uuid;

/// The CORE payload union value the test-app attaches to its mark (D-12).
const MARK_PAYLOAD: u64 = 0xCAFE_F00D;

/// Read every `Event<NvtxEventKind>` from `<entity_dir>/*.ndjson`.
fn read_events(entity_dir: &std::path::Path) -> Vec<Event<NvtxEventKind>> {
    let importer = <ImporterOptions as ImporterProvider<NvtxEventKind>>::create_importer(
        &ImporterOptions::FileSystem(filesystem::importer::Options {
            format: FileSystemFormat::Ndjson,
            path: entity_dir.to_path_buf(),
        }),
    )
    .expect("build ndjson importer");
    importer.collect()
}

/// The variant name of an [`NvtxEvent`], for coverage assertions.
fn kind_name(event: &NvtxEvent) -> &'static str {
    match event {
        NvtxEvent::RangePush { .. } => "RangePush",
        NvtxEvent::RangePop { .. } => "RangePop",
        NvtxEvent::RangeStart { .. } => "RangeStart",
        NvtxEvent::RangeEnd { .. } => "RangeEnd",
        NvtxEvent::Mark { .. } => "Mark",
        NvtxEvent::DomainCreate { .. } => "DomainCreate",
        NvtxEvent::DomainDestroy { .. } => "DomainDestroy",
        NvtxEvent::RegisterString { .. } => "RegisterString",
        NvtxEvent::NameCategory { .. } => "NameCategory",
        NvtxEvent::NameThread { .. } => "NameThread",
        NvtxEvent::ResourceCreate { .. } => "ResourceCreate",
        NvtxEvent::ResourceDestroy { .. } => "ResourceDestroy",
    }
}

#[test]
fn subprocess_captures_every_core_kind_payload_and_cross_thread_range() {
    // The test-app binary and the capture cdylib both live in the deps' target
    // dir; resolve the cdylib next to the binary.
    let test_app = PathBuf::from(env!("CARGO_BIN_EXE_nvtx_test_app"));
    let target_dir = test_app.parent().expect("binary has a parent dir");
    let cdylib = target_dir.join("libquent_nvtx.so");
    assert!(
        cdylib.is_file(),
        "capture cdylib not found at {} — build it first: \
         `cargo build -p quent-nvtx --features e2e`",
        cdylib.display()
    );

    let out = tempfile::tempdir().unwrap();
    let session = Uuid::now_v7();

    // Set NVTX_INJECTION64_PATH BEFORE spawn so it is present at the first NVTX
    // call (Pitfall 1). No app code change — capture is purely env-driven.
    let status = Command::new(&test_app)
        .env("NVTX_INJECTION64_PATH", &cdylib)
        .env("QUENT_NVTX_OUTPUT_DIR", out.path())
        .env("QUENT_NVTX_SESSION", session.to_string())
        .status()
        .expect("spawn nvtx_test_app");
    assert!(status.success(), "test-app exited with {status}");

    // The cdylib flushes on process exit (.fini_array), but poll briefly to be
    // robust to filesystem/scheduling latency.
    let entity_dir = out.path().join(session.to_string()).join("NvtxEvent");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut events = Vec::new();
    loop {
        if entity_dir.is_dir() {
            events = read_events(&entity_dir);
        }
        if !events.is_empty() || Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    assert!(
        !events.is_empty(),
        "no NvtxEvents captured under {}",
        entity_dir.display()
    );

    // CAP-04: every captured event carries a populated capture timestamp.
    for event in &events {
        assert!(event.timestamp > 0, "event missing capture timestamp");
    }

    // CAP-02 / success-criterion 1: every core NVTX kind is present.
    let present: HashSet<&str> = events.iter().map(|e| kind_name(&e.data.0)).collect();
    for expected in [
        "RangePush",
        "RangePop",
        "RangeStart",
        "RangeEnd",
        "Mark",
        "DomainCreate",
        "DomainDestroy",
        "RegisterString",
        "NameCategory",
        "NameThread",
        "ResourceCreate",
        "ResourceDestroy",
    ] {
        assert!(
            present.contains(expected),
            "missing captured kind {expected}; present: {present:?}"
        );
    }

    // CAP-03 / D-12: the CORE payload union round-trips verbatim on a mark.
    let payload_ok = events.iter().any(|e| match &e.data.0 {
        NvtxEvent::Mark { attributes, .. } => {
            attributes.payload
                == Some(NvtxPayload {
                    payload_type: 1, // NVTX_PAYLOAD_TYPE_UNSIGNED_INT64
                    value: NvtxPayloadValue::UnsignedInt64(MARK_PAYLOAD),
                })
        }
        _ => false,
    });
    assert!(
        payload_ok,
        "expected a Mark carrying the verbatim CORE payload union {MARK_PAYLOAD:#x}"
    );

    // D-11: a cross-thread RangeStart/RangeEnd pair shares one range id.
    let start_ids: HashSet<u64> = events
        .iter()
        .filter_map(|e| match &e.data.0 {
            NvtxEvent::RangeStart { range_id, .. } => Some(*range_id),
            _ => None,
        })
        .collect();
    let end_ids: HashSet<u64> = events
        .iter()
        .filter_map(|e| match &e.data.0 {
            NvtxEvent::RangeEnd { range_id, .. } => Some(*range_id),
            _ => None,
        })
        .collect();
    assert!(
        start_ids.intersection(&end_ids).next().is_some(),
        "no RangeStart id matched a RangeEnd id (start={start_ids:?}, end={end_ids:?})"
    );

    // D-11: a distinct NameThread for the main thread and each worker.
    let thread_ids: HashSet<u32> = events
        .iter()
        .filter_map(|e| match &e.data.0 {
            NvtxEvent::NameThread { thread_id, .. } => Some(*thread_id),
            _ => None,
        })
        .collect();
    assert!(
        thread_ids.len() >= 3,
        "expected NameThread for main + 2 workers, got {}: {thread_ids:?}",
        thread_ids.len()
    );

    // The worker thread names round-trip (proves per-thread naming, not just a
    // count of ids).
    let thread_names: HashSet<&str> = events
        .iter()
        .filter_map(|e| match &e.data.0 {
            NvtxEvent::NameThread { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    for expected in [
        "quent-nvtx-e2e/main",
        "quent-nvtx-e2e/worker-a",
        "quent-nvtx-e2e/worker-b",
    ] {
        assert!(
            thread_names.contains(expected),
            "missing NameThread {expected}; present: {thread_names:?}"
        );
    }
}
