// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::{convert, string};
use nvtx_injection::record;

use std::ffi::CString;
use std::mem::{offset_of, size_of};

use nvtx_events::{
    NvtxColor, NvtxEvent, NvtxEventAttributes, NvtxMessage, NvtxPayload, NvtxPayloadValue,
};

use nvtx_sys::ffi::{
    nvtxColorType_t, nvtxEventAttributes_v2, nvtxEventAttributes_v2_payload_t, nvtxMessageType_t,
    nvtxMessageValue_t, nvtxPayloadType_t, nvtxResourceAttributes_v0,
    nvtxResourceAttributes_v0_identifier_t, nvtxStringHandle_t, wchar_t,
};

/// A zeroed v2 attribute struct with `version`/`size` set for the full layout.
fn full_attr() -> nvtxEventAttributes_v2 {
    nvtxEventAttributes_v2 {
        version: 2,
        size: size_of::<nvtxEventAttributes_v2>() as u16,
        category: 0,
        colorType: nvtxColorType_t::NVTX_COLOR_UNKNOWN as i32,
        color: 0,
        payloadType: nvtxPayloadType_t::NVTX_PAYLOAD_UNKNOWN as i32,
        reserved0: 0,
        payload: nvtxEventAttributes_v2_payload_t { ullValue: 0 },
        messageType: nvtxMessageType_t::NVTX_MESSAGE_UNKNOWN as i32,
        message: nvtxMessageValue_t {
            ascii: std::ptr::null(),
        },
    }
}

#[test]
fn range_push_converts_message_category_and_core_payload_verbatim() {
    let message = CString::new("range").expect("cstring");
    let attr = nvtxEventAttributes_v2 {
        category: 7,
        colorType: nvtxColorType_t::NVTX_COLOR_ARGB as i32,
        color: 0xFF00_FF00,
        payloadType: nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_UNSIGNED_INT64 as i32,
        payload: nvtxEventAttributes_v2_payload_t {
            ullValue: 0xCAFE_F00D,
        },
        messageType: nvtxMessageType_t::NVTX_MESSAGE_TYPE_ASCII as i32,
        message: nvtxMessageValue_t {
            ascii: message.as_ptr(),
        },
        ..full_attr()
    };

    // SAFETY: `attr` is a valid, fully-sized attribute struct.
    let event = unsafe { convert(record::range_push(0x1234, &attr, 4242)) };

    let NvtxEvent::RangePush {
        domain,
        thread_id,
        attributes,
    } = event
    else {
        panic!("expected RangePush");
    };
    // Raw handle kept verbatim.
    assert_eq!(domain, 0x1234);
    // The OS thread id is passed through verbatim from the callback.
    assert_eq!(thread_id, 4242);
    assert_eq!(attributes.category, 7);
    // The message is copied into an OWNED String (no borrowed pointer).
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("range".to_owned()))
    );
    // The CORE payload union survives verbatim (raw tag + value).
    assert_eq!(
        attributes.payload,
        Some(NvtxPayload {
            payload_type: 1,
            value: NvtxPayloadValue::UnsignedInt64(0xCAFE_F00D),
        })
    );
    assert_eq!(
        attributes.color,
        Some(NvtxColor {
            color_type: 1,
            value: 0xFF00_FF00,
        })
    );
}

#[test]
fn range_pop_carries_domain_and_thread_id_verbatim() {
    let NvtxEvent::RangePop { domain, thread_id } = convert(record::range_pop(0x55, 4242)) else {
        panic!("expected RangePop");
    };
    // Both the raw domain and the passed-in OS thread id are kept verbatim,
    // so a pop pairs with its push on the same thread in the analyzer.
    assert_eq!(domain, 0x55);
    assert_eq!(thread_id, 4242);
}

