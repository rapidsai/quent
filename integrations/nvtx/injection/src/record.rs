// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Owned callback records with undecoded strings and payload bits.

use nvtx_sys::ffi::{
    nvtxColorType_t, nvtxEventAttributes_t, nvtxEventAttributes_v2, nvtxMessageType_t,
    nvtxPayloadType_t, nvtxResourceAttributes_t, wchar_t,
};
use std::ffi::CStr;
use std::mem::{offset_of, size_of};
use std::os::raw::c_char;

/// Caller-owned text copied without UTF decoding or a trailing NUL.
#[derive(Debug, Clone, PartialEq)]
pub enum RawString {
    Bytes(Vec<u8>),
    Wide(Vec<u32>),
}

/// An owned immediate message or a registered-string handle.
#[derive(Debug, Clone, PartialEq)]
pub enum RawMessage {
    String(RawString),
    RegisteredHandle(u64),
}

/// A raw color tag and value.
#[derive(Debug, Clone, PartialEq)]
pub struct RawColor {
    pub color_type: i32,
    pub value: u32,
}

/// Payload bits copied at the width declared by `payload_type`.
#[derive(Debug, Clone, PartialEq)]
pub struct RawPayload {
    pub payload_type: i32,
    pub bits: u64,
}

/// Owned fields present within the caller's declared attribute size.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RawAttributes {
    pub category: u32,
    pub color: Option<RawColor>,
    pub message: Option<RawMessage>,
    pub payload: Option<RawPayload>,
}

/// An owned NVTX call that can be decoded after the caller's buffers are released.
#[derive(Debug, Clone, PartialEq)]
pub enum RawEvent {
    RangePush {
        domain: u64,
        thread_id: u32,
        attributes: RawAttributes,
    },
    RangePop {
        domain: u64,
        thread_id: u32,
    },
    RangeStart {
        domain: u64,
        range_id: u64,
        attributes: RawAttributes,
    },
    RangeEnd {
        domain: u64,
        range_id: u64,
    },
    Mark {
        domain: u64,
        attributes: RawAttributes,
    },
    DomainCreate {
        domain: u64,
        name: RawString,
    },
    DomainDestroy {
        domain: u64,
    },
    RegisterString {
        domain: u64,
        handle: u64,
        string: RawString,
    },
    NameCategory {
        domain: u64,
        category: u32,
        name: RawString,
    },
    NameThread {
        thread_id: u32,
        name: RawString,
    },
    ResourceCreate {
        domain: u64,
        handle: u64,
        identifier_type: i32,
        identifier: u64,
        message: Option<RawMessage>,
    },
    ResourceDestroy {
        handle: u64,
    },
}

/// Copy a `DomainRangePop` call to a verbatim [`RawEvent::RangePop`].
///
/// `thread_id` is the OS thread id read on the app thread by the callback (this
/// fn stays pure and does not read it here), so a pop pairs with the push on the
/// same thread.
#[inline(always)]
pub fn range_pop(domain: u64, thread_id: u32) -> RawEvent {
    RawEvent::RangePop { domain, thread_id }
}

/// Copy a `DomainMarkEx` call to a verbatim [`RawEvent::Mark`].
///
/// # Safety
/// See [`range_push`].
#[inline(always)]
pub unsafe fn mark(domain: u64, attr: *const nvtxEventAttributes_t) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `attr`.
    let attributes = unsafe { read_attributes_or_empty(attr) };
    RawEvent::Mark { domain, attributes }
}

/// Copy a `DomainRangeStartEx` call to a verbatim [`RawEvent::RangeStart`].
///
/// `range_id` is the id the injection layer synthesized and returns to the app,
/// captured verbatim so a later `DomainRangeEnd` correlates (the analyzer pairs them).
///
/// # Safety
/// See [`range_push`].
#[inline(always)]
pub unsafe fn range_start(
    domain: u64,
    range_id: u64,
    attr: *const nvtxEventAttributes_t,
) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `attr`.
    let attributes = unsafe { read_attributes_or_empty(attr) };
    RawEvent::RangeStart {
        domain,
        range_id,
        attributes,
    }
}

