// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use nvtx_injection::record::{Attributes, Color, Message, Payload, Record, String as RecordString};
use quent_nvtx_bridge::convert;
use quent_nvtx_sink::quent::nvtx::*;

fn assert_event(actual: SinkEvent, expected: SinkEvent) {
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}

fn string(raw: RecordString) -> String {
    let SinkEvent::DomainCreate {
        data: DomainCreate { name, .. },
    } = convert(Record::DomainCreate {
        domain: 0,
        name: raw,
    })
    else {
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
    let SinkEvent::RangePush {
        data:
            RangePush {
                domain,
                thread_id,
                attributes,
            },
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

    let SinkEvent::RangePop {
        data: RangePop { domain, thread_id },
    } = convert(Record::RangePop { domain: 0x1234 })
    else {
        panic!("expected RangePop");
    };
    assert_eq!(domain, 0x1234);
    assert_eq!(thread_id, current_thread_id());
}

#[test]
fn attributes_preserve_text_color_and_raw_payload() {
    let SinkEvent::Mark {
        data: Mark { domain, attributes },
    } = convert(Record::Mark {
        domain: 17,
        attributes: Attributes {
            category: 7,
            color: Some(Color {
                color_type: 1,
                value: 0xff00_ff00,
            }),
            message: Some(Message::String(RecordString::Bytes(b"mark".to_vec()))),
            payload: Some(Payload {
                payload_type: -7,
                bits: 0x7ff8_0000_0000_0042,
            }),
        },
    })
    else {
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
    assert_eq!(
        attributes.message,
        Some(NvtxMessage {
            string: Some("mark".into()),
            registered_handle: None
        })
    );
    assert_eq!(
        attributes.payload,
        Some(NvtxPayload {
            payload_type: -7,
            value: 0x7ff8_0000_0000_0042,
        })
    );
}

#[test]
fn strings_decode_utf8_and_wide_text_lossily() {
    assert_eq!(
        string(RecordString::Bytes(b"first\xff".to_vec())),
        "first\u{fffd}"
    );
    assert_eq!(
        string(RecordString::Wide(vec![0x63, 0x61, 0x66, 0xe9, 0x1f600])),
        "café😀"
    );
    assert_eq!(string(RecordString::Wide(vec![0xd800])), "\u{fffd}");
}

#[test]
fn range_ids_and_registered_handles_remain_unchanged() {
    let SinkEvent::RangeStart {
        data:
            RangeStart {
                domain,
                range_id,
                attributes,
            },
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
        Some(NvtxMessage {
            string: None,
            registered_handle: Some(0xabcd)
        })
    );
    assert_event(
        convert(Record::RangeEnd { domain, range_id }),
        SinkEvent::RangeEnd {
            data: RangeEnd { domain, range_id },
        },
    );
}

#[test]
fn names_and_resource_fields_convert_without_resolving_handles() {
    assert_event(
        convert(Record::DomainCreate {
            domain: 5,
            name: RecordString::Bytes(b"domain".to_vec()),
        }),
        SinkEvent::DomainCreate {
            data: DomainCreate {
                domain: 5,
                name: "domain".into(),
            },
        },
    );
    assert_event(
        convert(Record::DomainDestroy { domain: 5 }),
        SinkEvent::DomainDestroy {
            data: DomainDestroy { domain: 5 },
        },
    );
    assert_event(
        convert(Record::RegisterString {
            domain: 5,
            handle: 9,
            string: RecordString::Bytes(b"registered".to_vec()),
        }),
        SinkEvent::RegisterString {
            data: RegisterString {
                domain: 5,
                handle: 9,
                string: "registered".into(),
            },
        },
    );
    assert_event(
        convert(Record::NameCategory {
            domain: 5,
            category: 7,
            name: RecordString::Bytes(b"io".to_vec()),
        }),
        SinkEvent::NameCategory {
            data: NameCategory {
                domain: 5,
                category: 7,
                name: "io".into(),
            },
        },
    );
    assert_event(
        convert(Record::NameThread {
            thread_id: 42,
            name: RecordString::Wide(vec![0x1f600]),
        }),
        SinkEvent::NameThread {
            data: NameThread {
                thread_id: 42,
                name: "😀".into(),
            },
        },
    );
    assert_event(
        convert(Record::ResourceCreate {
            domain: 5,
            handle: 12,
            identifier_type: 2,
            identifier: 0xabcd,
            message: Some(Message::RegisteredHandle(9)),
        }),
        SinkEvent::ResourceCreate {
            data: ResourceCreate {
                domain: 5,
                handle: 12,
                identifier_type: 2,
                identifier: 0xabcd,
                message: Some(NvtxMessage {
                    string: None,
                    registered_handle: Some(9),
                }),
            },
        },
    );
    assert_event(
        convert(Record::ResourceDestroy { handle: 12 }),
        SinkEvent::ResourceDestroy {
            data: ResourceDestroy { handle: 12 },
        },
    );
}
