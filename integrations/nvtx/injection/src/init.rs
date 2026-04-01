// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX initialization: registers callback functions into NVTX's
//! function pointer tables.

#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_void;

use crate::bindings::*;
use crate::callbacks;

/// Entry point called by NVTX during initialization.
///
/// # Safety
///
/// Called from NVTX's one-shot init path. `get_export_table` must be a
/// valid function pointer provided by NVTX.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quent_nvtx_initialize_injection(
    get_export_table: unsafe extern "C" fn(u32) -> *const c_void,
) -> i32 {
    // NVTX_ETID_CALLBACKS = 1
    let callbacks_ptr = get_export_table(1) as *const NvtxExportTableCallbacks;
    if callbacks_ptr.is_null() {
        return 0;
    }
    let cb = &*callbacks_ptr;
    let get_table = match cb.GetModuleFunctionTable {
        Some(f) => f,
        None => return 0,
    };

    // Register CORE module callbacks.
    {
        let mut table: NvtxFunctionTable = std::ptr::null_mut();
        let mut size: u32 = 0;
        if get_table(
            NvtxCallbackModule_NVTX_CB_MODULE_CORE,
            &mut table,
            &mut size,
        ) == 0
        {
            return 0;
        }
        if !table.is_null() {
            register_core(table);
        }
    }

    // Register CORE2 module callbacks.
    {
        let mut table: NvtxFunctionTable = std::ptr::null_mut();
        let mut size: u32 = 0;
        if get_table(
            NvtxCallbackModule_NVTX_CB_MODULE_CORE2,
            &mut table,
            &mut size,
        ) == 0
        {
            return 0;
        }
        if !table.is_null() {
            register_core2(table);
        }
    }

    1
}

/// Write a callback function pointer into a table slot.
///
/// # Safety
///
/// `table` must be a valid `NvtxFunctionTable` with at least `idx + 1` slots.
/// The caller is responsible for matching the callback signature to the slot.
unsafe fn write_slot(table: NvtxFunctionTable, idx: usize, f: unsafe extern "C" fn()) {
    let slot = *table.add(idx);
    if !slot.is_null() {
        *slot = Some(f);
    }
}

unsafe fn register_core(table: NvtxFunctionTable) {
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_MarkEx as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const nvtxEventAttributes_v2), unsafe extern "C" fn()>(callbacks::cb_mark_ex),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_MarkA as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const i8), unsafe extern "C" fn()>(callbacks::cb_mark_a),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_MarkW as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const i32), unsafe extern "C" fn()>(callbacks::cb_mark_w),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_RangeStartEx as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const nvtxEventAttributes_v2) -> u64, unsafe extern "C" fn()>(callbacks::cb_range_start_ex),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_RangeStartA as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const i8) -> u64, unsafe extern "C" fn()>(callbacks::cb_range_start_a),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_RangeStartW as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const i32) -> u64, unsafe extern "C" fn()>(callbacks::cb_range_start_w),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_RangeEnd as usize,
        std::mem::transmute::<unsafe extern "C" fn(u64), unsafe extern "C" fn()>(callbacks::cb_range_end),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_RangePushEx as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const nvtxEventAttributes_v2) -> i32, unsafe extern "C" fn()>(callbacks::cb_range_push_ex),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_RangePushA as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const i8) -> i32, unsafe extern "C" fn()>(callbacks::cb_range_push_a),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_RangePushW as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const i32) -> i32, unsafe extern "C" fn()>(callbacks::cb_range_push_w),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_RangePop as usize,
        std::mem::transmute::<unsafe extern "C" fn() -> i32, unsafe extern "C" fn()>(callbacks::cb_range_pop),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_NameCategoryA as usize,
        std::mem::transmute::<unsafe extern "C" fn(u32, *const i8), unsafe extern "C" fn()>(callbacks::cb_name_category_a),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_NameCategoryW as usize,
        std::mem::transmute::<unsafe extern "C" fn(u32, *const i32), unsafe extern "C" fn()>(callbacks::cb_name_category_w),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_NameOsThreadA as usize,
        std::mem::transmute::<unsafe extern "C" fn(u32, *const i8), unsafe extern "C" fn()>(callbacks::cb_name_os_thread_a),
    );
    write_slot(
        table,
        NvtxCallbackIdCore_NVTX_CBID_CORE_NameOsThreadW as usize,
        std::mem::transmute::<unsafe extern "C" fn(u32, *const i32), unsafe extern "C" fn()>(callbacks::cb_name_os_thread_w),
    );
}

unsafe fn register_core2(table: NvtxFunctionTable) {
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainMarkEx as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void, *const nvtxEventAttributes_v2), unsafe extern "C" fn()>(callbacks::cb_domain_mark_ex),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainRangeStartEx as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void, *const nvtxEventAttributes_v2) -> u64, unsafe extern "C" fn()>(callbacks::cb_domain_range_start_ex),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainRangeEnd as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void, u64), unsafe extern "C" fn()>(callbacks::cb_domain_range_end),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainRangePushEx as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void, *const nvtxEventAttributes_v2) -> i32, unsafe extern "C" fn()>(callbacks::cb_domain_range_push_ex),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainRangePop as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void) -> i32, unsafe extern "C" fn()>(callbacks::cb_domain_range_pop),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainResourceCreate as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void, *mut nvtxResourceAttributes_v0) -> *mut c_void, unsafe extern "C" fn()>(callbacks::cb_domain_resource_create),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainResourceDestroy as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void), unsafe extern "C" fn()>(callbacks::cb_domain_resource_destroy),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainNameCategoryA as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void, u32, *const i8), unsafe extern "C" fn()>(callbacks::cb_domain_name_category_a),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainNameCategoryW as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void, u32, *const i32), unsafe extern "C" fn()>(callbacks::cb_domain_name_category_w),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainRegisterStringA as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void, *const i8) -> *mut c_void, unsafe extern "C" fn()>(callbacks::cb_domain_register_string_a),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainRegisterStringW as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void, *const i32) -> *mut c_void, unsafe extern "C" fn()>(callbacks::cb_domain_register_string_w),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainCreateA as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const i8) -> *mut c_void, unsafe extern "C" fn()>(callbacks::cb_domain_create_a),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainCreateW as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const i32) -> *mut c_void, unsafe extern "C" fn()>(callbacks::cb_domain_create_w),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_DomainDestroy as usize,
        std::mem::transmute::<unsafe extern "C" fn(*mut c_void), unsafe extern "C" fn()>(callbacks::cb_domain_destroy),
    );
    write_slot(
        table,
        NvtxCallbackIdCore2_NVTX_CBID_CORE2_Initialize as usize,
        std::mem::transmute::<unsafe extern "C" fn(*const c_void), unsafe extern "C" fn()>(callbacks::cb_initialize),
    );
}