/// Copy a `DomainRangeEnd` call to a verbatim [`RawEvent::RangeEnd`].
#[inline(always)]
pub fn range_end(domain: u64, range_id: u64) -> RawEvent {
    RawEvent::RangeEnd { domain, range_id }
}

/// Copy a `DomainCreateA` call to a verbatim [`RawEvent::DomainCreate`].
///
/// `domain` is the handle the injection layer synthesized and returns to the app.
///
/// # Safety
/// `name` must be null or a valid NUL-terminated C string readable for this call;
/// it is copied in before returning.
#[inline(always)]
pub unsafe fn domain_create(domain: u64, name: *const c_char) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `name`.
    let name = unsafe { copy_cstr(name) };
    RawEvent::DomainCreate { domain, name }
}

/// Copy a `DomainDestroy` call to a verbatim [`RawEvent::DomainDestroy`].
#[inline(always)]
pub fn domain_destroy(domain: u64) -> RawEvent {
    RawEvent::DomainDestroy { domain }
}

/// Copy a `DomainRegisterStringA` call to a verbatim
/// [`RawEvent::RegisterString`].
///
/// The string value is captured ONCE here at registration; every
/// later event that references it carries only the raw `handle`.
///
/// # Safety
/// `string` must be null or a valid NUL-terminated C string readable for this
/// call; it is copied in before returning.
#[inline(always)]
pub unsafe fn register_string(domain: u64, handle: u64, string: *const c_char) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `string`.
    let string = unsafe { copy_cstr(string) };
    RawEvent::RegisterString {
        domain,
        handle,
        string,
    }
}

/// Copy a `DomainNameCategoryA` call to a verbatim [`RawEvent::NameCategory`].
///
/// # Safety
/// `name` must be null or a valid NUL-terminated C string readable for this call.
#[inline(always)]
pub unsafe fn name_category(domain: u64, category: u32, name: *const c_char) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `name`.
    let name = unsafe { copy_cstr(name) };
    RawEvent::NameCategory {
        domain,
        category,
        name,
    }
}

/// Copy a `NameOsThreadA` call to a verbatim [`RawEvent::NameThread`].
///
/// # Safety
/// `name` must be null or a valid NUL-terminated C string readable for this call.
#[inline(always)]
pub unsafe fn name_thread(thread_id: u32, name: *const c_char) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `name`.
    let name = unsafe { copy_cstr(name) };
    RawEvent::NameThread { thread_id, name }
}

/// Copy a `DomainResourceCreate` call to a verbatim
/// [`RawEvent::ResourceCreate`].
///
/// `handle` is the resource handle the injection layer synthesized and returns to
/// the app. The identifier tag/value are captured verbatim (raw bits, undecoded).
///
/// # Safety
/// `attr` must be null, or point to a valid `nvtxResourceAttributes_t` whose
/// `size` member truthfully describes the readable bytes. A null pointer yields
/// a resource with empty identity so the event is still captured.
#[inline(always)]
pub unsafe fn resource_create(
    domain: u64,
    handle: u64,
    attr: *const nvtxResourceAttributes_t,
) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `attr`.
    let (identifier_type, identifier, message) = unsafe { read_resource(attr) };
    RawEvent::ResourceCreate {
        domain,
        handle,
        identifier_type,
        identifier,
        message,
    }
}

/// Copy a `DomainResourceDestroy` call to a verbatim
/// [`RawEvent::ResourceDestroy`].
#[inline(always)]
pub fn resource_destroy(handle: u64) -> RawEvent {
    RawEvent::ResourceDestroy { handle }
}

/// Copy a `DomainRangePushEx` call to a verbatim [`RawEvent::RangePush`].
///
/// `thread_id` is the OS thread id read on the app thread by the callback (this
/// fn stays pure and does not read it here), so a push pairs with its pop on the
/// same thread.
///
/// # Safety
/// `attr` must be null, or point to a valid `nvtxEventAttributes_t` whose `size`
/// member truthfully describes the number of readable bytes. A null pointer
/// yields empty attributes so the push is still captured (never dropped),
/// keeping push/pop pairing balanced.
#[inline(always)]
pub unsafe fn range_push(
    domain: u64,
    attr: *const nvtxEventAttributes_t,
    thread_id: u32,
) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `attr`.
    let attributes = unsafe { read_attributes_or_empty(attr) };
    RawEvent::RangePush {
        domain,
        thread_id,
        attributes,
    }
}