#[test]
fn smaller_size_reads_only_declared_members_without_over_read() {
    // Claim a `size` that only covers through the `color` member. Even though
    // the backing struct has payload/message set, they sit past `size` and
    // must not be read.
    let message = CString::new("must-not-be-read").expect("cstring");
    let truncated_size = (offset_of!(nvtxEventAttributes_v2, color) + size_of::<u32>()) as u16;
    let attr = nvtxEventAttributes_v2 {
        size: truncated_size,
        category: 42,
        colorType: nvtxColorType_t::NVTX_COLOR_ARGB as i32,
        color: 0x00AA_BBCC,
        payloadType: nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_INT64 as i32,
        payload: nvtxEventAttributes_v2_payload_t { llValue: -1 },
        messageType: nvtxMessageType_t::NVTX_MESSAGE_TYPE_ASCII as i32,
        message: nvtxMessageValue_t {
            ascii: message.as_ptr(),
        },
        ..full_attr()
    };

    // SAFETY: reads are bounded by `attr.size`; the backing allocation is a
    // full struct, so even an accidental over-read would be in-bounds — the
    // assertions prove we honor `size` regardless.
    let event = unsafe { convert(record::range_push(1, &attr, 0)) };

    let NvtxEvent::RangePush { attributes, .. } = event else {
        panic!("expected RangePush");
    };
    assert_eq!(attributes.category, 42);
    assert_eq!(
        attributes.color,
        Some(NvtxColor {
            color_type: 1,
            value: 0x00AA_BBCC,
        })
    );
    // payload and message live past the declared `size` → not read.
    assert_eq!(attributes.payload, None);
    assert_eq!(attributes.message, None);
}

#[test]
fn registered_message_keeps_handle_without_dereferencing_a_string() {
    let handle: u64 = 0xABCD;
    let attr = nvtxEventAttributes_v2 {
        messageType: nvtxMessageType_t::NVTX_MESSAGE_TYPE_REGISTERED as i32,
        message: nvtxMessageValue_t {
            // A raw handle, NOT a string pointer — must never be dereferenced.
            registered: handle as nvtxStringHandle_t,
        },
        ..full_attr()
    };

    // SAFETY: `attr` is a valid, fully-sized attribute struct.
    let event = unsafe { convert(record::range_push(0, &attr, 0)) };

    let NvtxEvent::RangePush { attributes, .. } = event else {
        panic!("expected RangePush");
    };
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::RegisteredHandle(handle))
    );
}

#[test]
fn narrow_payloads_are_read_at_member_width_verbatim() {
    // 32-bit payload members occupy only the low four bytes of the 8-byte
    // union; the reader must take exactly the tagged member's width (not the
    // full u64, whose upper bytes may be uninitialized for a 32-bit write).
    for (tag, payload, expected) in [
        (
            nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_FLOAT,
            nvtxEventAttributes_v2_payload_t { fValue: 0.25 },
            NvtxPayloadValue::Float(0.25),
        ),
        (
            nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_INT32,
            nvtxEventAttributes_v2_payload_t { iValue: -7 },
            NvtxPayloadValue::Int32(-7),
        ),
        (
            nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_UNSIGNED_INT32,
            nvtxEventAttributes_v2_payload_t { uiValue: 0xABCD },
            NvtxPayloadValue::UnsignedInt32(0xABCD),
        ),
    ] {
        let attr = nvtxEventAttributes_v2 {
            payloadType: tag as i32,
            payload,
            ..full_attr()
        };
        // SAFETY: `attr` is a valid, fully-sized attribute struct.
        let NvtxEvent::RangePush { attributes, .. } =
            (unsafe { convert(record::range_push(0, &attr, 0)) })
        else {
            panic!("expected RangePush");
        };
        assert_eq!(attributes.payload.expect("payload present").value, expected);
    }
}

// ---- The remaining CORE/CORE2 kinds ---------------------------------------

/// A message-only ASCII attribute struct (for mark/range kinds).
fn message_attr(label: &std::ffi::CStr) -> nvtxEventAttributes_v2 {
    nvtxEventAttributes_v2 {
        messageType: nvtxMessageType_t::NVTX_MESSAGE_TYPE_ASCII as i32,
        message: nvtxMessageValue_t {
            ascii: label.as_ptr(),
        },
        ..full_attr()
    }
}

