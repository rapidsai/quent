// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Resource lifespan reconstruction, and the keying rule that makes it work.
//!
//! Matching is on the handle alone, since the destroy carries no domain. Keying
//! on `(domain, handle)` would not fail loudly — every resource would silently
//! become a leak with no end — so `resource_lifespan` pins the real end
//! timestamp to catch exactly that.

mod fixtures;

use fixtures::{GENERIC_POINTER, range_push, resource, resource_create, resource_destroy};
use nvtx_analyzer::{NvtxModelBuilder, NvtxSpan, SpanKind};

/// A CUDA-extension resource class (`nvtxResourceCUDAType_t` lives at class 4),
/// which the analyzer deliberately does not label.
const CUDA_EXTENSION_TYPE: i32 = 0x0004_0001;

#[test]
fn resource_lifespan() {
    // The destroy carries no domain at all — only the handle. The reconstructed
    // span must still land in domain 7, recovered from the create.
    let events = vec![
        resource_create(100, 7, 0xABCD, GENERIC_POINTER, 0xDEAD_BEEF, "buf"),
        resource_destroy(400, 0xABCD),
    ];

    let model = NvtxModelBuilder::build(events);

    let resources: Vec<&NvtxSpan> = model.resources().collect();
    assert_eq!(
        resources.len(),
        1,
        "one span per matched create/destroy pair"
    );

    let buf = resource(&model, "buf");
    assert_eq!(
        buf.kind,
        SpanKind::Resource {
            identifier_type: GENERIC_POINTER
        }
    );
    assert_eq!(buf.domain, 7, "domain recovered from the create");
    assert_eq!(buf.start, 100);
    assert_eq!(
        buf.end,
        Some(400),
        "matched by handle alone; a (domain, handle) key would leave this with no end"
    );
    assert_eq!(buf.duration(), Some(300));
    assert_eq!(
        buf.kind.parent(),
        None,
        "resource lifespans do not nest, whatever the kind"
    );
}

#[test]
fn resource_identifier_type_labels() {
    // One core/generic type and one the analyzer does not recognize.
    let events = vec![
        resource_create(100, 1, 0x01, GENERIC_POINTER, 0, "known"),
        resource_destroy(200, 0x01),
        resource_create(110, 1, 0x02, CUDA_EXTENSION_TYPE, 0, "extension"),
        resource_destroy(210, 0x02),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(
        resource(&model, "known").kind.identifier_type_label(),
        Some("generic pointer".to_owned()),
        "a core nvtxResourceGenericType_t value gets a static label"
    );
    assert_eq!(
        resource(&model, "extension").kind.identifier_type_label(),
        Some(format!("<identifier_type {CUDA_EXTENSION_TYPE}>")),
        "an unrecognized type passes through raw rather than being guessed at"
    );
}

#[test]
fn resource_unclosed_has_no_end() {
    // No destroy for the resource; a later push carries the trace to 500. The
    // handle may well still be live, so nothing is written into its end.
    let events = vec![
        resource_create(100, 1, 0xFEED, GENERIC_POINTER, 0, "leaked"),
        range_push(500, 1, 1, "later"),
    ];

    let model = NvtxModelBuilder::build(events);

    let leaked = resource(&model, "leaked");
    assert_eq!(leaked.end, None, "the destroy was never captured");
    assert_eq!(leaked.duration(), None);
    assert_eq!(model.trace_end(), 500, "the bound is on the model instead");
}

#[test]
fn recreated_handle_keeps_the_displaced_lifespan() {
    // The same handle is created twice with no destroy in between — a reuse
    // before the first lifespan ended. The first create was observed, so its
    // lifespan must reconstruct rather than vanish.
    let events = vec![
        resource_create(100, 1, 0xAB, GENERIC_POINTER, 0, "first"),
        resource_create(300, 1, 0xAB, GENERIC_POINTER, 0, "second"),
        resource_destroy(500, 0xAB),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.resources().count(), 2, "both creates produce a span");

    let first = resource(&model, "first");
    assert_eq!(first.start, 100);
    assert_eq!(
        first.end, None,
        "the displaced lifespan is kept, but nothing ever destroyed it"
    );

    // The recreate leaves no mark on the span itself, so the model carries it.
    assert_eq!(model.anomalies().reused_resource_handles, 1);
    assert!(!model.anomalies().is_faithful());

    let second = resource(&model, "second");
    assert_eq!(second.start, 300);
    assert_eq!(second.end, Some(500), "the destroy was observed");
}

#[test]
fn resource_orphan_destroy_skipped() {
    // A destroy with no create ahead of it — the normal case for a resource
    // created before capture attached. It must be skipped, not fatal.
    let events = vec![
        resource_destroy(100, 0x99),
        resource_create(200, 1, 0x11, GENERIC_POINTER, 0, "real"),
        resource_destroy(300, 0x11),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(
        model.resources().count(),
        1,
        "the orphan destroy contributes no span; the matched pair still does"
    );
    assert_eq!(resource(&model, "real").end, Some(300));
    assert_eq!(model.anomalies().orphan_resource_destroys, 1);
}
