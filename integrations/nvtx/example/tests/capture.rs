// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! In-process capture proof.
//!
//! Reuses the crate's [`run_capture`](nvtx_example::run_capture) with a
//! collecting callback exporter and asserts every core NVTX kind the example
//! emits is captured. No GPU, no subprocess, no files.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::NvtxEvent;
use quent_instrumentation::{Event, EventCallback};
use uuid::Uuid;

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
fn captures_core_nvtx_kinds() {
    // Collect every captured event in memory via a callback exporter.
    let collected: Arc<Mutex<Vec<NvtxEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let collected = Arc::clone(&collected);
        EventCallback::new(move |recorded| {
            if let Some(event) = recorded.event.downcast_ref::<Event<NvtxEventEntity>>() {
                collected.lock().unwrap().push(event.data.0.clone());
            }
        })
    };

    nvtx_example::run_capture(Uuid::now_v7(), sink).expect("capture");

    let events = collected.lock().unwrap();
    assert!(!events.is_empty(), "no NVTX events captured");

    let present: HashSet<&str> = events.iter().map(kind_name).collect();
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