#[test]
fn mark_converts_message_and_core_payload_verbatim() {
    let label = CString::new("mark").expect("cstring");
    let attr = nvtxEventAttributes_v2 {
        payloadType: nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_UNSIGNED_INT64 as i32,
        payload: nvtxEventAttributes_v2_payload_t {
            ullValue: 0xCAFE_F00D,
        },
        ..message_attr(&label)
    };

    // SAFETY: `attr` is a valid, fully-sized attribute struct.
    let NvtxEvent::Mark { domain, attributes } = (unsafe { convert(record::mark(0x77, &attr)) })
    else {
        panic!("expected Mark");
    };
    assert_eq!(domain, 0x77);
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("mark".to_owned()))
    );
    // CORE payload union preserved verbatim.
    assert_eq!(
        attributes.payload,
        Some(NvtxPayload {
            payload_type: 1,
            value: NvtxPayloadValue::UnsignedInt64(0xCAFE_F00D),
        })
    );
}

#[test]
fn range_start_captures_id_and_attributes_and_end_matches_verbatim() {
    let label = CString::new("proc-wide").expect("cstring");
    let attr = message_attr(&label);
    let range_id: u64 = 0xDEAD_BEEF;

    // SAFETY: `attr` is a valid, fully-sized attribute struct.
    let start = unsafe { convert(record::range_start(0x11, range_id, &attr)) };
    let NvtxEvent::RangeStart {
        domain,
        range_id: started,
        attributes,
    } = start
    else {
        panic!("expected RangeStart");
    };
    assert_eq!(domain, 0x11);
    // Verbatim id — a later RangeEnd correlates on the same handle (in the analyzer).
    assert_eq!(started, range_id);
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("proc-wide".to_owned()))
    );

    // The matching end carries the identical raw id verbatim.
    let NvtxEvent::RangeEnd {
        domain,
        range_id: ended,
    } = convert(record::range_end(0x11, range_id))
    else {
        panic!("expected RangeEnd");
    };
    assert_eq!(domain, 0x11);
    assert_eq!(ended, range_id);
}

#[test]
fn domain_create_and_destroy_capture_handle_and_name() {
    let name = CString::new("quent-domain").expect("cstring");
    // SAFETY: `name` is a valid NUL-terminated C string for this call.
    let NvtxEvent::DomainCreate { domain, name: got } =
        (unsafe { convert(record::domain_create(0x5, name.as_ptr())) })
    else {
        panic!("expected DomainCreate");
    };
    assert_eq!(domain, 0x5);
    assert_eq!(got, "quent-domain");

    let NvtxEvent::DomainDestroy { domain } = convert(record::domain_destroy(0x5)) else {
        panic!("expected DomainDestroy");
    };
    assert_eq!(domain, 0x5);
}

#[test]
fn register_string_captures_value_once_and_carries_handle() {
    let value = CString::new("registered-once").expect("cstring");
    let handle: u64 = 0x2222;
    // SAFETY: `value` is a valid NUL-terminated C string for this call.
    let event = unsafe { convert(record::register_string(0x9, handle, value.as_ptr())) };
    let NvtxEvent::RegisterString {
        domain,
        handle: got_handle,
        string,
    } = event
    else {
        panic!("expected RegisterString");
    };
    assert_eq!(domain, 0x9);
    // The value is captured ONCE, at registration...
    assert_eq!(string, "registered-once");
    // ...and the raw handle is what later events reference.
    assert_eq!(got_handle, handle);
}

#[test]
fn name_category_and_name_thread_capture_names_verbatim() {
    let cat = CString::new("io").expect("cstring");
    // SAFETY: `cat` is a valid NUL-terminated C string for this call.
    let NvtxEvent::NameCategory {
        domain,
        category,
        name,
    } = (unsafe { convert(record::name_category(0x3, 7, cat.as_ptr())) })
    else {
        panic!("expected NameCategory");
    };
    assert_eq!(domain, 0x3);
    assert_eq!(category, 7);
    assert_eq!(name, "io");

    let thread = CString::new("worker-1").expect("cstring");
    // SAFETY: `thread` is a valid NUL-terminated C string for this call.
    let NvtxEvent::NameThread { thread_id, name } =
        (unsafe { convert(record::name_thread(4242, thread.as_ptr())) })
    else {
        panic!("expected NameThread");
    };
    assert_eq!(thread_id, 4242);
    assert_eq!(name, "worker-1");
}