/// Build a message-only [`RawAttributes`] from a caller-owned C string.
///
/// The classic default-domain `*A` calls (`nvtxMarkA`, `nvtxRangePushA`,
/// `nvtxRangeStartA`) carry a bare `const char*` rather than a full attribute
/// struct, so their only attribute is the immediate message.
///
/// # Safety
/// `message` must be null or a valid NUL-terminated C string readable for this
/// call; it is copied in before returning.
#[inline(always)]
unsafe fn message_only_attributes(message: *const c_char) -> RawAttributes {
    RawAttributes {
        // SAFETY: forwarded from the caller's contract on `message`.
        message: Some(RawMessage::String(unsafe { copy_cstr(message) })),
        ..Default::default()
    }
}

/// Copy a default-domain `nvtxMarkA` call to a verbatim [`RawEvent::Mark`]
/// on the default domain (`0`).
///
/// # Safety
/// `message` must be null or a readable NUL-terminated C string.
#[inline(always)]
pub unsafe fn mark_a(message: *const c_char) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `message`.
    let attributes = unsafe { message_only_attributes(message) };
    RawEvent::Mark {
        domain: 0,
        attributes,
    }
}

/// Copy a default-domain `nvtxRangePushA` call to a verbatim
/// [`RawEvent::RangePush`] on the default domain (`0`).
///
/// `thread_id` is the OS thread id read on the app thread by the callback.
///
/// # Safety
/// `message` must be null or a readable NUL-terminated C string.
#[inline(always)]
pub unsafe fn range_push_a(message: *const c_char, thread_id: u32) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `message`.
    let attributes = unsafe { message_only_attributes(message) };
    RawEvent::RangePush {
        domain: 0,
        thread_id,
        attributes,
    }
}

/// Copy a default-domain `nvtxRangeStartA` call to a verbatim
/// [`RawEvent::RangeStart`] on the default domain (`0`).
///
/// `range_id` is the id the injection layer synthesized and returns to the app,
/// captured verbatim so a later `nvtxRangeEnd` correlates.
///
/// # Safety
/// `message` must be null or a readable NUL-terminated C string.
#[inline(always)]
pub unsafe fn range_start_a(range_id: u64, message: *const c_char) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `message`.
    let attributes = unsafe { message_only_attributes(message) };
    RawEvent::RangeStart {
        domain: 0,
        range_id,
        attributes,
    }
}

/// Read the captured attributes, or empty attributes if `attr` is null.
///
/// A null attribute pointer means "no metadata". The mark/range event still
/// happened, so we return [`RawAttributes::default`] and let the caller
/// emit the event rather than dropping it — dropping a push/start would desync
/// the captured stream from the nesting level / range id already handed back to
/// the app (an orphan pop/end later).
///
/// # Safety
/// `attr` must be null, or valid per [`read_attributes`].
#[inline(always)]
unsafe fn read_attributes_or_empty(attr: *const nvtxEventAttributes_t) -> RawAttributes {
    if attr.is_null() {
        return RawAttributes::default();
    }
    // SAFETY: non-null and valid per the caller's contract.
    unsafe { read_attributes(attr) }
}

