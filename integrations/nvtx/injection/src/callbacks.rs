// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The `extern "C"` NVTX callbacks installed into the CORE2 function table.
//!
//! Each callback does the minimum on the app thread — convert to a verbatim
//! [`NvtxEvent`] and hand it to the installed hook — with two invariants:
//!
//! * The entire body is wrapped in [`std::panic::catch_unwind`]; a Rust panic
//!   must never unwind into NVTX's C caller (UB → app crash).
//! * No allocation-heavy work, locking, or serialization happens here beyond the
//!   message copy-in required for safety; serialization lives on the
//!   downstream drain thread in the bridge.

use std::os::raw::{c_char, c_int, c_void};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::bindings::{
    nvtxDomainHandle_t, nvtxEventAttributes_t, nvtxRangeId_t, nvtxResourceAttributes_t,
    nvtxResourceHandle_t, nvtxStringHandle_t,
};
use crate::{convert, init};

/// CORE2 `DomainRangePushEx` subscriber.
///
/// Returns the 0-based nesting level of the range being started (NVTX's
/// `nvtxDomainRangePushEx` return value). The level is computed inside the
/// unwind guard so a conversion panic cannot leak it, yet still survives to the
/// return because it is written before any fallible work. A panic must never
/// cross the C ABI boundary.
pub(crate) extern "C" fn on_domain_range_push_ex(
    domain: nvtxDomainHandle_t,
    attr: *const nvtxEventAttributes_t,
) -> c_int {
    let domain = domain as usize as u64;
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        level = init::range_push_level(domain);
        if attr.is_null() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is valid for the duration of this call.
        let event = unsafe { convert::range_push(domain, attr) };
        init::dispatch(event);
    }));
    level
}

/// CORE2 `DomainRangePop` subscriber.
///
/// Returns the 0-based nesting level of the range being ended (NVTX's
/// `nvtxDomainRangePop` return value).
pub(crate) extern "C" fn on_domain_range_pop(domain: nvtxDomainHandle_t) -> c_int {
    let domain = domain as usize as u64;
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        level = init::range_pop_level(domain);
        init::dispatch(convert::range_pop(domain));
    }));
    level
}

/// CORE2 `DomainMarkEx` subscriber (instantaneous marker).
pub(crate) extern "C" fn on_domain_mark_ex(
    domain: nvtxDomainHandle_t,
    attr: *const nvtxEventAttributes_t,
) {
    let _ = std::panic::catch_unwind(|| {
        if attr.is_null() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is valid for the duration of this call.
        let event = unsafe { convert::mark(domain as usize as u64, attr) };
        init::dispatch(event);
    });
}

/// CORE2 `DomainRangeStartEx` subscriber.
///
/// Synthesizes and RETURNS a process-unique range id (the id NVTX hands back to
/// the caller). It is generated outside `catch_unwind` so the correct id is
/// returned even if conversion panics, and captured verbatim so a later
/// `DomainRangeEnd` correlates process-wide.
pub(crate) extern "C" fn on_domain_range_start_ex(
    domain: nvtxDomainHandle_t,
    attr: *const nvtxEventAttributes_t,
) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        if attr.is_null() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is valid for the duration of this call.
        let event = unsafe { convert::range_start(domain as usize as u64, range_id, attr) };
        init::dispatch(event);
    });
    range_id
}

/// CORE2 `DomainRangeEnd` subscriber.
pub(crate) extern "C" fn on_domain_range_end(domain: nvtxDomainHandle_t, range_id: nvtxRangeId_t) {
    let _ = std::panic::catch_unwind(|| {
        init::dispatch(convert::range_end(domain as usize as u64, range_id));
    });
}

/// CORE2 `DomainCreateA` subscriber. Synthesizes and RETURNS the domain handle.
pub(crate) extern "C" fn on_domain_create_a(name: *const c_char) -> nvtxDomainHandle_t {
    let handle = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call; it
        // is copied into an owned String inside `convert::domain_create`.
        let event = unsafe { convert::domain_create(handle, name) };
        init::dispatch(event);
    });
    handle as usize as nvtxDomainHandle_t
}