/// A zeroed v0 resource-attribute struct with `version`/`size` set full.
fn full_resource_attr() -> nvtxResourceAttributes_v0 {
    nvtxResourceAttributes_v0 {
        version: 1,
        size: size_of::<nvtxResourceAttributes_v0>() as u16,
        identifierType: 0,
        identifier: nvtxResourceAttributes_v0_identifier_t { ullValue: 0 },
        messageType: nvtxMessageType_t::NVTX_MESSAGE_UNKNOWN as i32,
        message: nvtxMessageValue_t {
            ascii: std::ptr::null(),
        },
    }
}

#[test]
fn resource_create_captures_identifier_and_name_verbatim() {
    let name = CString::new("cuda-stream-7").expect("cstring");
    let attr = nvtxResourceAttributes_v0 {
        identifierType: 5,
        identifier: nvtxResourceAttributes_v0_identifier_t {
            ullValue: 0x1234_5678,
        },
        messageType: nvtxMessageType_t::NVTX_MESSAGE_TYPE_ASCII as i32,
        message: nvtxMessageValue_t {
            ascii: name.as_ptr(),
        },
        ..full_resource_attr()
    };

    // SAFETY: `attr` is a valid, fully-sized resource-attribute struct.
    let event = unsafe { convert(record::resource_create(0x8, 0x99, &attr)) };
    let NvtxEvent::ResourceCreate {
        domain,
        handle,
        identifier_type,
        identifier,
        message,
    } = event
    else {
        panic!("expected ResourceCreate");
    };
    assert_eq!(domain, 0x8);
    assert_eq!(handle, 0x99);
    assert_eq!(identifier_type, 5);
    assert_eq!(identifier, 0x1234_5678);
    assert_eq!(
        message,
        Some(NvtxMessage::String("cuda-stream-7".to_owned()))
    );

    let NvtxEvent::ResourceDestroy { handle } = convert(record::resource_destroy(0x99)) else {
        panic!("expected ResourceDestroy");
    };
    assert_eq!(handle, 0x99);
}

#[test]
fn resource_create_honors_size_and_registered_identifier() {
    // Claim a `size` that stops before the message member; the message must
    // not be read even though the backing struct sets it.
    let name = CString::new("must-not-read").expect("cstring");
    let truncated = (offset_of!(nvtxResourceAttributes_v0, messageType)) as u16;
    let attr = nvtxResourceAttributes_v0 {
        size: truncated,
        identifierType: 2,
        identifier: nvtxResourceAttributes_v0_identifier_t { ullValue: 0xABCD },
        messageType: nvtxMessageType_t::NVTX_MESSAGE_TYPE_ASCII as i32,
        message: nvtxMessageValue_t {
            ascii: name.as_ptr(),
        },
        ..full_resource_attr()
    };

    // SAFETY: reads are bounded by `attr.size`; the backing allocation is a
    // full struct so an over-read would still be in-bounds — the assertions
    // prove we honor `size` regardless.
    let event = unsafe { convert(record::resource_create(0x1, 0x2, &attr)) };
    let NvtxEvent::ResourceCreate {
        identifier_type,
        identifier,
        message,
        ..
    } = event
    else {
        panic!("expected ResourceCreate");
    };
    assert_eq!(identifier_type, 2);
    assert_eq!(identifier, 0xABCD);
    // messageType/message live at/after the declared `size` → not read.
    assert_eq!(message, None);
}

// ---- Default-domain (CORE) *A conversions ---------------------------------

#[test]
fn mark_a_captures_message_on_default_domain() {
    let label = CString::new("default-mark").expect("cstring");
    // SAFETY: `label` is a valid NUL-terminated C string for this call.
    let NvtxEvent::Mark { domain, attributes } =
        (unsafe { convert(record::mark_a(label.as_ptr())) })
    else {
        panic!("expected Mark");
    };
    // Default domain is represented as `0`.
    assert_eq!(domain, 0);
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("default-mark".to_owned()))
    );
    // The bare-string form carries only the message.
    assert_eq!(attributes.category, 0);
    assert_eq!(attributes.color, None);
    assert_eq!(attributes.payload, None);
}

