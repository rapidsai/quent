// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX callback implementations for CORE and CORE2 modules.
//!
//! All callbacks are `unsafe extern "C"` because they are called from C code
//! through NVTX's function pointer table.

// All functions in this module are unsafe extern "C" callbacks whose bodies
// are inherently unsafe (raw pointer dereferences, FFI calls). Wrapping every
// statement in unsafe {} adds noise without safety benefit.
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};

use quent_nvtx_events::*;

use crate::bindings::{nvtxEventAttributes_v2, nvtxResourceAttributes_v0};
use crate::convert;
use crate::emit;

// ---------------------------------------------------------------------------
// Counters
// ---------------------------------------------------------------------------

// Relaxed is sufficient — we only need uniqueness, not cross-thread ordering.
static RANGE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static HANDLE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Returned from push/pop callbacks to signal that this injection does not
/// track nesting depth. NVTX interprets this as "depth not available".
const NO_PUSH_POP_TRACKING: i32 = -2;

fn next_range_id() -> u64 {
    RANGE_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn alloc_handle() -> (*mut c_void, u64) {
    let id = HANDLE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let ptr = Box::into_raw(Box::new(id));
    (ptr as *mut c_void, id)
}

unsafe fn free_handle(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut u64)) };
    }
}

// ---------------------------------------------------------------------------
// Thread ID
// ---------------------------------------------------------------------------

// Return values are not checked: gettid() cannot fail, and
// pthread_threadid_np(0, _) always succeeds (0 = current thread).

#[cfg(target_os = "linux")]
fn current_thread_id() -> u64 {
    unsafe { libc::syscall(libc::SYS_gettid) as u64 }
}

