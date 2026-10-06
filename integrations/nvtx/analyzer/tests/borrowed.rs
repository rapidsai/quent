// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The analysis contract accepts independently owned application records.
//! These test-only accessors stand in for the bindings that schema generation
//! will supply; no production application adapter or wire format is defined here.

mod fixtures;

use fixtures::{
    at, mark, range_end, range_pop, range_push, range_start, resource_create, resource_destroy,
    span, span_id,
};
use nvtx_analyzer::{
    NvtxAttributesData, NvtxAttributesView, NvtxColor, NvtxEventData, NvtxEventView,
    NvtxMessageData, NvtxMessageView, NvtxModel, NvtxModelBuilder, NvtxPayload, NvtxPayloadValue,
    ReconstructionAnomalies, SpanKind,
};
use nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage};

// Deliberately different field names and storage from the native vocabulary,
// with no Clone implementation. Reconstruction must borrow this storage.
enum MessageRecord {
    Inline(Box<str>),
    Key(u64),
}

impl NvtxMessageData for MessageRecord {
    fn nvtx_message(&self) -> Option<NvtxMessageView<'_>> {
        Some(match self {
            Self::Inline(text) => NvtxMessageView::String(text),
            Self::Key(key) => NvtxMessageView::RegisteredHandle(*key),
        })
    }
}

struct AttributesRecord {
    label: MessageRecord,
    group: u32,
}

impl NvtxAttributesData for AttributesRecord {
    fn nvtx_attributes(&self) -> NvtxAttributesView<'_> {
        NvtxAttributesView {
            message: self.label.nvtx_message(),
            category: self.group,
            ..Default::default()
        }
    }
}

enum ApplicationRecord {
    Binding,
    Begin {
        scope: u64,
        token: u64,
        fields: AttributesRecord,
    },
    Finish {
        scope: u64,
        token: u64,
    },
    Text {
        scope: u64,
        key: u64,
        text: Box<str>,
    },
}

impl NvtxEventData for ApplicationRecord {
    fn nvtx_event(&self) -> Option<NvtxEventView<'_>> {
        Some(match self {
            Self::Binding => return None,
            Self::Begin {
                scope,
                token,
                fields,
            } => NvtxEventView::RangeStart {
                domain: *scope,
                range_id: *token,
                attributes: fields.nvtx_attributes(),
            },
            Self::Finish { scope, token } => NvtxEventView::RangeEnd {
                domain: *scope,
                range_id: *token,
            },
            Self::Text { scope, key, text } => NvtxEventView::RegisterString {
                domain: *scope,
                handle: *key,
                string: text,
            },
        })
    }
}

fn assert_same_model(actual: &NvtxModel, expected: &NvtxModel) {
    assert_eq!(actual.spans(), expected.spans());
    assert_eq!(actual.marks(), expected.marks());
    assert_eq!(actual.domains(), expected.domains());
    assert_eq!(actual.threads(), expected.threads());
    assert_eq!(actual.categories(), expected.categories());
    assert_eq!(actual.anomalies(), expected.anomalies());
    assert_eq!(actual.trace_start(), expected.trace_start());
    assert_eq!(actual.trace_end(), expected.trace_end());
}

