// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The `extern "C"` NVTX callbacks installed into the CORE/CORE2 function tables.
//!
//! Each callback does the minimum on the app thread — copy caller-owned data
//! into a [`Record`](crate::Record) and hand it to the installed hook.
//! The callbacks preserve two invariants:
//!
//! * Handles, ids, and nesting levels are synthesized whether or not a hook is
//!   installed, so values the app caches before installation stay valid.
//! * No locking or serialization happens here beyond the message copy-in
//!   required for safety. Hooks must not call NVTX APIs, and an
//!   uncaught Rust panic aborts at the C ABI boundary.

use std::os::raw::{c_char, c_int};

use crate::{init, record};
use nvtx_sys::ffi::{
    nvtxDomainHandle_t, nvtxEventAttributes_t, nvtxRangeId_t, nvtxResourceAttributes_t,
    nvtxResourceHandle_t, nvtxStringHandle_t, wchar_t,
};

/// CORE2 `DomainRangePushEx` subscriber.
///
/// Returns the 0-based nesting level of the range being started (NVTX's
/// `nvtxDomainRangePushEx` return value), even when no hook is installed.
pub(crate) extern "C" fn on_domain_range_push_ex(
    domain: nvtxDomainHandle_t,
    attr: *const nvtxEventAttributes_t,
) -> c_int {
    let domain = domain as usize as u64;
    let level = init::range_push_level(domain);
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { record::range_push(domain, attr) };
        hook(event);
    }
    level
}

/// CORE2 `DomainRangePop` subscriber.
///
/// Returns the 0-based nesting level of the range being ended (NVTX's
/// `nvtxDomainRangePop` return value).
pub(crate) extern "C" fn on_domain_range_pop(domain: nvtxDomainHandle_t) -> c_int {
    let domain = domain as usize as u64;
    let level = init::range_pop_level(domain);
    if let Some(hook) = init::hook() {
        hook(record::range_pop(domain));
    }
    level
}

/// CORE2 `DomainMarkEx` subscriber (instantaneous marker).
pub(crate) extern "C" fn on_domain_mark_ex(
    domain: nvtxDomainHandle_t,
    attr: *const nvtxEventAttributes_t,
) {
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { record::mark(domain as usize as u64, attr) };
        hook(event);
    }
}

/// CORE2 `DomainRangeStartEx` subscriber.
///
/// Synthesizes and returns a process-unique range id even without a hook.
/// The id is captured verbatim so a later `DomainRangeEnd` correlates.
pub(crate) extern "C" fn on_domain_range_start_ex(
    domain: nvtxDomainHandle_t,
    attr: *const nvtxEventAttributes_t,
) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { record::range_start(domain as usize as u64, range_id, attr) };
        hook(event);
    }
    range_id
}

/// CORE2 `DomainRangeEnd` subscriber.
pub(crate) extern "C" fn on_domain_range_end(domain: nvtxDomainHandle_t, range_id: nvtxRangeId_t) {
    if let Some(hook) = init::hook() {
        hook(record::range_end(domain as usize as u64, range_id));
    }
}

/// CORE2 `DomainCreateA` subscriber. Synthesizes and RETURNS the domain handle.
pub(crate) extern "C" fn on_domain_create_a(name: *const c_char) -> nvtxDomainHandle_t {
    let handle = init::next_handle();
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call; it
        // is copied into an owned buffer inside `record::domain_create`.
        let event = unsafe { record::domain_create(handle, name) };
        hook(event);
    }
    handle as usize as nvtxDomainHandle_t
}

/// CORE2 `DomainDestroy` subscriber.
pub(crate) extern "C" fn on_domain_destroy(domain: nvtxDomainHandle_t) {
    if let Some(hook) = init::hook() {
        hook(record::domain_destroy(domain as usize as u64));
    }
}

/// CORE2 `DomainRegisterStringA` subscriber. Synthesizes and RETURNS the string
/// handle; the string value is captured ONCE here at registration.
pub(crate) extern "C" fn on_domain_register_string_a(
    domain: nvtxDomainHandle_t,
    string: *const c_char,
) -> nvtxStringHandle_t {
    let handle = init::next_handle();
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `string` (if non-null) is valid for this call;
        // it is copied into an owned buffer inside `record::register_string`.
        let event = unsafe { record::register_string(domain as usize as u64, handle, string) };
        hook(event);
    }
    handle as usize as nvtxStringHandle_t
}

/// CORE2 `DomainNameCategoryA` subscriber.
pub(crate) extern "C" fn on_domain_name_category_a(
    domain: nvtxDomainHandle_t,
    category: u32,
    name: *const c_char,
) {
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call.
        let event = unsafe { record::name_category(domain as usize as u64, category, name) };
        hook(event);
    }
}

