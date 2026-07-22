// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! End-to-end proof of in-process capture.
//!
//! Runs the `nvtx_capture` example binary (via `CARGO_BIN_EXE_nvtx_capture`)
//! against a temp dir, reads back the ndjson, and asserts every core NVTX kind
//! the example emits round-trips. No GPU.

use std::collections::HashSet;
use std::process::Command;

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::NvtxEvent;
use quent_events::Event;
use quent_io::{FileSystemFormat, ImporterOptions, ImporterProvider, filesystem};
use uuid::Uuid;

/// Read every `Event<NvtxEventEntity>` from `<entity_dir>/*.ndjson`.
fn read_events(entity_dir: &std::path::Path) -> Vec<Event<NvtxEventEntity>> {
    let importer = <ImporterOptions as ImporterProvider<NvtxEventEntity>>::create_importer(
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
fn example_binary_captures_core_nvtx_kinds() {
    let out = tempfile::tempdir().unwrap();
    let session = Uuid::now_v7();

    // Run the example binary with the temp dir and an explicit session id, so we
    // read back the exact output path. It self-injects via the static-injection
    // strong symbol — no NVTX_INJECTION64_PATH needed.
    let status = Command::new(env!("CARGO_BIN_EXE_nvtx_capture"))
        .arg(out.path())
        .arg(session.to_string())
        .status()
        .expect("run the nvtx_capture example");
    assert!(status.success(), "example exited with {status}");

    let entity_dir = out.path().join(session.to_string()).join("NvtxEvent");

    let events = read_events(&entity_dir);
    assert!(
        !events.is_empty(),
        "no NvtxEvents captured under {}",
        entity_dir.display()
    );

    // Every captured event carries a populated capture timestamp.
    for event in &events {
        assert!(event.timestamp > 0, "event missing capture timestamp");
    }

    // Every kind the example emits is present.
    let present: HashSet<&str> = events.iter().map(|e| kind_name(&e.data.0)).collect();
    for expected in [
        "NameThread",
        "Mark",
        "RangePush",
        "RangePop",
        "RangeStart",
        "RangeEnd",
    ] {
        assert!(
            present.contains(expected),
            "missing captured kind {expected}; present: {present:?}"
        );
    }
}
