// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Pure, side-effect-free conversion from NVTX callback arguments to verbatim
//! [`NvtxEvent`]s.
//!
//! Everything here is deterministic and touches no global state, so it is
//! unit-testable in-process even though the surrounding injection machinery is a
//! process-global one-shot (Pitfall 6 / VAL-02). Two safety disciplines live
//! here:
//!
//! * **String copy-in (Pitfall 3):** immediate `const char*` messages are copied
//!   into an owned `String` before returning — the caller's pointer is valid
//!   only for the call's duration, so nothing may outlive it.
//! * **Bounded reads (Pitfall 4):** `nvtxEventAttributes_t` carries an explicit
//!   `size`; we read only the members `size` declares present, so an app built
//!   against an older/smaller NVTX layout never triggers an over-read.

use std::ffi::CStr;
use std::mem::{offset_of, size_of};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};

use quent_nvtx_events::{
    NvtxColor, NvtxEvent, NvtxEventAttributes, NvtxMessage, NvtxPayload, NvtxPayloadValue,
};

use crate::bindings::{
    nvtxColorType_t, nvtxEventAttributes_t, nvtxEventAttributes_v2, nvtxMessageType_t,
    nvtxPayloadType_t,
};

/// Hand-written ABI declarations for the CORE/CORE2 kinds plan 04 wires that the
/// committed `bindings.rs` allowlist (D-14) intentionally omits.
///
/// `bindings.rs` is a committed bindgen artifact whose allowlist covers only the
/// injection surface plan 02 needed (push/pop + the event-attribute struct). The
/// range-id and resource types below are the small additional NVTX ABI surface
/// this plan needs; they are declared by hand — `#[repr(C)]` so `offset_of!`
/// reads match the NVTX C layout — rather than regenerating (and growing) the
/// committed bindings, which would require libclang and rewrite a plan-02
/// artifact.
///
/// IN-04: these resource/range-id ABI types are intentionally hand-maintained
/// and are **NOT** part of the regenerated `src/bindings.rs`. Running
/// `build.rs` under `regenerate-bindings` will not (re)produce them — that is
/// expected, not a sign the bindings are incomplete. Keep this module in sync
/// with the NVTX C headers by hand on an NVTX `rev` bump.
#[allow(nonstandard_style, dead_code)]
pub(crate) mod abi {
    use std::os::raw::c_void;

    use crate::bindings::nvtxMessageValue_t;

    /// `nvtxRangeId_t` — a process-wide range correlation id (NVTX: `uint64_t`).
    pub(crate) type nvtxRangeId_t = u64;

    /// The resource identifier union (`{ const void* pValue; uint64_t ullValue }`).
    #[repr(C)]
    pub(crate) union nvtxResourceIdentifier_t {
        pub p_value: *const c_void,
        pub ull_value: u64,
    }

    /// `nvtxResourceAttributes_v0` — resource identity tag/value + optional name.
    #[repr(C)]
    pub(crate) struct nvtxResourceAttributes_v0 {
        pub version: u16,
        pub size: u16,
        pub identifierType: i32,
        pub identifier: nvtxResourceIdentifier_t,
        pub messageType: i32,
        pub message: nvtxMessageValue_t,
    }

    /// Current resource-attribute struct version (mirrors the injection ABI).
    pub(crate) type nvtxResourceAttributes_t = nvtxResourceAttributes_v0;

    /// Opaque `nvtxResourceHandle_t` — captured verbatim as raw bits (D-01).
    #[repr(C)]
    pub(crate) struct nvtxResourceHandle {
        _unused: [u8; 0],
    }
    /// Pointer-sized opaque resource handle.
    pub(crate) type nvtxResourceHandle_t = *mut nvtxResourceHandle;
}

/// Convert a `DomainRangePop` call to a verbatim [`NvtxEvent::RangePop`].
pub(crate) fn range_pop(domain: u64) -> NvtxEvent {
    NvtxEvent::RangePop { domain }
}

/// Convert a `DomainMarkEx` call to a verbatim [`NvtxEvent::Mark`].
///
/// # Safety
/// See [`range_push`].
pub(crate) unsafe fn mark(domain: u64, attr: *const nvtxEventAttributes_t) -> NvtxEvent {
    // SAFETY: forwarded from the caller's contract on `attr`.
    let attributes = unsafe { read_attributes(attr) };
    NvtxEvent::Mark { domain, attributes }
}