/// CORE `NameOsThreadA` subscriber (non-domain thread naming).
pub(crate) extern "C" fn on_name_os_thread_a(thread_id: u32, name: *const c_char) {
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call.
        let event = unsafe { record::name_thread(thread_id, name) };
        hook(event);
    }
}

/// CORE2 `DomainResourceCreate` subscriber. Synthesizes and RETURNS the resource
/// handle.
pub(crate) extern "C" fn on_domain_resource_create(
    domain: nvtxDomainHandle_t,
    attr: *mut nvtxResourceAttributes_t,
) -> nvtxResourceHandle_t {
    let handle = init::next_handle();
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { record::resource_create(domain as usize as u64, handle, attr) };
        hook(event);
    }
    handle as usize as nvtxResourceHandle_t
}

/// CORE2 `DomainResourceDestroy` subscriber.
pub(crate) extern "C" fn on_domain_resource_destroy(resource: nvtxResourceHandle_t) {
    if let Some(hook) = init::hook() {
        hook(record::resource_destroy(resource as usize as u64));
    }
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
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { record::mark(0, attr) };
        hook(event);
    }
}

/// CORE `MarkA` subscriber (default-domain marker with an immediate string).
pub(crate) extern "C" fn on_mark_a(message: *const c_char) {
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `message` (if non-null) is valid for this call.
        let event = unsafe { record::mark_a(message) };
        hook(event);
    }
}

/// CORE `RangeStartEx` subscriber. Synthesizes and RETURNS a process-unique id.
pub(crate) extern "C" fn on_range_start_ex(attr: *const nvtxEventAttributes_t) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { record::range_start(0, range_id, attr) };
        hook(event);
    }
    range_id
}

/// CORE `RangeStartA` subscriber (immediate string). Synthesizes/RETURNS an id.
pub(crate) extern "C" fn on_range_start_a(message: *const c_char) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `message` (if non-null) is valid for this call.
        let event = unsafe { record::range_start_a(range_id, message) };
        hook(event);
    }
    range_id
}

/// CORE `RangeEnd` subscriber (default domain).
pub(crate) extern "C" fn on_range_end(range_id: nvtxRangeId_t) {
    if let Some(hook) = init::hook() {
        hook(record::range_end(0, range_id));
    }
}

/// CORE `RangePushEx` subscriber. Returns the 0-based default-domain nesting
/// level of the range being started.
pub(crate) extern "C" fn on_range_push_ex(attr: *const nvtxEventAttributes_t) -> c_int {
    let level = init::range_push_level(0);
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { record::range_push(0, attr) };
        hook(event);
    }
    level
}

/// CORE `RangePushA` subscriber (immediate string). Returns the nesting level.
pub(crate) extern "C" fn on_range_push_a(message: *const c_char) -> c_int {
    let level = init::range_push_level(0);
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `message` (if non-null) is valid for this call.
        let event = unsafe { record::range_push_a(message) };
        hook(event);
    }
    level
}

/// CORE `RangePop` subscriber (default domain). Returns the level ended.
pub(crate) extern "C" fn on_range_pop() -> c_int {
    let level = init::range_pop_level(0);
    if let Some(hook) = init::hook() {
        hook(record::range_pop(0));
    }
    level
}

/// CORE `NameCategoryA` subscriber (default-domain category naming).
pub(crate) extern "C" fn on_name_category_a(category: u32, name: *const c_char) {
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call.
        let event = unsafe { record::name_category(0, category, name) };
        hook(event);
    }
}

// ---- Wide-char (Unicode) CORE callbacks -----------------------------------
//
// Wide strings are copied as code units. The consumer decodes them after the
// callback returns.

/// CORE `MarkW` subscriber — wide-char instantaneous marker on the default domain.
pub(crate) extern "C" fn on_mark_w(message: *const wchar_t) {
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `message` is null or a valid NUL-terminated
        // wchar_t array for this call; copy_wchar copies before returning.
        let event = unsafe { record::mark_w(message) };
        hook(event);
    }
}

/// CORE `RangeStartW` subscriber — synthesizes and RETURNS a process-unique id,
/// then captures the wide-char label as owned code units.
pub(crate) extern "C" fn on_range_start_w(message: *const wchar_t) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `message` is null or a valid NUL-terminated
        // wchar_t array for this call.
        let event = unsafe { record::range_start_w(range_id, message) };
        hook(event);
    }
    range_id
}

/// CORE `RangePushW` subscriber — returns the 0-based default-domain nesting
/// level of the range being started, capturing owned wide-char code units.
pub(crate) extern "C" fn on_range_push_w(message: *const wchar_t) -> c_int {
    let level = init::range_push_level(0);
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `message` is null or a valid NUL-terminated
        // wchar_t array for this call.
        let event = unsafe { record::range_push_w(message) };
        hook(event);
    }
    level
}