#[test]
fn range_start_a_captures_id_and_message_on_default_domain() {
    let label = CString::new("default-start").expect("cstring");
    let range_id: u64 = 0x0BAD_F00D;
    // SAFETY: `label` is a valid NUL-terminated C string for this call.
    let NvtxEvent::RangeStart {
        domain,
        range_id: started,
        attributes,
    } = (unsafe { convert(record::range_start_a(range_id, label.as_ptr())) })
    else {
        panic!("expected RangeStart");
    };
    assert_eq!(domain, 0);
    // Verbatim id — a later RangeEnd correlates on the same handle.
    assert_eq!(started, range_id);
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("default-start".to_owned()))
    );
}

#[test]
fn mark_a_null_message_maps_to_empty_string() {
    // A NULL message is indistinguishable from an empty string (matches
    // NVTX's own "NULL == no text" treatment); it must not be dereferenced.
    // SAFETY: a null pointer is an explicitly handled input to `mark_a`.
    let NvtxEvent::Mark { domain, attributes } =
        (unsafe { convert(record::mark_a(std::ptr::null())) })
    else {
        panic!("expected Mark");
    };
    assert_eq!(domain, 0);
    assert_eq!(attributes.message, Some(NvtxMessage::String(String::new())));
}

#[test]
fn unknown_message_type_yields_no_message_at_full_size() {
    // A full-size attribute struct that declares `NVTX_MESSAGE_UNKNOWN` must
    // yield no message WITHOUT reading the message union (the tag says it is
    // absent; a caller may leave it uninitialized). `full_attr` sets the
    // union to null so an accidental read would still be observable as None,
    // but the guard means the union is never read.
    let attr = full_attr();
    assert_eq!(
        attr.messageType,
        nvtxMessageType_t::NVTX_MESSAGE_UNKNOWN as i32
    );
    // SAFETY: `attr` is a valid, fully-sized attribute struct.
    let NvtxEvent::RangePush { attributes, .. } =
        (unsafe { convert(record::range_push(0, &attr, 0)) })
    else {
        panic!("expected RangePush");
    };
    assert_eq!(attributes.message, None);
}

#[test]
fn unknown_resource_message_type_yields_no_message_at_full_size() {
    // Same guard for the resource path: a full-size resource attribute with
    // `NVTX_MESSAGE_UNKNOWN` yields no message without reading the union.
    let attr = full_resource_attr();
    assert_eq!(
        attr.messageType,
        nvtxMessageType_t::NVTX_MESSAGE_UNKNOWN as i32
    );
    // SAFETY: `attr` is a valid, fully-sized resource-attribute struct.
    let NvtxEvent::ResourceCreate { message, .. } =
        (unsafe { convert(record::resource_create(0x1, 0x2, &attr)) })
    else {
        panic!("expected ResourceCreate");
    };
    assert_eq!(message, None);
}

// ---- Null attribute pointer yields an event, never a dropped one ----------

#[test]
fn null_attr_range_push_is_captured_with_empty_attributes() {
    // A push happened and its nesting level was already handed back to the
    // app, so a null attr must still yield a RangePush (empty attributes) —
    // dropping it would leave a later pop unpaired.
    // SAFETY: a null pointer is an explicitly handled input.
    let NvtxEvent::RangePush {
        domain,
        thread_id,
        attributes,
    } = (unsafe { convert(record::range_push(0x1234, std::ptr::null(), 99)) })
    else {
        panic!("expected RangePush");
    };
    assert_eq!(domain, 0x1234);
    // The thread id is stamped even when the attribute pointer is null.
    assert_eq!(thread_id, 99);
    assert_eq!(attributes, NvtxEventAttributes::default());
}