/// Convert a `DomainRangeStartEx` call to a verbatim [`NvtxEvent::RangeStart`].
///
/// `range_id` is the id the injection layer synthesized and returns to the app,
/// captured verbatim so a later `DomainRangeEnd` correlates (pairing is Phase 2).
///
/// # Safety
/// See [`range_push`].
pub(crate) unsafe fn range_start(
    domain: u64,
    range_id: u64,
    attr: *const nvtxEventAttributes_t,
) -> NvtxEvent {
    // SAFETY: forwarded from the caller's contract on `attr`.
    let attributes = unsafe { read_attributes(attr) };
    NvtxEvent::RangeStart {
        domain,
        range_id,
        attributes,
    }
}

/// Convert a `DomainRangeEnd` call to a verbatim [`NvtxEvent::RangeEnd`].
pub(crate) fn range_end(domain: u64, range_id: u64) -> NvtxEvent {
    NvtxEvent::RangeEnd { domain, range_id }
}

/// Convert a `DomainCreateA` call to a verbatim [`NvtxEvent::DomainCreate`].
///
/// `domain` is the handle the injection layer synthesized and returns to the app.
///
/// # Safety
/// `name` must be null or a valid NUL-terminated C string readable for this call;
/// it is copied in before returning (Pitfall 3).
pub(crate) unsafe fn domain_create(domain: u64, name: *const c_char) -> NvtxEvent {
    // SAFETY: forwarded from the caller's contract on `name`.
    let name = unsafe { copy_cstr(name) };
    NvtxEvent::DomainCreate { domain, name }
}

/// Convert a `DomainDestroy` call to a verbatim [`NvtxEvent::DomainDestroy`].
pub(crate) fn domain_destroy(domain: u64) -> NvtxEvent {
    NvtxEvent::DomainDestroy { domain }
}

/// Convert a `DomainRegisterStringA` call to a verbatim
/// [`NvtxEvent::RegisterString`].
///
/// The string value is captured ONCE here at registration (Pitfall 3); every
/// later event that references it carries only the raw `handle`.
///
/// # Safety
/// `string` must be null or a valid NUL-terminated C string readable for this
/// call; it is copied in before returning.
pub(crate) unsafe fn register_string(domain: u64, handle: u64, string: *const c_char) -> NvtxEvent {
    // SAFETY: forwarded from the caller's contract on `string`.
    let string = unsafe { copy_cstr(string) };
    NvtxEvent::RegisterString {
        domain,
        handle,
        string,
    }
}

/// Convert a `DomainNameCategoryA` call to a verbatim [`NvtxEvent::NameCategory`].
///
/// # Safety
/// `name` must be null or a valid NUL-terminated C string readable for this call.
pub(crate) unsafe fn name_category(domain: u64, category: u32, name: *const c_char) -> NvtxEvent {
    // SAFETY: forwarded from the caller's contract on `name`.
    let name = unsafe { copy_cstr(name) };
    NvtxEvent::NameCategory {
        domain,
        category,
        name,
    }
}

/// Convert a `NameOsThreadA` call to a verbatim [`NvtxEvent::NameThread`].
///
/// # Safety
/// `name` must be null or a valid NUL-terminated C string readable for this call.
pub(crate) unsafe fn name_thread(thread_id: u32, name: *const c_char) -> NvtxEvent {
    // SAFETY: forwarded from the caller's contract on `name`.
    let name = unsafe { copy_cstr(name) };
    NvtxEvent::NameThread { thread_id, name }
}

/// Convert a `DomainResourceCreate` call to a verbatim
/// [`NvtxEvent::ResourceCreate`].
///
/// `handle` is the resource handle the injection layer synthesized and returns to
/// the app. The identifier tag/value are captured verbatim (raw bits, undecoded).
///
/// # Safety
/// `attr` must be non-null and point to a valid `nvtxResourceAttributes_t` whose
/// `size` member truthfully describes the readable bytes.
pub(crate) unsafe fn resource_create(
    domain: u64,
    handle: u64,
    attr: *const abi::nvtxResourceAttributes_t,
) -> NvtxEvent {
    // SAFETY: forwarded from the caller's contract on `attr`.
    let (identifier_type, identifier, message) = unsafe { read_resource(attr) };
    NvtxEvent::ResourceCreate {
        domain,
        handle,
        identifier_type,
        identifier,
        message,
    }
}

