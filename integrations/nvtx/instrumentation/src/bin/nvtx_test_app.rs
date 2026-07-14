// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Deterministic, multi-threaded NVTX emitter for the `quent-nvtx` capture
//! end-to-end test (VAL-01, D-11).
//!
//! This binary is the "instrumented application": it issues a fixed NVTX v3
//! script through the real NVTX client (a cc-compiled C shim, `c/emit.c`) and
//! links **nothing** from Quent. It never references the injection or bridge
//! crates — capture happens only because the harness sets `NVTX_INJECTION64_PATH`
//! to the `quent-nvtx` capture cdylib and `QUENT_NVTX_OUTPUT_DIR` to an output
//! dir before spawning this process (CAP-01 / VAL-02). No GPU is required.
//!
//! # Emitted timeline (fixed, greppable ids)
//!
//! On the **main** thread (named `quent-nvtx-e2e/main`):
//! 1. `DomainCreate("quent-nvtx-e2e")`.
//! 2. `NameCategory(7, "quent-nvtx-e2e/category-io")`.
//! 3. `RegisterString("quent-nvtx-e2e/registered")`.
//! 4. Nested `RangePush("outer")`, `RangePush("inner")`, `RangePop`, `RangePop`,
//!    then three flat push/pop pairs — five `RangePush` + five `RangePop`.
//! 5. `Mark("quent-nvtx-e2e/mark")` carrying a CORE payload union
//!    (`NVTX_PAYLOAD_TYPE_UNSIGNED_INT64` = `0xCAFE_F00D`, D-12).
//! 6. `ResourceCreate`/`ResourceDestroy` for `quent-nvtx-e2e/resource`.
//!
//! Then two **worker** threads exercise cross-thread range pairing + per-thread
//! naming (D-11): worker A names itself `quent-nvtx-e2e/worker-a` and calls
//! `RangeStartEx` on the shared domain, hands the returned range id to worker B,
//! which names itself `quent-nvtx-e2e/worker-b` and calls `RangeEnd` with that
//! same id. Each worker also does its own push/pop.
//!
//! # Deliberate "nasty" fixtures for Phase 2
//!
//! The main thread leaves an **unclosed** `RangePush("quent-nvtx-e2e/unclosed")`
//! at exit, and a **second** domain `quent-nvtx-e2e-b` is created and marked so a
//! later analyzer inherits multi-domain + unbalanced-range fixtures.

use std::ffi::{CString, c_void};
use std::os::raw::{c_char, c_int};
use std::sync::mpsc;
use std::thread;

// SAFETY: FFI to the deterministic NVTX emitter shim (`c/emit.c`), compiled by
// `build.rs` under the `e2e` feature. Each function is a thin wrapper over one
// NVTX v3 client entry point; the binary depends on nothing else from Quent.
#[link(name = "quent_nvtx_emit", kind = "static")]
unsafe extern "C" {
    fn quent_nvtx_emit_domain_create(name: *const c_char) -> *mut c_void;
    fn quent_nvtx_emit_domain_destroy(domain: *mut c_void);
    fn quent_nvtx_emit_register_string(domain: *mut c_void, s: *const c_char) -> u64;
    fn quent_nvtx_emit_name_category(domain: *mut c_void, category: u32, name: *const c_char);
    fn quent_nvtx_emit_name_current_thread(name: *const c_char) -> u32;
    fn quent_nvtx_emit_mark(domain: *mut c_void, msg: *const c_char, payload: u64);
    fn quent_nvtx_emit_push(domain: *mut c_void, label: *const c_char);
    fn quent_nvtx_emit_pop(domain: *mut c_void);
    fn quent_nvtx_emit_range_start(domain: *mut c_void, label: *const c_char) -> u64;
    fn quent_nvtx_emit_range_end(domain: *mut c_void, id: u64);
    fn quent_nvtx_emit_resource_create(
        domain: *mut c_void,
        id_type: c_int,
        identifier: u64,
        name: *const c_char,
    ) -> *mut c_void;
    fn quent_nvtx_emit_resource_destroy(resource: *mut c_void);
}

/// The CORE payload union value carried on the mark (D-12); asserted verbatim.
const MARK_PAYLOAD: u64 = 0xCAFE_F00D;

/// An opaque NVTX domain handle, wrapped so it can cross the worker-thread
/// boundary. The pointer is never dereferenced in Rust — it is only handed back
/// to the C shim, which passes it to NVTX (opaque under injection).
#[derive(Clone, Copy)]
struct Domain(*mut c_void);

// SAFETY: the handle is an opaque token; it is only forwarded to NVTX via the C
// shim and never dereferenced, so sending it between threads is sound.
unsafe impl Send for Domain {}

/// Build a NUL-terminated C string for a label (kept alive across the FFI call).
fn cstr(s: &str) -> CString {
    CString::new(s).expect("label has no interior NUL")
}

fn main() {
    // SAFETY: every call below forwards valid, live C-string pointers and opaque
    // handles into the NVTX client shim; each string outlives its call.
    unsafe { emit_timeline() };
}