/// CORE `NameCategoryW` subscriber — wide-char category name on the default domain.
pub(crate) extern "C" fn on_name_category_w(category: u32, name: *const wchar_t) {
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `name` is null or a valid NUL-terminated
        // wchar_t array for this call.
        let name = unsafe { record::copy_wchar(name) };
        hook(crate::Record::NameCategory {
            domain: 0,
            category,
            name,
        });
    }
}

/// CORE `NameOsThreadW` subscriber — wide-char thread name.
pub(crate) extern "C" fn on_name_os_thread_w(thread_id: u32, name: *const wchar_t) {
    if let Some(hook) = init::hook() {
        // SAFETY: NVTX guarantees `name` is null or a valid NUL-terminated
        // wchar_t array for this call.
        let name = unsafe { record::copy_wchar(name) };
        hook(crate::Record::NameThread { thread_id, name });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    use crate::Record;

    use super::*;

    /// Exercise every subscribed CORE/CORE2 callback, including all return kinds.
    /// Return values must not depend on whether a hook is installed.
    fn exercise_callbacks() {
        let name = c"capture".as_ptr();
        let wide = [b'w' as wchar_t, 0];
        let attr = std::ptr::null();

        let domain = on_domain_create_a(name);
        assert!(!domain.is_null());
        assert!(!on_domain_register_string_a(domain, name).is_null());
        on_domain_name_category_a(domain, 1, name);
        on_domain_mark_ex(domain, attr);
        let range = on_domain_range_start_ex(domain, attr);
        assert_ne!(range, 0);
        on_domain_range_end(domain, range);
        assert_eq!(on_domain_range_push_ex(domain, attr), 0);
        assert_eq!(on_domain_range_pop(domain), 0);
        let resource = on_domain_resource_create(domain, std::ptr::null_mut());
        assert!(!resource.is_null());
        on_domain_resource_destroy(resource);
        on_domain_destroy(domain);

        on_mark_ex(attr);
        on_mark_a(name);
        on_mark_w(wide.as_ptr());
        let ranges = [
            on_range_start_ex(attr),
            on_range_start_a(name),
            on_range_start_w(wide.as_ptr()),
        ];
        for range in ranges {
            assert_ne!(range, 0);
            on_range_end(range);
        }
        assert_eq!(on_range_push_ex(attr), 0);
        assert_eq!(on_range_push_a(name), 1);
        assert_eq!(on_range_push_w(wide.as_ptr()), 2);
        assert_eq!(on_range_pop(), 2);
        assert_eq!(on_range_pop(), 1);
        assert_eq!(on_range_pop(), 0);
        on_name_category_a(1, name);
        on_name_category_w(1, wide.as_ptr());
        on_name_os_thread_a(1, name);
        on_name_os_thread_w(1, wide.as_ptr());
    }

    // One test owns the process-global one-shot hook for this test binary.
    #[test]
    fn callbacks_use_permanent_one_shot_hook() {
        exercise_callbacks();
        // A domain handle and an open range created before install must stay
        // valid once capture starts: apps cache handles, and nesting levels
        // count ranges opened before the hook.
        let early_domain = on_domain_create_a(c"early".as_ptr());
        assert!(!early_domain.is_null());
        assert_eq!(on_range_push_a(c"outer".as_ptr()), 0);

        let calls = Arc::new(AtomicUsize::new(0));
        let last_mark_domain = Arc::new(AtomicU64::new(0));
        init::install_hook({
            let calls = Arc::clone(&calls);
            let last_mark_domain = Arc::clone(&last_mark_domain);
            move |event| {
                calls.fetch_add(1, Ordering::Relaxed);
                if let Record::Mark { domain, .. } = event {
                    last_mark_domain.store(domain, Ordering::Relaxed);
                }
            }
        })
        .unwrap();

        on_domain_mark_ex(early_domain, std::ptr::null());
        assert_eq!(
            last_mark_domain.load(Ordering::Relaxed),
            early_domain as usize as u64
        );
        assert_eq!(on_range_push_a(c"inner".as_ptr()), 1);
        assert_eq!(on_range_pop(), 1);
        assert_eq!(on_range_pop(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 4);

        exercise_callbacks();
        assert_eq!(calls.load(Ordering::Relaxed), 34);
        assert!(matches!(
            init::install_hook(|_| unreachable!()),
            Err(init::InstallHookError::AlreadyInstalled)
        ));
        on_mark_a(c"still capturing".as_ptr());
        assert_eq!(calls.load(Ordering::Relaxed), 35);

        exercise_callbacks();
        assert_eq!(calls.load(Ordering::Relaxed), 65);
        assert!(matches!(
            init::install_hook(|_| unreachable!()),
            Err(init::InstallHookError::AlreadyInstalled)
        ));
    }
}