/// Convert a `DomainResourceDestroy` call to a verbatim
/// [`NvtxEvent::ResourceDestroy`].
pub(crate) fn resource_destroy(handle: u64) -> NvtxEvent {
    NvtxEvent::ResourceDestroy { handle }
}

/// Convert a `DomainRangePushEx` call to a verbatim [`NvtxEvent::RangePush`].
///
/// # Safety
/// `attr` must be non-null and point to a valid `nvtxEventAttributes_t` whose
/// `size` member truthfully describes the number of readable bytes.
pub(crate) unsafe fn range_push(domain: u64, attr: *const nvtxEventAttributes_t) -> NvtxEvent {
    // SAFETY: forwarded from the caller's contract on `attr`.
    let attributes = unsafe { read_attributes(attr) };
    NvtxEvent::RangePush { domain, attributes }
}

/// Read the captured subset of an attribute struct, honoring its `size` bound.
///
/// # Safety
/// See [`range_push`]. Reads go through unaligned raw-pointer loads and never
/// materialize a reference to the whole struct, so a smaller-than-`v2` app
/// struct is safe as long as `size` is honest.
unsafe fn read_attributes(attr: *const nvtxEventAttributes_t) -> NvtxEventAttributes {
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

    NvtxEventAttributes {
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
unsafe fn read_at<T: Copy>(base: *const u8, offset: usize) -> T {
    // SAFETY: readability guaranteed by the caller; `read_unaligned` tolerates
    // any field alignment the foreign struct may have.
    unsafe { base.add(offset).cast::<T>().read_unaligned() }
}

/// Read a value only if the struct's `size` declares those bytes present.
///
/// # Safety
/// Same contract as [`read_at`] once the `size` guard passes.
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
unsafe fn read_color(base: *const u8, size: usize) -> Option<NvtxColor> {
    let color_type =
        unsafe { read_present::<i32>(base, size, offset_of!(nvtxEventAttributes_v2, colorType)) }?;
    if color_type == nvtxColorType_t::NVTX_COLOR_UNKNOWN as i32 {
        return None;
    }
    let value =
        unsafe { read_present::<u32>(base, size, offset_of!(nvtxEventAttributes_v2, color)) }?;
    Some(NvtxColor { color_type, value })
}

/// Read the CORE payload union verbatim into the vocabulary payload type (D-12).
///
/// # Safety
/// See [`read_attributes`].
unsafe fn read_payload(base: *const u8, size: usize) -> Option<NvtxPayload> {
    let payload_type = unsafe {
        read_present::<i32>(base, size, offset_of!(nvtxEventAttributes_v2, payloadType))
    }?;
    if payload_type == nvtxPayloadType_t::NVTX_PAYLOAD_UNKNOWN as i32 {
        return None;
    }
    // The payload union is 8 bytes; capture its raw bits, then reinterpret per
    // the tag. On the only supported target (x86-64 LE) the 32-bit members
    // occupy the low four bytes.
    let bits =
        unsafe { read_present::<u64>(base, size, offset_of!(nvtxEventAttributes_v2, payload)) }?;
    let value = match payload_type as u32 {
        nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_UNSIGNED_INT64 => {
            NvtxPayloadValue::UnsignedInt64(bits)
        }
        nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_INT64 => NvtxPayloadValue::Int64(bits as i64),
        nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_DOUBLE => {
            NvtxPayloadValue::Double(f64::from_bits(bits))
        }
        nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_UNSIGNED_INT32 => {
            NvtxPayloadValue::UnsignedInt32(bits as u32)
        }
        nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_INT32 => NvtxPayloadValue::Int32(bits as u32 as i32),
        nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_FLOAT => {
            NvtxPayloadValue::Float(f32::from_bits(bits as u32))
        }
        // Unknown/future tag: preserve the raw bits verbatim rather than drop.
        _ => NvtxPayloadValue::UnsignedInt64(bits),
    };
    Some(NvtxPayload {
        payload_type,
        value,
    })
}

/// Read the message attribute, copying immediate strings in (Pitfall 3) and
/// keeping only the raw handle for registered strings (resolved in the analyzer).
///
/// # Safety
/// See [`read_attributes`]; the message union holds a pointer valid only for the
/// call's duration, which is copied before returning.
unsafe fn read_message(base: *const u8, size: usize) -> Option<NvtxMessage> {
    let message_type = unsafe {
        read_present::<i32>(base, size, offset_of!(nvtxEventAttributes_v2, messageType))
    }?;
    // The message union is pointer-sized; capture the raw pointer bits.
    let bits =
        unsafe { read_present::<usize>(base, size, offset_of!(nvtxEventAttributes_v2, message)) }?;
    // SAFETY: `bits` came from the caller's message union for `message_type`.
    unsafe { decode_message(message_type, bits) }
}

/// Decode a raw `(messageType, message-union bits)` pair into an owned
/// [`NvtxMessage`], copying immediate strings in (Pitfall 3) and keeping only the
/// raw handle for registered strings (resolved in the analyzer).
///
/// Shared by the event-attribute and resource-attribute readers.
///
/// # Safety
/// If `message_type` is `NVTX_MESSAGE_TYPE_ASCII`, `bits` must be a `const char*`
/// valid for this call; the bytes are copied before returning.
unsafe fn decode_message(message_type: i32, bits: usize) -> Option<NvtxMessage> {
    match message_type as u32 {
        nvtxMessageType_t::NVTX_MESSAGE_TYPE_ASCII => {
            // SAFETY: NVTX guarantees the const char* is valid for the call; the
            // bytes are copied into an owned String before returning.
            Some(NvtxMessage::String(unsafe {
                copy_cstr(bits as *const c_char)
            }))
        }
        nvtxMessageType_t::NVTX_MESSAGE_TYPE_REGISTERED => {
            Some(NvtxMessage::RegisteredHandle(bits as u64))
        }
        // UNICODE (wide-char) and any other message encoding have no Phase 1
        // vocabulary representation; they are dropped (undecoded). Surface the
        // gap once (WR-03) so an app using an unsupported/wide-char message
        // surface gets a visible signal instead of a silent drop.
        _ => {
            warn_unsupported_message_once();
            None
        }
    }
}

/// Emit a one-time, process-global diagnostic that an unsupported (e.g.
/// wide-char / Unicode) NVTX message encoding was dropped (WR-03).
///
/// Phase 1 captures only the ASCII and registered-string message surfaces; the
/// classic default-domain and wide-char APIs stay deferred. The cdylib installs
/// no tracing subscriber, so this uses `eprintln!`; an [`AtomicBool`] guard
/// fires it at most once per process to avoid spamming the hot capture path.
fn warn_unsupported_message_once() {
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "quent-nvtx: an unsupported NVTX message encoding (e.g. wide-char/Unicode) was \
             dropped; Phase 1 captures only the domain-scoped ASCII surface. This warning \
             fires once."
        );
    }
}