#[test]
fn null_attr_range_start_is_captured_with_empty_attributes() {
    // SAFETY: a null pointer is an explicitly handled input.
    let NvtxEvent::RangeStart {
        range_id,
        attributes,
        ..
    } = (unsafe { convert(record::range_start(0x1, 0xABCD, std::ptr::null())) })
    else {
        panic!("expected RangeStart");
    };
    // The synthesized id survives so a later RangeEnd correlates.
    assert_eq!(range_id, 0xABCD);
    assert_eq!(attributes, NvtxEventAttributes::default());
}

#[test]
fn null_attr_resource_create_is_captured_with_empty_identity() {
    // SAFETY: a null pointer is an explicitly handled input.
    let NvtxEvent::ResourceCreate {
        handle,
        identifier_type,
        identifier,
        message,
        ..
    } = (unsafe { convert(record::resource_create(0x1, 0x99, std::ptr::null())) })
    else {
        panic!("expected ResourceCreate");
    };
    // The synthesized handle survives so a later ResourceDestroy correlates.
    assert_eq!(handle, 0x99);
    assert_eq!((identifier_type, identifier, message), (0, 0, None));
}

// ---- Wide-char (`*W`) conversions ----------------------------------------

/// Build a NUL-terminated `wchar_t` array from a Rust string slice.
///
/// On Linux `wchar_t` is `i32` (UTF-32); each `char` maps to one code unit.
fn wchar_literal(s: &str) -> Vec<wchar_t> {
    let mut v: Vec<wchar_t> = s.chars().map(|c| c as wchar_t).collect();
    v.push(0);
    v
}

#[test]
fn copy_wchar_converts_ascii_wide_string_to_utf8() {
    let wide = wchar_literal("hello");
    // SAFETY: `wide` is a valid NUL-terminated wchar_t array.
    let s = unsafe { string(record::copy_wchar(wide.as_ptr())) };
    assert_eq!(s, "hello");
}

#[test]
fn copy_wchar_converts_unicode_code_points() {
    // U+1F600 GRINNING FACE — outside the BMP, confirms 32-bit code unit handling.
    let wide = wchar_literal("café \u{1F600}");
    // SAFETY: `wide` is a valid NUL-terminated wchar_t array.
    let s = unsafe { string(record::copy_wchar(wide.as_ptr())) };
    assert_eq!(s, "café \u{1F600}");
}

#[test]
fn copy_wchar_null_maps_to_empty_string() {
    // SAFETY: null is an explicitly handled input.
    let s = unsafe { string(record::copy_wchar(std::ptr::null())) };
    assert_eq!(s, "");
}

#[test]
fn mark_w_captures_wide_message_on_default_domain() {
    let wide = wchar_literal("wide-mark");
    // SAFETY: `wide` is a valid NUL-terminated wchar_t array.
    let NvtxEvent::Mark { domain, attributes } =
        (unsafe { convert(record::mark_w(wide.as_ptr())) })
    else {
        panic!("expected Mark");
    };
    assert_eq!(domain, 0);
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("wide-mark".to_owned()))
    );
}

#[test]
fn range_push_w_captures_wide_label_with_thread_id() {
    let wide = wchar_literal("wide-push");
    // SAFETY: `wide` is a valid NUL-terminated wchar_t array.
    let NvtxEvent::RangePush {
        domain,
        thread_id,
        attributes,
    } = (unsafe { convert(record::range_push_w(wide.as_ptr(), 9999)) })
    else {
        panic!("expected RangePush");
    };
    assert_eq!(domain, 0);
    assert_eq!(thread_id, 9999);
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("wide-push".to_owned()))
    );
}

#[test]
fn range_start_w_captures_wide_label_and_id() {
    let wide = wchar_literal("wide-start");
    let range_id: u64 = 0xCAFE;
    // SAFETY: `wide` is a valid NUL-terminated wchar_t array.
    let NvtxEvent::RangeStart {
        domain,
        range_id: rid,
        attributes,
    } = (unsafe { convert(record::range_start_w(range_id, wide.as_ptr())) })
    else {
        panic!("expected RangeStart");
    };
    assert_eq!(domain, 0);
    assert_eq!(rid, range_id);
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("wide-start".to_owned()))
    );
}

