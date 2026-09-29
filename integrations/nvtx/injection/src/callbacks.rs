// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The `extern "C"` NVTX callbacks installed into the CORE/CORE2 function tables.
//!
//! Each callback does the minimum on the app thread — convert to a verbatim
//! [`NvtxEvent`] and hand it to the installed hook — with three invariants:
//!
//! * Handles, ids, and nesting levels are synthesized whether or not capture is
//!   active, so values the app caches before install or after shutdown stay
//!   valid; event conversion and dispatch are gated (see [`init::CaptureGuard`]).
//! * Fallible work is wrapped in [`std::panic::catch_unwind`]; a Rust panic
//!   must never unwind into NVTX's C caller (UB → app crash).
//! * No allocation-heavy work, locking, or serialization happens here beyond the
//!   message copy-in required for safety; serialization lives on the
//!   downstream drain thread in the bridge.

use std::os::raw::{c_char, c_int};
use std::panic::AssertUnwindSafe;

use crate::{convert, init};
use nvtx_sys::ffi::{
    nvtxDomainHandle_t, nvtxEventAttributes_t, nvtxRangeId_t, nvtxResourceAttributes_t,
    nvtxResourceHandle_t, nvtxStringHandle_t, wchar_t,
};

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
        if !init::capture_active() {
            return;
        }
        // The OS thread id is read on the app thread so the push pairs with its
        // pop on the same thread; only used to build the event, so reading it
        // inside the guard is enough (it need not survive a panic).
        let thread_id = init::current_thread_id();
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { convert::range_push(domain, attr, thread_id) };
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
        if !init::capture_active() {
            return;
        }
        let thread_id = init::current_thread_id();
        init::dispatch(convert::range_pop(domain, thread_id));
    }));
    level
}

/// CORE2 `DomainMarkEx` subscriber (instantaneous marker).
pub(crate) extern "C" fn on_domain_mark_ex(
    domain: nvtxDomainHandle_t,
    attr: *const nvtxEventAttributes_t,
) {
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
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
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { convert::range_start(domain as usize as u64, range_id, attr) };
        init::dispatch(event);
    });
    range_id
}

/// CORE2 `DomainRangeEnd` subscriber.
pub(crate) extern "C" fn on_domain_range_end(domain: nvtxDomainHandle_t, range_id: nvtxRangeId_t) {
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        init::dispatch(convert::range_end(domain as usize as u64, range_id));
    });
}

/// CORE2 `DomainCreateA` subscriber. Synthesizes and RETURNS the domain handle.
pub(crate) extern "C" fn on_domain_create_a(name: *const c_char) -> nvtxDomainHandle_t {
    let handle = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
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
        if !init::capture_active() {
            return;
        }
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
        if !init::capture_active() {
            return;
        }
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
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call.
        let event = unsafe { convert::name_category(domain as usize as u64, category, name) };
        init::dispatch(event);
    });
}

/// CORE `NameOsThreadA` subscriber (non-domain thread naming).
pub(crate) extern "C" fn on_name_os_thread_a(thread_id: u32, name: *const c_char) {
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call.
        let event = unsafe { convert::name_thread(thread_id, name) };
        init::dispatch(event);
    });
}

/// CORE2 `DomainResourceCreate` subscriber. Synthesizes and RETURNS the resource
/// handle.
pub(crate) extern "C" fn on_domain_resource_create(
    domain: nvtxDomainHandle_t,
    attr: *mut nvtxResourceAttributes_t,
) -> nvtxResourceHandle_t {
    let handle = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { convert::resource_create(domain as usize as u64, handle, attr) };
        init::dispatch(event);
    });
    handle as usize as nvtxResourceHandle_t
}

/// CORE2 `DomainResourceDestroy` subscriber.
pub(crate) extern "C" fn on_domain_resource_destroy(resource: nvtxResourceHandle_t) {
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
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
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { convert::mark(0, attr) };
        init::dispatch(event);
    });
}

/// CORE `MarkA` subscriber (default-domain marker with an immediate string).
pub(crate) extern "C" fn on_mark_a(message: *const c_char) {
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `message` (if non-null) is valid for this call.
        let event = unsafe { convert::mark_a(message) };
        init::dispatch(event);
    });
}

/// CORE `RangeStartEx` subscriber. Synthesizes and RETURNS a process-unique id.
pub(crate) extern "C" fn on_range_start_ex(attr: *const nvtxEventAttributes_t) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { convert::range_start(0, range_id, attr) };
        init::dispatch(event);
    });
    range_id
}

