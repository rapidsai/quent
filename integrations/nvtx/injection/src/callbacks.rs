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

use std::os::raw::{c_char, c_int};
use std::panic::AssertUnwindSafe;

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
