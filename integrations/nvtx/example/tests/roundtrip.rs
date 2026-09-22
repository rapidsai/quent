// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! End-to-end proof against a **real** NVTX capture.
//!
//! Exercises generated instrumentation, the common exporter/importer, and
//! borrowed reconstruction together with the native injection layer.

use nvtx_analyzer::{NvtxModelBuilder, SpanKind, StatsKey};
use nvtx_example::store::{NvtxDemo, NvtxDemoEvent};
use quent_instrumentation::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
use quent_store::event::filesystem::Store;
use uuid::Uuid;

/// The default (NULL) NVTX domain, which is where `nvtx_example` annotates.
const DEFAULT_DOMAIN: u64 = 0;

#[test]
fn example_capture_roundtrip() {
    let output = tempfile::tempdir().expect("temporary capture directory");
    let context_id = Uuid::now_v7();
    let exporter =
        FileSystemExporterOptions::new(FileSystemFormat::Ndjson, output.path().to_path_buf());

    // Injection is process-global and one-shot, so this is deliberately a single
    // test doing a single capture — no parallel capture is possible here.
    nvtx_example::run_capture(context_id, ExporterOptions::FileSystem(exporter)).expect("capture");

    let events = Store::<NvtxDemo>::new(output.path())
        .load_context(context_id)
        .expect("import generated model events")
        .into_events();
    assert!(
        events
            .iter()
            .any(|event| matches!(event.data, NvtxDemoEvent::Process(_)))
    );
    let model = NvtxModelBuilder::build_from(events.iter().filter_map(|event| match &event.data {
        NvtxDemoEvent::NvtxEvent(data) => Some((event.timestamp, data)),
        _ => None,
    }));

    // `nvtx::name_thread` — the name must reach the thread view.
    assert!(
        model
            .threads()
            .iter()
            .any(|thread| thread.name == "nvtx-example/main"),
        "named thread missing; threads: {:?}",
        model.threads()
    );

    // `nvtx::mark` — an instant, not a zero-length span.
    assert!(
        model.marks().iter().any(|mark| mark.name == "startup"),
        "mark \"startup\" missing; marks: {:?}",
        model.marks()
    );
    assert!(
        !model.spans().iter().any(|span| span.name == "startup"),
        "the mark must not have been reconstructed as a span"
    );

    // `nvtx::LocalRange` — a per-thread nested range.
    let phase1 = model
        .spans()
        .iter()
        .find(|span| span.name == "phase-1")
        .expect("span \"phase-1\" missing");
    assert!(
        matches!(phase1.kind, SpanKind::PushPop { .. }),
        "a push/pop range carries the OS thread it ran on; got {:?}",
        phase1.kind
    );
    assert!(phase1.end.is_some(), "the pop was observed");
    assert!(phase1.duration().is_some());

    // `nvtx::Range` — a process-wide start/end range.
    let phase2 = model
        .spans()
        .iter()
        .find(|span| span.name == "phase-2")
        .expect("span \"phase-2\" missing");
    assert_eq!(phase2.kind, SpanKind::StartEnd);
    assert!(phase2.end.is_some(), "the end was observed");
    assert!(phase2.duration().is_some());

    // Both ranges reach the statistics, one group each.
    let stats = model.range_statistics();
    for name in ["phase-1", "phase-2"] {
        let group = stats
            .get(&StatsKey {
                name: name.to_owned(),
                domain: DEFAULT_DOMAIN,
                category: None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "no statistics for {name:?}; groups: {:?}",
                    stats.keys().collect::<Vec<_>>()
                )
            });
        assert_eq!(group.count, 1, "one occurrence of {name:?}");
        assert_eq!(group.observed_count, 1, "{name:?} closed for real");
        assert_eq!(
            group.min_duration, group.max_duration,
            "a single occurrence bounds itself"
        );
        assert_eq!(group.avg_duration, group.total_duration);
    }
}
