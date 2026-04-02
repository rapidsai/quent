// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Converts NVTX C types to NvtxEvent types.

#![allow(unsafe_op_in_unsafe_fn)]


use std::ffi::{CStr, c_void};

use quent_nvtx_events::{NvtxAttributes, NvtxMessage, NvtxPayload, NvtxResourceAttributes};

use crate::bindings::{nvtxEventAttributes_v2, nvtxResourceAttributes_v0};

// NVTX constants (defined as C macros, not captured by bindgen).
const NVTX_COLOR_ARGB: i32 = 1;
const NVTX_MESSAGE_TYPE_ASCII: i32 = 1;
const NVTX_MESSAGE_TYPE_UNICODE: i32 = 2;
const NVTX_MESSAGE_TYPE_REGISTERED: i32 = 3;
const NVTX_PAYLOAD_TYPE_UNSIGNED_INT64: i32 = 1;
const NVTX_PAYLOAD_TYPE_INT64: i32 = 2;
const NVTX_PAYLOAD_TYPE_DOUBLE: i32 = 3;
const NVTX_PAYLOAD_TYPE_UNSIGNED_INT32: i32 = 4;
const NVTX_PAYLOAD_TYPE_INT32: i32 = 5;
const NVTX_PAYLOAD_TYPE_FLOAT: i32 = 6;

/// Convert `nvtxEventAttributes_v2` to `NvtxAttributes`.
///
/// # Safety
///
/// `attr` must point to a valid `nvtxEventAttributes_v2` or be null.
pub(crate) unsafe fn convert_attributes(
    attr: *const nvtxEventAttributes_v2,
) -> Option<NvtxAttributes> {
    if attr.is_null() {
        return None;
    }
    let attr = &*attr;

    let color = if attr.colorType == NVTX_COLOR_ARGB {
        Some(attr.color)
    } else {
        None
    };

    let payload = convert_payload(attr.payloadType, &attr.payload);
    let message = convert_message(attr.messageType, &attr.message);

    Some(NvtxAttributes {
        category_id: attr.category,
        color,
        payload,
        message,
    })
}

/// Convert `nvtxResourceAttributes_v0` to `NvtxResourceAttributes`.
///
/// # Safety
///
/// `attr` must point to a valid `nvtxResourceAttributes_v0` or be null.
pub(crate) unsafe fn convert_resource_attributes(
    attr: *mut nvtxResourceAttributes_v0,
) -> NvtxResourceAttributes {
    if attr.is_null() {
        return NvtxResourceAttributes {
            identifier: 0,
            message: None,
        };
    }
    let attr = &*attr;
    NvtxResourceAttributes {
        identifier: attr.identifier.ullValue,
        message: convert_message(attr.messageType, &attr.message),
    }
}

// Safety: union field access is guarded by the discriminant match — each arm
// only reads the variant that payloadType/messageType guarantees is active.

unsafe fn convert_payload(
    payload_type: i32,
    payload: &crate::bindings::nvtxEventAttributes_v2_payload_t,
) -> Option<NvtxPayload> {
    match payload_type {
        NVTX_PAYLOAD_TYPE_UNSIGNED_INT64 => Some(NvtxPayload::U64(payload.ullValue)),
        NVTX_PAYLOAD_TYPE_INT64 => Some(NvtxPayload::I64(payload.llValue)),
        NVTX_PAYLOAD_TYPE_DOUBLE => Some(NvtxPayload::F64(payload.dValue)),
        NVTX_PAYLOAD_TYPE_UNSIGNED_INT32 => Some(NvtxPayload::U32(payload.uiValue)),
        NVTX_PAYLOAD_TYPE_INT32 => Some(NvtxPayload::I32(payload.iValue)),
        NVTX_PAYLOAD_TYPE_FLOAT => Some(NvtxPayload::F32(payload.fValue)),
        _ => None,
    }
}

unsafe fn convert_message(
    msg_type: i32,
    msg: &crate::bindings::nvtxMessageValue_t,
) -> Option<NvtxMessage> {
    match msg_type {
        NVTX_MESSAGE_TYPE_ASCII => {
            let ptr = msg.ascii;
            if ptr.is_null() {
                return None;
            }
            Some(NvtxMessage::String(
                CStr::from_ptr(ptr).to_string_lossy().into_owned(),
            ))
        }
        NVTX_MESSAGE_TYPE_UNICODE => {
            let ptr = msg.unicode;
            if ptr.is_null() {
                return None;
            }
            Some(NvtxMessage::String(wchar_to_string(ptr)))
        }
        NVTX_MESSAGE_TYPE_REGISTERED => {
            let handle = msg.registered;
            if handle.is_null() {
                return None;
            }
            Some(NvtxMessage::RegisteredHandle(*(handle as *const u64)))
        }
        _ => None,
    }
}

/// Convert a null-terminated wchar_t string to a Rust String.
/// Assumes wchar_t is 4 bytes (UTF-32), which holds on Linux and macOS.
#[cfg(not(target_os = "windows"))]
unsafe fn wchar_to_string(ptr: *const i32) -> String {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr as *const u32, len);
    slice
        .iter()
        .map(|&c| char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// Build `NvtxAttributes` from a plain ASCII message string pointer.
pub(crate) unsafe fn attributes_from_ascii(msg: *const i8) -> Option<NvtxAttributes> {
    if msg.is_null() {
        return None;
    }
    Some(NvtxAttributes {
        message: Some(NvtxMessage::String(
            CStr::from_ptr(msg).to_string_lossy().into_owned(),
        )),
        ..Default::default()
    })
}

/// Build `NvtxAttributes` from a plain wchar_t message string pointer.
pub(crate) unsafe fn attributes_from_wchar(msg: *const i32) -> Option<NvtxAttributes> {
    if msg.is_null() {
        return None;
    }
    Some(NvtxAttributes {
        message: Some(NvtxMessage::String(wchar_to_string(msg))),
        ..Default::default()
    })
}

/// Extract domain handle ID. Returns `None` for the default domain (null).
pub(crate) unsafe fn domain_handle_id(handle: *mut c_void) -> Option<u64> {
    if handle.is_null() {
        None
    } else {
        Some(*(handle as *const u64))
    }
}

/// Read a C string, returning an empty string for null pointers.
pub(crate) unsafe fn cstr_to_string(ptr: *const i8) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// Read a wchar_t string, returning an empty string for null pointers.
pub(crate) unsafe fn wstr_to_string(ptr: *const i32) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        wchar_to_string(ptr)
    }
}