/// CORE2 `DomainDestroy` subscriber.
pub(crate) extern "C" fn on_domain_destroy(domain: nvtxDomainHandle_t) {
    let _ = std::panic::catch_unwind(|| {
        init::dispatch(convert::domain_destroy(domain as usize as u64));
    });
}

/// CORE2 `DomainRegisterStringA` subscriber. Synthesizes and RETURNS the string
/// handle; the string value is captured ONCE here at registration.
pub(crate) extern "C" fn on_domain_register_string_a(
    domain: nvtxDomainHandle_t,
    string: *const c_char,
) -> nvtxStringHandle_t {
    let handle = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        // SAFETY: NVTX guarantees `string` (if non-null) is valid for this call;
        // it is copied into an owned String inside `convert::register_string`.
        let event = unsafe { convert::register_string(domain as usize as u64, handle, string) };
        init::dispatch(event);
    });
    handle as usize as nvtxStringHandle_t
}

/// CORE2 `DomainNameCategoryA` subscriber.
pub(crate) extern "C" fn on_domain_name_category_a(
    domain: nvtxDomainHandle_t,
    category: u32,
    name: *const c_char,
) {
    let _ = std::panic::catch_unwind(|| {
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call.
        let event = unsafe { convert::name_category(domain as usize as u64, category, name) };
        init::dispatch(event);
    });
}

/// CORE `NameOsThreadA` subscriber (non-domain thread naming).
pub(crate) extern "C" fn on_name_os_thread_a(thread_id: u32, name: *const c_char) {
    let _ = std::panic::catch_unwind(|| {
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call.
        let event = unsafe { convert::name_thread(thread_id, name) };
        init::dispatch(event);
    });
}

/// CORE2 `DomainResourceCreate` subscriber. Synthesizes and RETURNS the resource
/// handle.
pub(crate) extern "C" fn on_domain_resource_create(
    domain: nvtxDomainHandle_t,
    attr: *const nvtxResourceAttributes_t,
) -> nvtxResourceHandle_t {
    let handle = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        if attr.is_null() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is valid for the duration of this call.
        let event = unsafe { convert::resource_create(domain as usize as u64, handle, attr) };
        init::dispatch(event);
    });
    handle as usize as nvtxResourceHandle_t
}

/// CORE2 `DomainResourceDestroy` subscriber.
pub(crate) extern "C" fn on_domain_resource_destroy(resource: nvtxResourceHandle_t) {
    let _ = std::panic::catch_unwind(|| {
        init::dispatch(convert::resource_destroy(resource as usize as u64));
    });
}

// ---- Default-domain (CORE) callbacks --------------------------------------
//
// The classic NVTX API (`nvtxMarkA`, `nvtxRangePushA`, `nvtxRangePop`, …) is not
// domain-scoped; NVTX dispatches it through the CORE table rather than the
// domain-scoped CORE2 table. We capture it verbatim on the default domain
// (`0`). Range nesting levels and start/end ids are synthesized exactly as for
// the domain surface, keyed by domain `0`, so an app that reads NVTX's return
// values still observes faithful behavior.

/// CORE `MarkEx` subscriber (default-domain instantaneous marker).
pub(crate) extern "C" fn on_mark_ex(attr: *const nvtxEventAttributes_t) {
    let _ = std::panic::catch_unwind(|| {
        if attr.is_null() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is valid for the duration of this call.
        let event = unsafe { convert::mark(0, attr) };
        init::dispatch(event);
    });
}

/// CORE `MarkA` subscriber (default-domain marker with an immediate string).
pub(crate) extern "C" fn on_mark_a(message: *const c_char) {
    let _ = std::panic::catch_unwind(|| {
        // SAFETY: NVTX guarantees `message` (if non-null) is valid for this call.
        let event = unsafe { convert::mark_a(message) };
        init::dispatch(event);
    });
}

/// CORE `RangeStartEx` subscriber. Synthesizes and RETURNS a process-unique id.
pub(crate) extern "C" fn on_range_start_ex(attr: *const nvtxEventAttributes_t) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        if attr.is_null() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is valid for the duration of this call.
        let event = unsafe { convert::range_start(0, range_id, attr) };
        init::dispatch(event);
    });
    range_id
}