#[test]
fn application_records_work_owned_and_borrowed_without_native_conversion() {
    let records = [
        // Late metadata contributes to bounds even though it creates no span.
        (
            90,
            ApplicationRecord::Text {
                scope: 2,
                key: 8,
                text: "registered".into(),
            },
        ),
        (
            40,
            ApplicationRecord::Finish {
                scope: 999,
                token: 3,
            },
        ),
        (
            20,
            ApplicationRecord::Begin {
                scope: 2,
                token: 3,
                fields: AttributesRecord {
                    label: MessageRecord::Key(8),
                    group: 7,
                },
            },
        ),
        (
            10,
            ApplicationRecord::Begin {
                scope: 2,
                token: 4,
                fields: AttributesRecord {
                    label: MessageRecord::Inline("open".into()),
                    group: 0,
                },
            },
        ),
    ];
    let borrowed =
        NvtxModelBuilder::build_from(records.iter().map(|(time, record)| (*time, record)));
    let owned = NvtxModelBuilder::build_from(records);
    let native = NvtxModelBuilder::build([
        at(
            90,
            NvtxEvent::RegisterString {
                domain: 2,
                handle: 8,
                string: "registered".into(),
            },
        ),
        range_end(40, 999, 3),
        at(
            20,
            NvtxEvent::RangeStart {
                domain: 2,
                range_id: 3,
                attributes: NvtxEventAttributes {
                    category: 7,
                    message: Some(NvtxMessage::RegisteredHandle(8)),
                    ..Default::default()
                },
            },
        ),
        range_start(10, 2, 4, "open"),
    ]);

    assert_same_model(&borrowed, &native);
    assert_same_model(&owned, &native);
    assert_eq!(span(&borrowed, "registered").end, Some(40));
    assert_eq!(span(&borrowed, "registered").category, Some(7));
    assert_eq!(span(&borrowed, "open").end, None);
    assert_eq!(borrowed.trace_start(), 10);
    assert_eq!(borrowed.trace_end(), 90);
    assert_eq!(borrowed.spans()[0].domain, 2);
    assert_eq!(
        borrowed
            .domains()
            .iter()
            .map(|d| d.domain)
            .collect::<Vec<_>>(),
        [2, 999]
    );
}

#[test]
fn borrowed_replay_preserves_all_kinds_anomalies_and_span_ids() {
    let mut events = vec![
        range_end(1, 1, 9),
        range_pop(2, 1, 7),
        resource_destroy(3, 8),
        range_push(10, 1, 7, "outer"),
        range_push(11, 1, 7, "inner"),
        range_push(12, 2, 7, "other domain"),
        range_push(13, 1, 99, "other thread"),
        range_pop(14, 1, 7),
        range_start(15, 1, 9, "displaced range"),
        range_start(16, 2, 9, "replacement range"),
        resource_create(17, 1, 8, -4, u64::MAX, "displaced resource"),
        resource_create(18, 2, 8, 9, 44, "replacement resource"),
        resource_destroy(19, 8),
        range_end(20, 999, 9),
        mark(21, 1, "checkpoint"),
        at(
            22,
            NvtxEvent::Mark {
                domain: 2,
                attributes: NvtxEventAttributes {
                    message: Some(NvtxMessage::RegisteredHandle(5)),
                    ..Default::default()
                },
            },
        ),
        // Metadata arrives after its uses, including the domain's creation.
        at(
            25,
            NvtxEvent::DomainCreate {
                domain: 1,
                name: "named domain".into(),
            },
        ),
        at(
            26,
            NvtxEvent::RegisterString {
                domain: 2,
                handle: 5,
                string: "late name".into(),
            },
        ),
        at(
            27,
            NvtxEvent::NameCategory {
                domain: 1,
                category: 6,
                name: "category".into(),
            },
        ),
        at(
            28,
            NvtxEvent::NameThread {
                thread_id: 7,
                name: "thread".into(),
            },
        ),
        at(30, NvtxEvent::DomainDestroy { domain: 1 }),
    ];
    events.reverse();
    let borrowed =
        NvtxModelBuilder::build_from(events.iter().map(|event| (event.timestamp, &event.data)));
    let native = NvtxModelBuilder::build(events);

    assert_same_model(&borrowed, &native);
    assert_eq!(
        borrowed.anomalies(),
        ReconstructionAnomalies {
            orphan_range_ends: 1,
            orphan_range_pops: 1,
            orphan_resource_destroys: 1,
            reused_range_ids: 1,
            reused_resource_handles: 1,
        }
    );
    let outer_id = span_id(&borrowed, "outer");
    let inner_id = span_id(&borrowed, "inner");
    assert_eq!(outer_id.0, 0);
    assert_eq!(inner_id.0, 1);
    assert_eq!(
        span(&borrowed, "inner").kind,
        SpanKind::PushPop {
            thread_id: 7,
            parent: Some(outer_id)
        }
    );
    assert_eq!(span(&borrowed, "inner").end, Some(14));
    for name in [
        "outer",
        "other domain",
        "other thread",
        "displaced range",
        "displaced resource",
    ] {
        assert_eq!(span(&borrowed, name).end, None, "{name}");
    }
    assert_eq!(span(&borrowed, "replacement range").end, Some(20));
    assert_eq!(span(&borrowed, "replacement resource").end, Some(19));
    assert_eq!(borrowed.marks()[1].name, "late name");
    assert_eq!(borrowed.thread_name(7), "thread");
    assert_eq!(borrowed.category_name(1, 6).as_deref(), Some("category"));
    assert_eq!(borrowed.domains()[0].first_seen, 1);
    assert_eq!(borrowed.domains()[0].created, Some(25));
    assert_eq!(borrowed.domains()[0].destroyed, Some(30));
    assert_eq!(borrowed.trace_start(), 1);
    assert_eq!(borrowed.trace_end(), 30);
}