#[test]
fn unicode_message_type_in_event_attributes_is_decoded() {
    // An app that uses nvtxDomainRangePushEx with messageType=UNICODE should
    // have its wide label captured, not dropped.
    let wide = wchar_literal("domain-wide");
    let attr = nvtxEventAttributes_v2 {
        messageType: nvtxMessageType_t::NVTX_MESSAGE_TYPE_UNICODE as i32,
        message: nvtxMessageValue_t {
            // The unicode union member is a *const wchar_t; store as ascii
            // field (same union, pointer width) since the generated binding
            // aliases both views.
            ascii: wide.as_ptr().cast(),
        },
        ..full_attr()
    };
    // SAFETY: `attr` is a valid, fully-sized attribute struct; `wide` lives
    // for the duration of the call.
    let NvtxEvent::RangePush { attributes, .. } =
        (unsafe { convert(record::range_push(0, &attr, 0)) })
    else {
        panic!("expected RangePush");
    };
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("domain-wide".to_owned()))
    );
}

#[test]
fn records_own_bytes_and_wide_units_before_decoding() {
    let mut bytes = b"first\xff\0".to_vec();
    // SAFETY: the buffer is NUL-terminated and readable during capture.
    let raw = unsafe { record::mark_a(bytes.as_ptr().cast()) };
    let nvtx_injection::RawEvent::Mark { attributes, .. } = &raw else {
        unreachable!()
    };
    assert_eq!(
        attributes.message,
        Some(record::RawMessage::String(record::RawString::Bytes(
            b"first\xff".to_vec()
        )))
    );
    bytes.fill(0);
    drop(bytes);
    let NvtxEvent::Mark { attributes, .. } = convert(raw) else {
        unreachable!()
    };
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("first\u{fffd}".into()))
    );

    let mut units = vec![0x1f600, 0xd800, 0];
    // SAFETY: the buffer is a readable NUL-terminated wchar_t array.
    let raw = unsafe { record::mark_w(units.as_ptr()) };
    let nvtx_injection::RawEvent::Mark { attributes, .. } = &raw else {
        unreachable!()
    };
    assert_eq!(
        attributes.message,
        Some(record::RawMessage::String(record::RawString::Wide(vec![
            0x1f600, 0xd800
        ])))
    );
    units.fill(0);
    drop(units);
    let NvtxEvent::Mark { attributes, .. } = convert(raw) else {
        unreachable!()
    };
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("😀\u{fffd}".into()))
    );
}

#[test]
fn records_own_attributes_and_narrow_payload_bits_before_decoding() {
    let mut label = b"owned\0".to_vec();
    let mut attr = full_attr();
    attr.messageType = nvtxMessageType_t::NVTX_MESSAGE_TYPE_ASCII as i32;
    attr.message = nvtxMessageValue_t {
        ascii: label.as_ptr().cast(),
    };
    attr.payloadType = nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_INT32 as i32;
    attr.payload = nvtxEventAttributes_v2_payload_t { iValue: -7 };
    // SAFETY: only the selected union members are initialized, and the message
    // remains valid throughout capture.
    let raw = unsafe { record::mark(17, &attr) };
    let nvtx_injection::RawEvent::Mark { attributes, .. } = &raw else {
        unreachable!()
    };
    assert_eq!(
        attributes.payload.as_ref().unwrap().bits,
        (-7i32) as u32 as u64
    );
    attr.payload = nvtxEventAttributes_v2_payload_t { ullValue: 0 };
    attr.message = nvtxMessageValue_t {
        ascii: std::ptr::null(),
    };
    std::hint::black_box(&attr);
    label.fill(0);
    drop(label);
    let NvtxEvent::Mark { domain, attributes } = convert(raw) else {
        unreachable!()
    };
    assert_eq!(domain, 17);
    assert_eq!(
        attributes.message,
        Some(NvtxMessage::String("owned".into()))
    );
    assert_eq!(
        attributes.payload.unwrap().value,
        NvtxPayloadValue::Int32(-7)
    );
}