/// CORE `RangeStartA` subscriber (immediate string). Synthesizes/RETURNS an id.
pub(crate) extern "C" fn on_range_start_a(message: *const c_char) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `message` (if non-null) is valid for this call.
        let event = unsafe { convert::range_start_a(range_id, message) };
        init::dispatch(event);
    });
    range_id
}

/// CORE `RangeEnd` subscriber (default domain).
pub(crate) extern "C" fn on_range_end(range_id: nvtxRangeId_t) {
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        init::dispatch(convert::range_end(0, range_id));
    });
}

/// CORE `RangePushEx` subscriber. Returns the 0-based default-domain nesting
/// level of the range being started.
pub(crate) extern "C" fn on_range_push_ex(attr: *const nvtxEventAttributes_t) -> c_int {
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        level = init::range_push_level(0);
        if !init::capture_active() {
            return;
        }
        let thread_id = init::current_thread_id();
        // SAFETY: NVTX guarantees `attr` is null or valid for this call; a null
        // attr yields empty attributes (the event is still captured).
        let event = unsafe { convert::range_push(0, attr, thread_id) };
        init::dispatch(event);
    }));
    level
}

/// CORE `RangePushA` subscriber (immediate string). Returns the nesting level.
pub(crate) extern "C" fn on_range_push_a(message: *const c_char) -> c_int {
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        level = init::range_push_level(0);
        if !init::capture_active() {
            return;
        }
        let thread_id = init::current_thread_id();
        // SAFETY: NVTX guarantees `message` (if non-null) is valid for this call.
        let event = unsafe { convert::range_push_a(message, thread_id) };
        init::dispatch(event);
    }));
    level
}

/// CORE `RangePop` subscriber (default domain). Returns the level ended.
pub(crate) extern "C" fn on_range_pop() -> c_int {
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        level = init::range_pop_level(0);
        if !init::capture_active() {
            return;
        }
        let thread_id = init::current_thread_id();
        init::dispatch(convert::range_pop(0, thread_id));
    }));
    level
}

/// CORE `NameCategoryA` subscriber (default-domain category naming).
pub(crate) extern "C" fn on_name_category_a(category: u32, name: *const c_char) {
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `name` (if non-null) is valid for this call.
        let event = unsafe { convert::name_category(0, category, name) };
        init::dispatch(event);
    });
}

// ---- Wide-char (Unicode) CORE callbacks -----------------------------------
//
// On Linux `wchar_t` is 32-bit (UTF-32), so each code unit is a Unicode scalar
// value. `convert::copy_wchar` iterates the NUL-terminated sequence and builds
// an owned UTF-8 `String`; the result enters the event stream as a plain
// `NvtxMessage::String`, identical to the ASCII surface. No vocabulary changes
// are needed downstream.

/// CORE `MarkW` subscriber — wide-char instantaneous marker on the default domain.
pub(crate) extern "C" fn on_mark_w(message: *const wchar_t) {
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `message` is null or a valid NUL-terminated
        // wchar_t array for this call; copy_wchar copies before returning.
        let event = unsafe { convert::mark_w(message) };
        init::dispatch(event);
    });
}

/// CORE `RangeStartW` subscriber — synthesizes and RETURNS a process-unique id,
/// then captures the wide-char label converted to UTF-8.
pub(crate) extern "C" fn on_range_start_w(message: *const wchar_t) -> nvtxRangeId_t {
    let range_id = init::next_handle();
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `message` is null or a valid NUL-terminated
        // wchar_t array for this call.
        let event = unsafe { convert::range_start_w(range_id, message) };
        init::dispatch(event);
    });
    range_id
}

/// CORE `RangePushW` subscriber — returns the 0-based default-domain nesting
/// level of the range being started, capturing the wide-char label as UTF-8.
pub(crate) extern "C" fn on_range_push_w(message: *const wchar_t) -> c_int {
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        level = init::range_push_level(0);
        if !init::capture_active() {
            return;
        }
        let thread_id = init::current_thread_id();
        // SAFETY: NVTX guarantees `message` is null or a valid NUL-terminated
        // wchar_t array for this call.
        let event = unsafe { convert::range_push_w(message, thread_id) };
        init::dispatch(event);
    }));
    level
}