#[test]
fn sorting_retains_equal_timestamp_pairing_and_last_registration() {
    let events = [
        at(
            50,
            NvtxEvent::RegisterString {
                domain: 1,
                handle: 8,
                string: "first".into(),
            },
        ),
        range_start(10, 1, 2, "zero duration"),
        range_end(10, 999, 2),
        at(
            50,
            NvtxEvent::RegisterString {
                domain: 1,
                handle: 8,
                string: "last".into(),
            },
        ),
        at(
            20,
            NvtxEvent::Mark {
                domain: 1,
                attributes: NvtxEventAttributes {
                    message: Some(NvtxMessage::RegisteredHandle(8)),
                    ..Default::default()
                },
            },
        ),
        // Earlier in time, but later in arrival order: this must not win.
        at(
            30,
            NvtxEvent::RegisterString {
                domain: 1,
                handle: 8,
                string: "older".into(),
            },
        ),
        range_end(10, 1, 3),
        range_start(10, 1, 3, "open after orphan"),
    ];
    let model =
        NvtxModelBuilder::build_from(events.iter().map(|event| (event.timestamp, &event.data)));
    assert_eq!(span(&model, "zero duration").duration(), Some(0));
    assert_eq!(span(&model, "open after orphan").end, None);
    assert_eq!(model.anomalies().orphan_range_ends, 1);
    assert_eq!(model.marks()[0].name, "last");
}

fn payload_bits(payload: NvtxPayload) -> (i32, u8, u64) {
    use NvtxPayloadValue::*;
    let (kind, bits) = match payload.value {
        UnsignedInt64(v) => (0, v),
        Int64(v) => (1, v as u64),
        Double(v) => (2, v.to_bits()),
        UnsignedInt32(v) => (3, u64::from(v)),
        Int32(v) => (4, u64::from(v as u32)),
        Float(v) => (5, u64::from(v.to_bits())),
        Pointer(v) => (6, v),
    };
    (payload.payload_type, kind, bits)
}