/// CORE `RangeStartA` subscriber (immediate string). Synthesizes/RETURNS an id.
pub(crate) extern "C" fn on_range_start_a(message: *const c_char) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        // SAFETY: NVTX guarantees `message` (if non-null) is valid for this call.
        let event = unsafe { convert::range_start_a(range_id, message) };
        init::dispatch(event);
    });
    range_id
}

/// CORE `RangeEnd` subscriber (default domain).
pub(crate) extern "C" fn on_range_end(range_id: nvtxRangeId_t) {
    let _ = std::panic::catch_unwind(|| {
        init::dispatch(convert::range_end(0, range_id));
    });
}

/// CORE `RangePushEx` subscriber. Returns the 0-based default-domain nesting
/// level of the range being started.
pub(crate) extern "C" fn on_range_push_ex(attr: *const nvtxEventAttributes_t) -> c_int {
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        level = init::range_push_level(0);
        if attr.is_null() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is valid for the duration of this call.
        let event = unsafe { convert::range_push(0, attr) };
        init::dispatch(event);
    }));
    level
}

/// CORE `RangePushA` subscriber (immediate string). Returns the nesting level.
pub(crate) extern "C" fn on_range_push_a(message: *const c_char) -> c_int {
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        level = init::range_push_level(0);
        // SAFETY: NVTX guarantees `message` (if non-null) is valid for this call.
        let event = unsafe { convert::range_push_a(message) };
        init::dispatch(event);
    }));
    level
}

/// CORE `RangePop` subscriber (default domain). Returns the level ended.
pub(crate) extern "C" fn on_range_pop() -> c_int {
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        level = init::range_pop_level(0);
        init::dispatch(convert::range_pop(0));
    }));
    level
}

/// CORE `NameCategoryA` subscriber (default-domain category naming).
pub(crate) extern "C" fn on_name_category_a(category: u32, name: *const c_char) {
    let _ = std::panic::catch_unwind(|| {
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call.
        let event = unsafe { convert::name_category(0, category, name) };
        init::dispatch(event);
    });
}

// ---- Wide-char (Unicode) CORE stubs ---------------------------------------
//
// The `*W` variants carry UTF-16 strings, which have no vocabulary
// representation yet. Rather than leave them unsubscribed (NVTX would then null
// them into silent no-ops), we subscribe stubs that warn once and preserve
// range nesting/ids, so an app mixing ASCII and wide-char calls keeps balanced
// ranges and valid return values while the wide labels are dropped.

/// Emit a one-time, process-global diagnostic that a wide-char (`*W`) NVTX call
/// was seen but not captured. Fires at most once to avoid spamming the hot path.
fn warn_wide_surface_once() {
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "quent-nvtx: a wide-char (Unicode) NVTX call was seen but not captured; only the \
             ASCII surface is decoded. This warning fires once."
        );
    }
}

/// CORE `MarkW` stub — wide-char marker, dropped with a one-time warning.
pub(crate) extern "C" fn on_mark_w(_message: *const c_void) {
    let _ = std::panic::catch_unwind(warn_wide_surface_once);
}

/// CORE `RangeStartW` stub — synthesizes/RETURNS an id so a later `RangeEnd`
/// stays valid; the wide label is dropped and warned once.
pub(crate) extern "C" fn on_range_start_w(_message: *const c_void) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    let _ = std::panic::catch_unwind(warn_wide_surface_once);
    range_id
}

/// CORE `RangePushW` stub — preserves default-domain nesting; label dropped,
/// warned once.
pub(crate) extern "C" fn on_range_push_w(_message: *const c_void) -> c_int {
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        warn_wide_surface_once();
        level = init::range_push_level(0);
    }));
    level
}

/// CORE `NameCategoryW` stub — wide-char category name, dropped with a warning.
pub(crate) extern "C" fn on_name_category_w(_category: u32, _name: *const c_void) {
    let _ = std::panic::catch_unwind(warn_wide_surface_once);
}

/// CORE `NameOsThreadW` stub — wide-char thread name, dropped with a warning.
pub(crate) extern "C" fn on_name_os_thread_w(_thread_id: u32, _name: *const c_void) {
    let _ = std::panic::catch_unwind(warn_wide_surface_once);
}