/// CORE `NameCategoryW` subscriber — wide-char category name on the default domain.
pub(crate) extern "C" fn on_name_category_w(category: u32, name: *const wchar_t) {
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `name` is null or a valid NUL-terminated
        // wchar_t array for this call.
        let name = unsafe { convert::copy_wchar_pub(name) };
        init::dispatch(nvtx_events::NvtxEvent::NameCategory {
            domain: 0,
            category,
            name,
        });
    });
}

/// CORE `NameOsThreadW` subscriber — wide-char thread name.
pub(crate) extern "C" fn on_name_os_thread_w(thread_id: u32, name: *const wchar_t) {
    let _ = std::panic::catch_unwind(|| {
        if !init::capture_active() {
            return;
        }
        // SAFETY: NVTX guarantees `name` is null or a valid NUL-terminated
        // wchar_t array for this call.
        let name = unsafe { convert::copy_wchar_pub(name) };
        init::dispatch(nvtx_events::NvtxEvent::NameThread { thread_id, name });
    });
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
    use std::sync::{Arc, mpsc};
    use std::time::Duration;

    use nvtx_events::NvtxEvent;

    use super::*;

    /// Exercise every subscribed CORE/CORE2 callback, including all return kinds.
    /// Return values must not depend on whether capture is active.
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
    fn callbacks_capture_only_between_install_and_disable() {
        exercise_callbacks();
        // A domain handle and an open range created before install must stay
        // valid once capture starts: apps cache handles, and nesting levels
        // count ranges opened before the hook.
        let early_domain = on_domain_create_a(c"early".as_ptr());
        assert!(!early_domain.is_null());
        assert_eq!(on_range_push_a(c"outer".as_ptr()), 0);

        let calls = Arc::new(AtomicUsize::new(0));
        let last_mark_domain = Arc::new(AtomicU64::new(0));
        let panic_next = Arc::new(AtomicBool::new(false));
        let block_next = Arc::new(AtomicBool::new(false));
        let entered = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let guard = init::install_hook({
            let calls = Arc::clone(&calls);
            let last_mark_domain = Arc::clone(&last_mark_domain);
            let panic_next = Arc::clone(&panic_next);
            let block_next = Arc::clone(&block_next);
            let entered = Arc::clone(&entered);
            let release = Arc::clone(&release);
            move |event| {
                calls.fetch_add(1, Ordering::Relaxed);
                if let NvtxEvent::Mark { domain, .. } = event {
                    last_mark_domain.store(domain, Ordering::Relaxed);
                }
                // Hook-induced NVTX must still be dropped by the reentry guard.
                on_mark_a(c"nested".as_ptr());
                assert!(!panic_next.swap(false, Ordering::Relaxed), "hook panic");
                if block_next.swap(false, Ordering::Relaxed) {
                    entered.store(true, Ordering::Release);
                    while !release.load(Ordering::Acquire) {
                        std::thread::yield_now();
                    }
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
        assert!(init::install_hook(|_| unreachable!()).is_err());
        on_mark_a(c"still capturing".as_ptr());
        assert_eq!(calls.load(Ordering::Relaxed), 35);

        panic_next.store(true, Ordering::Relaxed);
        assert_eq!(on_range_push_a(c"panic contained".as_ptr()), 0);
        assert_eq!(on_range_pop(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 37);

        // Dropping the guard disables capture without waiting for a hook that
        // is still running on another thread.
        block_next.store(true, Ordering::Relaxed);
        let in_flight = std::thread::spawn(|| on_mark_a(c"in flight".as_ptr()));
        while !entered.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        let (dropped_tx, dropped_rx) = mpsc::channel();
        let dropper = std::thread::spawn(move || {
            drop(guard);
            dropped_tx.send(()).unwrap();
        });
        let dropped = dropped_rx.recv_timeout(Duration::from_secs(10));
        // Unblock and join both threads even if guard drop unexpectedly waited.
        release.store(true, Ordering::Release);
        dropper.join().unwrap();
        in_flight.join().unwrap();
        dropped.expect("guard drop waited for a running hook");
        assert_eq!(calls.load(Ordering::Relaxed), 38);

        exercise_callbacks();
        assert_eq!(calls.load(Ordering::Relaxed), 38);
        // Shutdown does not release the one-shot hook or permit reactivation.
        assert!(init::install_hook(|_| unreachable!()).is_err());
        exercise_callbacks();
        assert_eq!(calls.load(Ordering::Relaxed), 38);
    }
}