/// Read the captured subset of an attribute struct, honoring its `size` bound.
///
/// # Safety
/// `attr` must be non-null. Reads go through unaligned raw-pointer loads and
/// never materialize a reference to the whole struct, so a smaller-than-`v2` app
/// struct is safe as long as `size` is honest.
#[inline(always)]
unsafe fn read_attributes(attr: *const nvtxEventAttributes_t) -> RawAttributes {
    let base = attr.cast::<u8>();
    // `size` (u16) sits immediately after `version` (u16) at the head of the
    // struct; it is present for any valid attribute pointer.
    // SAFETY: offset(size) + 2 is within any real attribute allocation.
    let size = unsafe { read_at::<u16>(base, offset_of!(nvtxEventAttributes_v2, size)) } as usize;

    // SAFETY: each read is guarded by `size` before dereferencing.
    let category =
        unsafe { read_present::<u32>(base, size, offset_of!(nvtxEventAttributes_v2, category)) }
            .unwrap_or(0);
    let color = unsafe { read_color(base, size) };
    let payload = unsafe { read_payload(base, size) };
    let message = unsafe { read_message(base, size) };

    RawAttributes {
        category,
        color,
        message,
        payload,
    }
}

/// Unaligned read of a `Copy` value at `base + offset`.
///
/// # Safety
/// `base + offset .. base + offset + size_of::<T>()` must be readable.
#[inline(always)]
unsafe fn read_at<T: Copy>(base: *const u8, offset: usize) -> T {
    // SAFETY: readability guaranteed by the caller; `read_unaligned` tolerates
    // any field alignment the foreign struct may have.
    unsafe { base.add(offset).cast::<T>().read_unaligned() }
}

/// Read a value only if the struct's `size` declares those bytes present.
///
/// # Safety
/// Same contract as [`read_at`] once the `size` guard passes.
#[inline(always)]
unsafe fn read_present<T: Copy>(base: *const u8, size: usize, offset: usize) -> Option<T> {
    if offset + size_of::<T>() <= size {
        // SAFETY: the guard proves the bytes are within `size`.
        Some(unsafe { read_at::<T>(base, offset) })
    } else {
        None
    }
}

/// Read the color attribute verbatim (raw `colorType` tag + value).
///
/// # Safety
/// See [`read_attributes`].
#[inline(always)]
unsafe fn read_color(base: *const u8, size: usize) -> Option<RawColor> {
    let color_type =
        unsafe { read_present::<i32>(base, size, offset_of!(nvtxEventAttributes_v2, colorType)) }?;
    if color_type == nvtxColorType_t::NVTX_COLOR_UNKNOWN as i32 {
        return None;
    }
    let value =
        unsafe { read_present::<u32>(base, size, offset_of!(nvtxEventAttributes_v2, color)) }?;
    Some(RawColor { color_type, value })
}

/// Copy the initialized payload bits without decoding their numeric type.
///
/// # Safety
/// See [`read_attributes`].
#[inline(always)]
unsafe fn read_payload(base: *const u8, size: usize) -> Option<RawPayload> {
    let payload_type = unsafe {
        read_present::<i32>(base, size, offset_of!(nvtxEventAttributes_v2, payloadType))
    }?;
    if payload_type == nvtxPayloadType_t::NVTX_PAYLOAD_UNKNOWN as i32 {
        return None;
    }
    // The payload is an 8-byte union; read exactly the width the tag names so a
    // 32-bit member never pulls in the union's (possibly uninitialized) upper
    // four bytes. `offset` is the union's start; on the supported little-endian
    // x86-64 and aarch64 Linux targets each member occupies the low bytes.
    let offset = offset_of!(nvtxEventAttributes_v2, payload);
    let bits = match payload_type {
        tag if tag == nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_UNSIGNED_INT32 as i32
            || tag == nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_INT32 as i32
            || tag == nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_FLOAT as i32 =>
        {
            unsafe { read_present::<u32>(base, size, offset) }? as u64
        }
        _ => unsafe { read_present::<u64>(base, size, offset) }?,
    };
    Some(RawPayload { payload_type, bits })
}