/// Read the captured subset of a resource-attribute struct, honoring its `size`
/// bound (Pitfall 4). Returns `(identifierType, identifier bits, optional name)`,
/// all verbatim (D-01).
///
/// # Safety
/// `attr` must be non-null and point to a valid `nvtxResourceAttributes_t` whose
/// `size` member truthfully describes the readable bytes.
unsafe fn read_resource(
    attr: *const abi::nvtxResourceAttributes_t,
) -> (i32, u64, Option<NvtxMessage>) {
    use abi::nvtxResourceAttributes_v0 as Res;

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
        Some(message_type) => {
            // SAFETY: the message union is pointer-sized; guarded by `size`.
            match unsafe { read_present::<usize>(base, size, offset_of!(Res, message)) } {
                // SAFETY: `bits` is the message union for `message_type`.
                Some(bits) => unsafe { decode_message(message_type, bits) },
                None => None,
            }
        }
        None => None,
    };

    (identifier_type, identifier, message)
}

/// Copy a caller-owned NUL-terminated C string into an owned `String`.
///
/// NVTX strings are captured with UTF-8 fidelity. Two fidelity caveats apply
/// (WR-02); both are Phase-1 limitations rather than bugs:
///
/// * A NULL pointer maps to an empty `String`, so a genuine empty string and a
///   NULL/"no text" argument are indistinguishable in the captured value. This
///   is acceptable because NVTX treats a NULL message the same as no text.
/// * Non-UTF-8 bytes (e.g. a Latin-1 domain name) are replaced with U+FFFD via
///   [`CStr::to_string_lossy`]. When that lossy replacement actually happens a
///   one-time process-global diagnostic is emitted (see [`warn_lossy_once`]) so
///   the fidelity loss is observable. True byte-verbatim capture (raw
///   `Vec<u8>`) is deferred (D-01) to avoid a breaking change to the public
///   `quent-nvtx-events` vocabulary in Phase 1.
///
/// # Safety
/// `ptr` must be null or a valid NUL-terminated C string readable for this call.
unsafe fn copy_cstr(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: validity guaranteed by the caller; the bytes are copied into an
    // owned String here so no pointer outlives the callback (Pitfall 3 UAF).
    let cstr = unsafe { CStr::from_ptr(ptr) };
    match cstr.to_str() {
        // Common case: valid UTF-8, captured byte-for-byte.
        Ok(s) => s.to_owned(),
        // Invalid UTF-8: `to_string_lossy` substitutes U+FFFD, a non-reversible
        // fidelity loss — flag it once so it is observable (WR-02 / D-01).
        Err(_) => {
            warn_lossy_once();
            cstr.to_string_lossy().into_owned()
        }
    }
}