#[cfg(target_os = "macos")]
fn current_thread_id() -> u64 {
    let mut tid: u64 = 0;
    unsafe { libc::pthread_threadid_np(0, &mut tid) };
    tid
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn current_thread_id() -> u64 {
    0
}

// ---------------------------------------------------------------------------
// CORE module callbacks
// ---------------------------------------------------------------------------

// Slot 1: MarkEx
pub(crate) unsafe extern "C" fn cb_mark_ex(attr: *const nvtxEventAttributes_v2) {
    emit(NvtxEvent::Mark(Mark {
        thread_id: current_thread_id(),
        domain_handle_id: None,
        attributes: convert::convert_attributes(attr),
    }));
}

// Slot 2: MarkA
pub(crate) unsafe extern "C" fn cb_mark_a(msg: *const i8) {
    emit(NvtxEvent::Mark(Mark {
        thread_id: current_thread_id(),
        domain_handle_id: None,
        attributes: convert::attributes_from_ascii(msg),
    }));
}

// Slot 3: MarkW
pub(crate) unsafe extern "C" fn cb_mark_w(msg: *const i32) {
    emit(NvtxEvent::Mark(Mark {
        thread_id: current_thread_id(),
        domain_handle_id: None,
        attributes: convert::attributes_from_wchar(msg),
    }));
}

// Slot 4: RangeStartEx
pub(crate) unsafe extern "C" fn cb_range_start_ex(
    attr: *const nvtxEventAttributes_v2,
) -> u64 {
    let id = next_range_id();
    emit(NvtxEvent::RangeStart(RangeStart {
        range_handle_id: id,
        domain_handle_id: None,
        attributes: convert::convert_attributes(attr),
    }));
    id
}

// Slot 5: RangeStartA
pub(crate) unsafe extern "C" fn cb_range_start_a(msg: *const i8) -> u64 {
    let id = next_range_id();
    emit(NvtxEvent::RangeStart(RangeStart {
        range_handle_id: id,
        domain_handle_id: None,
        attributes: convert::attributes_from_ascii(msg),
    }));
    id
}

// Slot 6: RangeStartW
pub(crate) unsafe extern "C" fn cb_range_start_w(msg: *const i32) -> u64 {
    let id = next_range_id();
    emit(NvtxEvent::RangeStart(RangeStart {
        range_handle_id: id,
        domain_handle_id: None,
        attributes: convert::attributes_from_wchar(msg),
    }));
    id
}

// Slot 7: RangeEnd
pub(crate) unsafe extern "C" fn cb_range_end(id: u64) {
    emit(NvtxEvent::RangeEnd(RangeEnd {
        range_handle_id: id,
        domain_handle_id: None,
    }));
}

// Slot 8: RangePushEx
pub(crate) unsafe extern "C" fn cb_range_push_ex(
    attr: *const nvtxEventAttributes_v2,
) -> i32 {
    emit(NvtxEvent::Push(Push {
        thread_id: current_thread_id(),
        domain_handle_id: None,
        attributes: convert::convert_attributes(attr),
    }));
    NO_PUSH_POP_TRACKING
}

// Slot 9: RangePushA
pub(crate) unsafe extern "C" fn cb_range_push_a(msg: *const i8) -> i32 {
    emit(NvtxEvent::Push(Push {
        thread_id: current_thread_id(),
        domain_handle_id: None,
        attributes: convert::attributes_from_ascii(msg),
    }));
    NO_PUSH_POP_TRACKING
}

// Slot 10: RangePushW
pub(crate) unsafe extern "C" fn cb_range_push_w(msg: *const i32) -> i32 {
    emit(NvtxEvent::Push(Push {
        thread_id: current_thread_id(),
        domain_handle_id: None,
        attributes: convert::attributes_from_wchar(msg),
    }));
    NO_PUSH_POP_TRACKING
}

// Slot 11: RangePop
pub(crate) unsafe extern "C" fn cb_range_pop() -> i32 {
    emit(NvtxEvent::Pop(Pop {
        thread_id: current_thread_id(),
        domain_handle_id: None,
    }));
    NO_PUSH_POP_TRACKING
}

// Slot 12: NameCategoryA
pub(crate) unsafe extern "C" fn cb_name_category_a(category: u32, name: *const i8) {
    emit(NvtxEvent::NameCategory(NameCategory {
        domain_handle_id: None,
        category_id: category,
        name: convert::cstr_to_string(name),
    }));
}

// Slot 13: NameCategoryW
pub(crate) unsafe extern "C" fn cb_name_category_w(category: u32, name: *const i32) {
    emit(NvtxEvent::NameCategory(NameCategory {
        domain_handle_id: None,
        category_id: category,
        name: convert::wstr_to_string(name),
    }));
}

// Slot 14: NameOsThreadA
pub(crate) unsafe extern "C" fn cb_name_os_thread_a(tid: u32, name: *const i8) {
    emit(NvtxEvent::NameThread(NameThread {
        os_thread_id: tid,
        name: convert::cstr_to_string(name),
    }));
}

// Slot 15: NameOsThreadW
pub(crate) unsafe extern "C" fn cb_name_os_thread_w(tid: u32, name: *const i32) {
    emit(NvtxEvent::NameThread(NameThread {
        os_thread_id: tid,
        name: convert::wstr_to_string(name),
    }));
}

// ---------------------------------------------------------------------------
// CORE2 module callbacks (domain-aware)
// ---------------------------------------------------------------------------

// Slot 1: DomainMarkEx
pub(crate) unsafe extern "C" fn cb_domain_mark_ex(
    domain: *mut c_void,
    attr: *const nvtxEventAttributes_v2,
) {
    emit(NvtxEvent::Mark(Mark {
        thread_id: current_thread_id(),
        domain_handle_id: convert::domain_handle_id(domain),
        attributes: convert::convert_attributes(attr),
    }));
}

// Slot 2: DomainRangeStartEx
pub(crate) unsafe extern "C" fn cb_domain_range_start_ex(
    domain: *mut c_void,
    attr: *const nvtxEventAttributes_v2,
) -> u64 {
    let id = next_range_id();
    emit(NvtxEvent::RangeStart(RangeStart {
        range_handle_id: id,
        domain_handle_id: convert::domain_handle_id(domain),
        attributes: convert::convert_attributes(attr),
    }));
    id
}

// Slot 3: DomainRangeEnd
pub(crate) unsafe extern "C" fn cb_domain_range_end(domain: *mut c_void, id: u64) {
    emit(NvtxEvent::RangeEnd(RangeEnd {
        range_handle_id: id,
        domain_handle_id: convert::domain_handle_id(domain),
    }));
}

// Slot 4: DomainRangePushEx
pub(crate) unsafe extern "C" fn cb_domain_range_push_ex(
    domain: *mut c_void,
    attr: *const nvtxEventAttributes_v2,
) -> i32 {
    emit(NvtxEvent::Push(Push {
        thread_id: current_thread_id(),
        domain_handle_id: convert::domain_handle_id(domain),
        attributes: convert::convert_attributes(attr),
    }));
    NO_PUSH_POP_TRACKING
}

// Slot 5: DomainRangePop
pub(crate) unsafe extern "C" fn cb_domain_range_pop(domain: *mut c_void) -> i32 {
    emit(NvtxEvent::Pop(Pop {
        thread_id: current_thread_id(),
        domain_handle_id: convert::domain_handle_id(domain),
    }));
    NO_PUSH_POP_TRACKING
}

// Slot 6: DomainResourceCreate
pub(crate) unsafe extern "C" fn cb_domain_resource_create(
    domain: *mut c_void,
    attr: *mut nvtxResourceAttributes_v0,
) -> *mut c_void {
    let (handle_ptr, resource_id) = alloc_handle();
    emit(NvtxEvent::ResourceCreate(ResourceCreate {
        domain_handle_id: convert::domain_handle_id(domain),
        resource_handle_id: resource_id,
        attributes: convert::convert_resource_attributes(attr),
    }));
    handle_ptr
}

// Slot 7: DomainResourceDestroy
pub(crate) unsafe extern "C" fn cb_domain_resource_destroy(resource: *mut c_void) {
    if !resource.is_null() {
        let id = *(resource as *const u64);
        emit(NvtxEvent::ResourceDestroy(ResourceDestroy {
            resource_handle_id: id,
        }));
        free_handle(resource);
    }
}

// Slot 8: DomainNameCategoryA
pub(crate) unsafe extern "C" fn cb_domain_name_category_a(
    domain: *mut c_void,
    category: u32,
    name: *const i8,
) {
    emit(NvtxEvent::NameCategory(NameCategory {
        domain_handle_id: convert::domain_handle_id(domain),
        category_id: category,
        name: convert::cstr_to_string(name),
    }));
}

// Slot 9: DomainNameCategoryW
pub(crate) unsafe extern "C" fn cb_domain_name_category_w(
    domain: *mut c_void,
    category: u32,
    name: *const i32,
) {
    emit(NvtxEvent::NameCategory(NameCategory {
        domain_handle_id: convert::domain_handle_id(domain),
        category_id: category,
        name: convert::wstr_to_string(name),
    }));
}

// Slot 10: DomainRegisterStringA
pub(crate) unsafe extern "C" fn cb_domain_register_string_a(
    domain: *mut c_void,
    string: *const i8,
) -> *mut c_void {
    let (handle_ptr, string_id) = alloc_handle();
    emit(NvtxEvent::RegisterString(RegisterString {
        domain_handle_id: convert::handle_to_id(domain),
        string_handle_id: string_id,
        value: convert::cstr_to_string(string),
    }));
    handle_ptr
}

// Slot 11: DomainRegisterStringW
pub(crate) unsafe extern "C" fn cb_domain_register_string_w(
    domain: *mut c_void,
    string: *const i32,
) -> *mut c_void {
    let (handle_ptr, string_id) = alloc_handle();
    emit(NvtxEvent::RegisterString(RegisterString {
        domain_handle_id: convert::handle_to_id(domain),
        string_handle_id: string_id,
        value: convert::wstr_to_string(string),
    }));
    handle_ptr
}

// Slot 12: DomainCreateA
pub(crate) unsafe extern "C" fn cb_domain_create_a(name: *const i8) -> *mut c_void {
    let (handle_ptr, domain_id) = alloc_handle();
    emit(NvtxEvent::DomainCreate(DomainCreate {
        domain_handle_id: domain_id,
        name: convert::cstr_to_string(name),
    }));
    handle_ptr
}

// Slot 13: DomainCreateW
pub(crate) unsafe extern "C" fn cb_domain_create_w(name: *const i32) -> *mut c_void {
    let (handle_ptr, domain_id) = alloc_handle();
    emit(NvtxEvent::DomainCreate(DomainCreate {
        domain_handle_id: domain_id,
        name: convert::wstr_to_string(name),
    }));
    handle_ptr
}

// Slot 14: DomainDestroy
pub(crate) unsafe extern "C" fn cb_domain_destroy(domain: *mut c_void) {
    if !domain.is_null() {
        let id = *(domain as *const u64);
        emit(NvtxEvent::DomainDestroy(DomainDestroy {
            domain_handle_id: id,
        }));
        free_handle(domain);
    }
}

// Slot 15: Initialize
pub(crate) unsafe extern "C" fn cb_initialize(_reserved: *const c_void) {
    // No-op.
}