#[test]
fn access_and_reconstruction_preserve_raw_tags_and_scalar_bits() {
    use NvtxPayloadValue::*;
    let values = [
        UnsignedInt64(u64::MAX),
        Int64(i64::MIN),
        Double(-0.0),
        Double(f64::from_bits(0x7ff8_0000_0000_0042)),
        UnsignedInt32(u32::MAX),
        Int32(i32::MIN),
        Float(-0.0),
        Float(f32::from_bits(0x7fc0_0042)),
        Pointer(u64::MAX),
    ];
    let color = NvtxColor {
        color_type: -17,
        value: 0xfedc_ba98,
    };
    for value in values {
        let payload = NvtxPayload {
            payload_type: -99,
            value,
        };
        let attrs = NvtxEventAttributes {
            category: u32::MAX,
            color: Some(color),
            message: Some(NvtxMessage::String("raw".into())),
            payload: Some(payload),
        };
        let view = attrs.nvtx_attributes();
        assert_eq!(view.color, Some(color));
        assert_eq!(payload_bits(view.payload.unwrap()), payload_bits(payload));
        let events = [
            (
                1,
                NvtxEvent::RangeStart {
                    domain: 1,
                    range_id: 7,
                    attributes: attrs.clone(),
                },
            ),
            (
                2,
                NvtxEvent::RangePush {
                    domain: 1,
                    thread_id: u32::MAX,
                    attributes: attrs.clone(),
                },
            ),
            (
                3,
                NvtxEvent::Mark {
                    domain: 1,
                    attributes: attrs,
                },
            ),
        ];
        let borrowed =
            NvtxModelBuilder::build_from(events.iter().map(|(time, data)| (*time, data)));
        let owned = NvtxModelBuilder::build_from(events);
        for model in [borrowed, owned] {
            assert_eq!(model.spans().len(), 2);
            assert_eq!(model.marks().len(), 1);
            for span in model.spans() {
                assert_eq!(span.color, Some(color));
                assert_eq!(span.category, Some(u32::MAX));
                assert_eq!(payload_bits(span.payload.unwrap()), payload_bits(payload));
            }
            assert_eq!(model.marks()[0].color, Some(color));
            assert_eq!(
                payload_bits(model.marks()[0].payload.unwrap()),
                payload_bits(payload)
            );
            assert_eq!(model.threads()[0].thread_id, u32::MAX);
        }
    }
}

#[test]
fn views_borrow_strings_and_expose_raw_resource_identifiers() {
    let attrs = NvtxEventAttributes {
        message: Some(NvtxMessage::String("borrow me".into())),
        ..Default::default()
    };
    let Some(NvtxMessage::String(original)) = &attrs.message else {
        panic!()
    };
    let Some(NvtxMessageView::String(borrowed)) = attrs.nvtx_attributes().message else {
        panic!()
    };
    assert!(std::ptr::eq(original.as_str(), borrowed));

    let event = NvtxEvent::ResourceCreate {
        domain: u64::MAX,
        handle: 7,
        identifier_type: i32::MIN,
        identifier: u64::MAX - 1,
        message: Some(NvtxMessage::RegisteredHandle(8)),
    };
    assert_eq!(
        event.nvtx_event(),
        Some(NvtxEventView::ResourceCreate {
            domain: u64::MAX,
            handle: 7,
            identifier_type: i32::MIN,
            identifier: u64::MAX - 1,
            message: Some(NvtxMessageView::RegisteredHandle(8)),
        })
    );
}

#[test]
fn skipped_stream_metadata_affects_bounds_without_reconstruction() {
    let model = NvtxModelBuilder::build_from([
        (30, ApplicationRecord::Binding),
        (20, ApplicationRecord::Finish { scope: 2, token: 3 }),
        (
            10,
            ApplicationRecord::Begin {
                scope: 2,
                token: 3,
                fields: AttributesRecord {
                    label: MessageRecord::Inline("work".into()),
                    group: 0,
                },
            },
        ),
        (5, ApplicationRecord::Binding),
    ]);

    assert_eq!(model.trace_start(), 5);
    assert_eq!(model.trace_end(), 30);
    assert_eq!(model.spans().len(), 1);
    assert_eq!(span(&model, "work").start, 10);
    assert_eq!(span(&model, "work").end, Some(20));
    assert!(model.marks().is_empty());
    assert!(model.anomalies().is_faithful());
}

#[test]
fn empty_and_metadata_only_inputs_have_correct_bounds() {
    let empty = NvtxModelBuilder::build_from(std::iter::empty::<(u64, &ApplicationRecord)>());
    assert_same_model(&empty, &NvtxModel::default());
    let model = NvtxModelBuilder::build_from([
        (
            70,
            NvtxEvent::NameThread {
                thread_id: 7,
                name: "last".into(),
            },
        ),
        (
            40,
            NvtxEvent::NameThread {
                thread_id: 7,
                name: "first".into(),
            },
        ),
    ]);
    assert_eq!(model.trace_start(), 40);
    assert_eq!(model.trace_end(), 70);
    assert_eq!(model.thread_name(7), "last");
    assert!(model.spans().is_empty());
    assert!(model.marks().is_empty());
    assert!(model.anomalies().is_faithful());
}
