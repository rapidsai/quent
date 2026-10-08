// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use nvtx_injection::record::{Attributes, Color, Message, Payload, Record, String as RecordString};
use nvtx_sys::ffi::nvtxPayloadType_t;
use quent_nvtx_bridge::convert;
use quent_nvtx_events::{
    NvtxColor, NvtxEvent, NvtxEventAttributes, NvtxMessage, NvtxPayload, NvtxPayloadValue,
};

fn string(raw: RecordString) -> String {
    let NvtxEvent::DomainCreate { name, .. } = convert(Record::DomainCreate {
        domain: 0,
        name: raw,
    }) else {
        panic!("expected DomainCreate");
    };
    name
}

fn current_thread_id() -> u32 {
    // SAFETY: SYS_gettid takes no arguments and returns this thread's OS ID.
    unsafe { libc::syscall(libc::SYS_gettid) as u32 }
}

#[test]
fn push_and_pop_receive_the_converting_threads_id() {
    let NvtxEvent::RangePush {
        domain,
        thread_id,
        attributes,
    } = convert(Record::RangePush {
        domain: 0x1234,
        attributes: Attributes::default(),
    })
    else {
        panic!("expected RangePush");
    };
    assert_eq!(domain, 0x1234);
    assert_eq!(thread_id, current_thread_id());
    assert_eq!(attributes, NvtxEventAttributes::default());

    let NvtxEvent::RangePop { domain, thread_id } = convert(Record::RangePop { domain: 0x1234 })
    else {
        panic!("expected RangePop");
    };
    assert_eq!(domain, 0x1234);
    assert_eq!(thread_id, current_thread_id());
}

#[test]
fn attributes_decode_text_color_and_payload() {
    let NvtxEvent::Mark { domain, attributes } = convert(Record::Mark {
        domain: 17,
        attributes: Attributes {
            category: 7,
            color: Some(Color {
                color_type: 1,
                value: 0xff00_ff00,
            }),
            message: Some(Message::String(RecordString::Bytes(b"mark".to_vec()))),
            payload: Some(Payload {
                payload_type: nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_UNSIGNED_INT64 as i32,
                bits: 0xcafe_f00d,
            }),
        },
    }) else {
        panic!("expected Mark");
    };
    assert_eq!(domain, 17);
    assert_eq!(attributes.category, 7);
    assert_eq!(
        attributes.color,
        Some(NvtxColor {
            color_type: 1,
            value: 0xff00_ff00
        })
    );
    assert_eq!(attributes.message, Some(NvtxMessage::String("mark".into())));
    assert_eq!(
        attributes.payload,
        Some(NvtxPayload {
            payload_type: nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_UNSIGNED_INT64 as i32,
            value: NvtxPayloadValue::UnsignedInt64(0xcafe_f00d),
        })
    );
}

#[test]
fn payload_tags_decode_numeric_values() {
    for (payload_type, bits, expected) in [
        (
            nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_INT64,
            (-7_i64) as u64,
            NvtxPayloadValue::Int64(-7),
        ),
        (
            nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_DOUBLE,
            0.25_f64.to_bits(),
            NvtxPayloadValue::Double(0.25),
        ),
        (
            nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_UNSIGNED_INT32,
            0xabcd,
            NvtxPayloadValue::UnsignedInt32(0xabcd),
        ),
        (
            nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_INT32,
            (-7_i32) as u32 as u64,
            NvtxPayloadValue::Int32(-7),
        ),
        (
            nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_FLOAT,
            0.25_f32.to_bits() as u64,
            NvtxPayloadValue::Float(0.25),
        ),
    ] {
        let NvtxEvent::Mark { attributes, .. } = convert(Record::Mark {
            domain: 0,
            attributes: Attributes {
                payload: Some(Payload {
                    payload_type: payload_type as i32,
                    bits,
                }),
                ..Attributes::default()
            },
        }) else {
            panic!("expected Mark");
        };
        assert_eq!(attributes.payload.unwrap().value, expected);
    }
}

#[test]
fn strings_decode_only_in_the_bridge() {
    assert_eq!(string(RecordString::Bytes(b"hello".to_vec())), "hello");
    assert_eq!(
        string(RecordString::Bytes(b"first\xff".to_vec())),
        "first\u{fffd}"
    );
    assert_eq!(
        string(RecordString::Wide(vec![0x63, 0x61, 0x66, 0xe9, 0x1f600])),
        "café😀"
    );
    assert_eq!(string(RecordString::Wide(vec![0xd800])), "\u{fffd}");
    assert_eq!(string(RecordString::Wide(vec![])), "");
}

#[test]
fn range_ids_and_registered_handles_remain_unchanged() {
    let NvtxEvent::RangeStart {
        domain,
        range_id,
        attributes,
    } = convert(Record::RangeStart {
        domain: 0x11,
        range_id: 0xdead_beef,
        attributes: Attributes {
            message: Some(Message::RegisteredHandle(0xabcd)),
            ..Attributes::default()
        },
    })
    else {
        panic!("expected RangeStart");
    };
    assert_eq!((domain, range_id), (0x11, 0xdead_beef));
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::RegisteredHandle(0xabcd))
    );
    assert_eq!(
        convert(Record::RangeEnd { domain, range_id }),
        NvtxEvent::RangeEnd { domain, range_id }
    );
}

#[test]
fn names_and_resource_fields_convert_without_resolving_handles() {
    assert_eq!(
        convert(Record::DomainCreate {
            domain: 5,
            name: RecordString::Bytes(b"domain".to_vec())
        }),
        NvtxEvent::DomainCreate {
            domain: 5,
            name: "domain".into()
        }
    );
    assert_eq!(
        convert(Record::DomainDestroy { domain: 5 }),
        NvtxEvent::DomainDestroy { domain: 5 }
    );
    assert_eq!(
        convert(Record::RegisterString {
            domain: 5,
            handle: 9,
            string: RecordString::Bytes(b"registered".to_vec())
        }),
        NvtxEvent::RegisterString {
            domain: 5,
            handle: 9,
            string: "registered".into()
        }
    );
    assert_eq!(
        convert(Record::NameCategory {
            domain: 5,
            category: 7,
            name: RecordString::Bytes(b"io".to_vec())
        }),
        NvtxEvent::NameCategory {
            domain: 5,
            category: 7,
            name: "io".into()
        }
    );
    assert_eq!(
        convert(Record::NameThread {
            thread_id: 42,
            name: RecordString::Wide(vec![0x1f600])
        }),
        NvtxEvent::NameThread {
            thread_id: 42,
            name: "😀".into()
        }
    );
    assert_eq!(
        convert(Record::ResourceCreate {
            domain: 5,
            handle: 12,
            identifier_type: 2,
            identifier: 0xabcd,
            message: Some(Message::RegisteredHandle(9))
        }),
        NvtxEvent::ResourceCreate {
            domain: 5,
            handle: 12,
            identifier_type: 2,
            identifier: 0xabcd,
            message: Some(NvtxMessage::RegisteredHandle(9))
        }
    );
    assert_eq!(
        convert(Record::ResourceDestroy { handle: 12 }),
        NvtxEvent::ResourceDestroy { handle: 12 }
    );
}