/// Read the message attribute, copying immediate strings in and
/// keeping only the raw handle for registered strings (resolved in the analyzer).
///
/// # Safety
/// See [`read_attributes`]; the message union holds a pointer valid only for the
/// call's duration, which is copied before returning.
#[inline(always)]
unsafe fn read_message(base: *const u8, size: usize) -> Option<RawMessage> {
    let message_type = unsafe {
        read_present::<i32>(base, size, offset_of!(nvtxEventAttributes_v2, messageType))
    }?;
    // Only read the union for a tag that carries a value. `NVTX_MESSAGE_UNKNOWN`
    // (the "no message" default) says the union is absent, and a caller that
    // leaves it uninitialized would otherwise have those bytes read. Mirrors the
    // `read_payload` guard.
    if message_type == nvtxMessageType_t::NVTX_MESSAGE_UNKNOWN as i32 {
        return None;
    }
    // The message union is pointer-sized; capture the raw pointer bits.
    let bits =
        unsafe { read_present::<usize>(base, size, offset_of!(nvtxEventAttributes_v2, message)) }?;
    // SAFETY: `bits` came from the caller's message union for `message_type`.
    unsafe { copy_message(message_type, bits) }
}

/// Copy a raw `(messageType, message-union bits)` pair into an owned
/// [`RawMessage`], copying immediate strings in and keeping only the
/// raw handle for registered strings (resolved in the analyzer).
///
/// Shared by the event-attribute and resource-attribute readers.
///
/// # Safety
/// If `message_type` is `NVTX_MESSAGE_TYPE_ASCII`, `bits` must be a `const char*`
/// valid for this call. If `NVTX_MESSAGE_TYPE_UNICODE`, `bits` must be a
/// `const wchar_t*` valid for this call. Both are copied before returning.
#[inline(always)]
unsafe fn copy_message(message_type: i32, bits: usize) -> Option<RawMessage> {
    match message_type {
        value if value == nvtxMessageType_t::NVTX_MESSAGE_TYPE_ASCII as i32 => {
            // SAFETY: NVTX guarantees the const char* is valid for the call; the
            // bytes are copied into an owned buffer before returning.
            Some(RawMessage::String(unsafe {
                copy_cstr(bits as *const c_char)
            }))
        }
        value if value == nvtxMessageType_t::NVTX_MESSAGE_TYPE_UNICODE as i32 => {
            // SAFETY: NVTX guarantees the const wchar_t* is valid for the call;
            // the code units are copied into an owned buffer before returning.
            Some(RawMessage::String(unsafe {
                copy_wchar(bits as *const wchar_t)
            }))
        }
        value if value == nvtxMessageType_t::NVTX_MESSAGE_TYPE_REGISTERED as i32 => {
            Some(RawMessage::RegisteredHandle(bits as u64))
        }
        // The default "no message" sentinel — silently absent, not an
        // unsupported encoding.
        _ => None,
    }
}

/// Read the captured subset of a resource-attribute struct, honoring its `size`
/// bound. Returns `(identifierType, identifier bits, optional name)`,
/// all verbatim.
///
/// # Safety
/// `attr` must be null, or point to a valid `nvtxResourceAttributes_t` whose
/// `size` member truthfully describes the readable bytes. A null pointer yields
/// `(0, 0, None)` so the resource is still captured (never dropped), keeping the
/// synthesized handle paired with a later `ResourceDestroy`.
#[inline(always)]
unsafe fn read_resource(attr: *const nvtxResourceAttributes_t) -> (i32, u64, Option<RawMessage>) {
    use nvtx_sys::ffi::nvtxResourceAttributes_v0 as Res;

    if attr.is_null() {
        return (0, 0, None);
    }

    let base = attr.cast::<u8>();
    // `size` (u16) sits immediately after `version` (u16) at the struct head.
    // SAFETY: offset(size) + 2 is within any real attribute allocation.
    let size = unsafe { read_at::<u16>(base, offset_of!(Res, size)) } as usize;

    // SAFETY: each read is guarded by `size` before dereferencing.
    let identifier_type =
        unsafe { read_present::<i32>(base, size, offset_of!(Res, identifierType)) }.unwrap_or(0);
    // The identifier union is 8 bytes; capture its raw bits verbatim.
    let identifier =
        unsafe { read_present::<u64>(base, size, offset_of!(Res, identifier)) }.unwrap_or(0);

    let message = match unsafe { read_present::<i32>(base, size, offset_of!(Res, messageType)) } {
        // Only read the union for a tag that carries a value; `UNKNOWN` (the
        // "no message" default) says it is absent, and a caller may leave the
        // union uninitialized. Mirrors the event-attribute `read_message` guard.
        Some(message_type) if message_type != nvtxMessageType_t::NVTX_MESSAGE_UNKNOWN as i32 => {
            // SAFETY: the message union is pointer-sized; guarded by `size`.
            match unsafe { read_present::<usize>(base, size, offset_of!(Res, message)) } {
                // SAFETY: `bits` is the message union for `message_type`.
                Some(bits) => unsafe { copy_message(message_type, bits) },
                None => None,
            }
        }
        _ => None,
    };

    (identifier_type, identifier, message)
}