/// # Safety
/// Calls the NVTX client shim; must run once from `main` on a live process.
unsafe fn emit_timeline() {
    let domain_name = cstr("quent-nvtx-e2e");
    // SAFETY: `domain_name` is a valid, live C string.
    let domain = Domain(unsafe { quent_nvtx_emit_domain_create(domain_name.as_ptr()) });

    // Main thread naming (D-11: per-thread naming).
    let main_name = cstr("quent-nvtx-e2e/main");
    // SAFETY: live C string.
    unsafe { quent_nvtx_emit_name_current_thread(main_name.as_ptr()) };

    // Category + registered string.
    let category = cstr("quent-nvtx-e2e/category-io");
    // SAFETY: live C string + opaque domain.
    unsafe { quent_nvtx_emit_name_category(domain.0, 7, category.as_ptr()) };
    let registered = cstr("quent-nvtx-e2e/registered");
    // SAFETY: as above.
    unsafe { quent_nvtx_emit_register_string(domain.0, registered.as_ptr()) };

    // Nested + flat push/pop: five RangePush + five RangePop.
    let outer = cstr("quent-nvtx-e2e/outer");
    let inner = cstr("quent-nvtx-e2e/inner");
    // SAFETY: live C strings + opaque domain.
    unsafe {
        quent_nvtx_emit_push(domain.0, outer.as_ptr());
        quent_nvtx_emit_push(domain.0, inner.as_ptr());
        quent_nvtx_emit_pop(domain.0);
        quent_nvtx_emit_pop(domain.0);
    }
    let flat = cstr("quent-nvtx-e2e/flat");
    for _ in 0..3 {
        // SAFETY: as above.
        unsafe {
            quent_nvtx_emit_push(domain.0, flat.as_ptr());
            quent_nvtx_emit_pop(domain.0);
        }
    }

    // Mark carrying a CORE payload union (D-12).
    let mark = cstr("quent-nvtx-e2e/mark");
    // SAFETY: live C string + opaque domain.
    unsafe { quent_nvtx_emit_mark(domain.0, mark.as_ptr(), MARK_PAYLOAD) };

    // Resource create/destroy.
    let resource_name = cstr("quent-nvtx-e2e/resource");
    // SAFETY: live C string + opaque domain.
    let resource = unsafe {
        quent_nvtx_emit_resource_create(domain.0, 42, 0x1234_5678, resource_name.as_ptr())
    };
    // SAFETY: `resource` is the handle just returned by the shim.
    unsafe { quent_nvtx_emit_resource_destroy(resource) };

    // Cross-thread range pairing + per-thread naming (D-11).
    emit_cross_thread_range(domain);

    // "Nasty" Phase-2 fixtures: a second domain, and an unclosed range at exit.
    let domain_b_name = cstr("quent-nvtx-e2e-b");
    // SAFETY: live C string.
    let domain_b = unsafe { quent_nvtx_emit_domain_create(domain_b_name.as_ptr()) };
    let mark_b = cstr("quent-nvtx-e2e-b/mark");
    // SAFETY: live C string + opaque domain.
    unsafe { quent_nvtx_emit_mark(domain_b, mark_b.as_ptr(), MARK_PAYLOAD) };
    unsafe { quent_nvtx_emit_domain_destroy(domain_b) };

    let unclosed = cstr("quent-nvtx-e2e/unclosed");
    // Intentionally never popped — an unbalanced-range fixture for Phase 2.
    // SAFETY: live C string + opaque domain.
    unsafe { quent_nvtx_emit_push(domain.0, unclosed.as_ptr()) };

    // SAFETY: opaque domain handle from `domain_create`.
    unsafe { quent_nvtx_emit_domain_destroy(domain.0) };
}

/// Worker A starts a process-wide range and hands its id to worker B, which ends
/// it — a cross-thread `RangeStart`/`RangeEnd` pair on the same range id. Each
/// worker also names itself and does its own push/pop.
fn emit_cross_thread_range(domain: Domain) {
    let (tx, rx) = mpsc::channel::<u64>();

    let worker_a = thread::spawn(move || {
        // Capture the whole `Domain` (Send), not just its raw-pointer field.
        let domain = domain;
        let name = cstr("quent-nvtx-e2e/worker-a");
        // SAFETY: live C string.
        unsafe { quent_nvtx_emit_name_current_thread(name.as_ptr()) };

        let cross = cstr("quent-nvtx-e2e/cross");
        // SAFETY: live C string + opaque domain.
        let range_id = unsafe { quent_nvtx_emit_range_start(domain.0, cross.as_ptr()) };
        tx.send(range_id).expect("hand range id to worker B");

        let a_push = cstr("quent-nvtx-e2e/worker-a-range");
        // SAFETY: as above.
        unsafe {
            quent_nvtx_emit_push(domain.0, a_push.as_ptr());
            quent_nvtx_emit_pop(domain.0);
        }
    });

    let worker_b = thread::spawn(move || {
        // Capture the whole `Domain` (Send), not just its raw-pointer field.
        let domain = domain;
        let name = cstr("quent-nvtx-e2e/worker-b");
        // SAFETY: live C string.
        unsafe { quent_nvtx_emit_name_current_thread(name.as_ptr()) };

        let range_id = rx.recv().expect("receive range id from worker A");
        // SAFETY: opaque domain + the id worker A started.
        unsafe { quent_nvtx_emit_range_end(domain.0, range_id) };

        let b_push = cstr("quent-nvtx-e2e/worker-b-range");
        // SAFETY: as above.
        unsafe {
            quent_nvtx_emit_push(domain.0, b_push.as_ptr());
            quent_nvtx_emit_pop(domain.0);
        }
    });

    worker_a.join().expect("worker A joins");
    worker_b.join().expect("worker B joins");
}