/// Emit a one-time, process-global diagnostic that a non-UTF-8 NVTX string was
/// captured lossily (WR-02 / D-01).
///
/// The cdylib installs no tracing subscriber, so this uses `eprintln!`; an
/// [`AtomicBool`] guard fires it at most once per process to avoid flooding a
/// hot capture path.
fn warn_lossy_once() {
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "quent-nvtx: a non-UTF-8 NVTX string was captured lossily (invalid bytes replaced \
             with U+FFFD); byte-verbatim capture is deferred (D-01). This warning fires once."
        );
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use std::mem::{offset_of, size_of};

    use quent_nvtx_events::{NvtxColor, NvtxEvent, NvtxMessage, NvtxPayload, NvtxPayloadValue};

    use crate::bindings::{
        nvtxColorType_t, nvtxEventAttributes_v2, nvtxEventAttributes_v2_payload_t,
        nvtxMessageType_t, nvtxMessageValue_t, nvtxPayloadType_t, nvtxStringHandle_t,
    };

    use super::range_push;

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
        let event = unsafe { range_push(0x1234, &attr) };

        let NvtxEvent::RangePush { domain, attributes } = event else {
            panic!("expected RangePush");
        };
        // Raw handle kept verbatim.
        assert_eq!(domain, 0x1234);
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
    fn smaller_size_reads_only_declared_members_without_over_read() {
        // Claim a `size` that only covers through the `color` member. Even though
        // the backing struct has payload/message set, they sit past `size` and
        // must not be read (Pitfall 4).
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
        let event = unsafe { range_push(1, &attr) };

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
        let event = unsafe { range_push(0, &attr) };

        let NvtxEvent::RangePush { attributes, .. } = event else {
            panic!("expected RangePush");
        };
        assert_eq!(
            attributes.message,
            Some(NvtxMessage::RegisteredHandle(handle))
        );
    }

    // ---- Task 1: the remaining CORE/CORE2 kinds -------------------------------

    use super::{
        abi, domain_create, domain_destroy, mark, name_category, name_thread, range_end,
        range_start, register_string, resource_create, resource_destroy,
    };

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
        let NvtxEvent::Mark { domain, attributes } = (unsafe { mark(0x77, &attr) }) else {
            panic!("expected Mark");
        };
        assert_eq!(domain, 0x77);
        assert_eq!(
            attributes.message,
            Some(NvtxMessage::String("mark".to_owned()))
        );
        // CORE payload union preserved verbatim (D-12).
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
        let start = unsafe { range_start(0x11, range_id, &attr) };
        let NvtxEvent::RangeStart {
            domain,
            range_id: started,
            attributes,
        } = start
        else {
            panic!("expected RangeStart");
        };
        assert_eq!(domain, 0x11);
        // Verbatim id — a later RangeEnd correlates on the same handle (Phase 2).
        assert_eq!(started, range_id);
        assert_eq!(
            attributes.message,
            Some(NvtxMessage::String("proc-wide".to_owned()))
        );

        // The matching end carries the identical raw id verbatim.
        let NvtxEvent::RangeEnd {
            domain,
            range_id: ended,
        } = range_end(0x11, range_id)
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
            (unsafe { domain_create(0x5, name.as_ptr()) })
        else {
            panic!("expected DomainCreate");
        };
        assert_eq!(domain, 0x5);
        assert_eq!(got, "quent-domain");

        let NvtxEvent::DomainDestroy { domain } = domain_destroy(0x5) else {
            panic!("expected DomainDestroy");
        };
        assert_eq!(domain, 0x5);
    }

    #[test]
    fn register_string_captures_value_once_and_carries_handle() {
        let value = CString::new("registered-once").expect("cstring");
        let handle: u64 = 0x2222;
        // SAFETY: `value` is a valid NUL-terminated C string for this call.
        let event = unsafe { register_string(0x9, handle, value.as_ptr()) };
        let NvtxEvent::RegisterString {
            domain,
            handle: got_handle,
            string,
        } = event
        else {
            panic!("expected RegisterString");
        };
        assert_eq!(domain, 0x9);
        // The value is captured ONCE, at registration (Pitfall 3)...
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
        } = (unsafe { name_category(0x3, 7, cat.as_ptr()) })
        else {
            panic!("expected NameCategory");
        };
        assert_eq!(domain, 0x3);
        assert_eq!(category, 7);
        assert_eq!(name, "io");

        let thread = CString::new("worker-1").expect("cstring");
        // SAFETY: `thread` is a valid NUL-terminated C string for this call.
        let NvtxEvent::NameThread { thread_id, name } =
            (unsafe { name_thread(4242, thread.as_ptr()) })
        else {
            panic!("expected NameThread");
        };
        assert_eq!(thread_id, 4242);
        assert_eq!(name, "worker-1");
    }

    /// A zeroed v0 resource-attribute struct with `version`/`size` set full.
    fn full_resource_attr() -> abi::nvtxResourceAttributes_v0 {
        abi::nvtxResourceAttributes_v0 {
            version: 1,
            size: size_of::<abi::nvtxResourceAttributes_v0>() as u16,
            identifierType: 0,
            identifier: abi::nvtxResourceIdentifier_t { ull_value: 0 },
            messageType: nvtxMessageType_t::NVTX_MESSAGE_UNKNOWN as i32,
            message: nvtxMessageValue_t {
                ascii: std::ptr::null(),
            },
        }
    }

    #[test]
    fn resource_create_captures_identifier_and_name_verbatim() {
        let name = CString::new("cuda-stream-7").expect("cstring");
        let attr = abi::nvtxResourceAttributes_v0 {
            identifierType: 5,
            identifier: abi::nvtxResourceIdentifier_t {
                ull_value: 0x1234_5678,
            },
            messageType: nvtxMessageType_t::NVTX_MESSAGE_TYPE_ASCII as i32,
            message: nvtxMessageValue_t {
                ascii: name.as_ptr(),
            },
            ..full_resource_attr()
        };

        // SAFETY: `attr` is a valid, fully-sized resource-attribute struct.
        let event = unsafe { resource_create(0x8, 0x99, &attr) };
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

        let NvtxEvent::ResourceDestroy { handle } = resource_destroy(0x99) else {
            panic!("expected ResourceDestroy");
        };
        assert_eq!(handle, 0x99);
    }

    #[test]
    fn resource_create_honors_size_and_registered_identifier() {
        // Claim a `size` that stops before the message member; the message must
        // not be read even though the backing struct sets it (Pitfall 4).
        let name = CString::new("must-not-read").expect("cstring");
        let truncated = (offset_of!(abi::nvtxResourceAttributes_v0, messageType)) as u16;
        let attr = abi::nvtxResourceAttributes_v0 {
            size: truncated,
            identifierType: 2,
            identifier: abi::nvtxResourceIdentifier_t { ull_value: 0xABCD },
            messageType: nvtxMessageType_t::NVTX_MESSAGE_TYPE_ASCII as i32,
            message: nvtxMessageValue_t {
                ascii: name.as_ptr(),
            },
            ..full_resource_attr()
        };

        // SAFETY: reads are bounded by `attr.size`; the backing allocation is a
        // full struct so an over-read would still be in-bounds — the assertions
        // prove we honor `size` regardless.
        let event = unsafe { resource_create(0x1, 0x2, &attr) };
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
}