/// Copy a caller-owned C string without decoding its bytes.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated string readable for this call.
#[inline(always)]
unsafe fn copy_cstr(ptr: *const c_char) -> RawString {
    let bytes = if ptr.is_null() {
        Vec::new()
    } else {
        // SAFETY: the caller guarantees a readable NUL-terminated string.
        unsafe { CStr::from_ptr(ptr) }.to_bytes().to_vec()
    };
    RawString::Bytes(bytes)
}

/// Copy a caller-owned wide string without decoding its code units.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated wide string readable for this call.
#[inline(always)]
pub unsafe fn copy_wchar(ptr: *const wchar_t) -> RawString {
    let mut units = Vec::new();
    if !ptr.is_null() {
        let mut p = ptr;
        // SAFETY: the caller guarantees a readable NUL-terminated array.
        while unsafe { *p } != 0 {
            // SAFETY: this element precedes the terminating NUL.
            units.push(unsafe { *p } as u32);
            // SAFETY: the next element is in the same readable array.
            p = unsafe { p.add(1) };
        }
    }
    RawString::Wide(units)
}

/// Build a message-only [`RawAttributes`] from a caller-owned wide string.
///
/// The wide-char default-domain `*W` calls (`nvtxMarkW`, `nvtxRangePushW`,
/// `nvtxRangeStartW`) carry a bare `const wchar_t*`; the only attribute is the
/// immediate message, copied without decoding.
///
/// # Safety
/// `message` must be null or a valid NUL-terminated `wchar_t` array readable
/// for this call; the code points are copied before returning.
#[inline(always)]
pub unsafe fn message_only_attributes_w(message: *const wchar_t) -> RawAttributes {
    RawAttributes {
        // SAFETY: forwarded from the caller's contract on `message`.
        message: Some(RawMessage::String(unsafe { copy_wchar(message) })),
        ..Default::default()
    }
}

/// Copy a default-domain `nvtxMarkW` call to a verbatim [`RawEvent::Mark`]
/// on the default domain (`0`).
///
/// # Safety
/// `message` must be null or a readable NUL-terminated wide string.
#[inline(always)]
pub unsafe fn mark_w(message: *const wchar_t) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `message`.
    let attributes = unsafe { message_only_attributes_w(message) };
    RawEvent::Mark {
        domain: 0,
        attributes,
    }
}

/// Copy a default-domain `nvtxRangePushW` call to a verbatim
/// [`RawEvent::RangePush`] on the default domain (`0`).
///
/// # Safety
/// `message` must be null or a readable NUL-terminated wide string.
#[inline(always)]
pub unsafe fn range_push_w(message: *const wchar_t, thread_id: u32) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `message`.
    let attributes = unsafe { message_only_attributes_w(message) };
    RawEvent::RangePush {
        domain: 0,
        thread_id,
        attributes,
    }
}

/// Copy a default-domain `nvtxRangeStartW` call to a verbatim
/// [`RawEvent::RangeStart`] on the default domain (`0`).
///
/// # Safety
/// `message` must be null or a readable NUL-terminated wide string.
#[inline(always)]
pub unsafe fn range_start_w(range_id: u64, message: *const wchar_t) -> RawEvent {
    // SAFETY: forwarded from the caller's contract on `message`.
    let attributes = unsafe { message_only_attributes_w(message) };
    RawEvent::RangeStart {
        domain: 0,
        range_id,
        attributes,
    }
}
