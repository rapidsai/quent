// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The NVTX injection entry point, one-shot table fill, and the sink-agnostic
//! hook installation surface (D-03/D-15).

use std::mem::transmute;
use std::os::raw::{c_char, c_int, c_uint};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use quent_nvtx_events::NvtxEvent;
use thiserror::Error;

use crate::bindings::{
    NvtxCallbackIdCore, NvtxCallbackIdCore2, NvtxCallbackModule, NvtxExportTableCallbacks,
    NvtxExportTableID, NvtxFunctionPointer, NvtxFunctionTable, NvtxGetExportTableFunc_t,
    nvtxDomainHandle_t, nvtxEventAttributes_t, nvtxStringHandle_t,
};
use crate::callbacks;
use crate::convert::abi::{nvtxRangeId_t, nvtxResourceAttributes_t, nvtxResourceHandle_t};

/// The stored capture hook. Sink-agnostic: it depends only on [`NvtxEvent`].
type Hook = Box<dyn Fn(NvtxEvent) + Send + Sync + 'static>;

static HOOK: OnceLock<Hook> = OnceLock::new();

/// One-shot guard for [`InitializeInjectionNvtx2`]; caches whether the CORE2
/// callback tables were installed successfully.
static INITIALIZED: OnceLock<bool> = OnceLock::new();

/// Monotonic source of the NVTX handles/ids the injection layer synthesizes and
/// hands back to the application: domain, registered-string, and resource handles
/// plus range ids. Starts at `1` so `0` stays reserved for the default/NULL
/// domain. In injection mode these values are opaque to NVTX (never
/// dereferenced), so a synthetic counter is a valid handle source; they are
/// captured verbatim (D-01).
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

/// Return a fresh, process-unique, nonzero handle/id.
pub(crate) fn next_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

/// Error returned by [`install_hook`].
#[derive(Debug, Error)]
pub enum InstallHookError {
    /// A hook was already installed; installation is one-shot per process.
    #[error("an NVTX capture hook is already installed (install_hook is one-shot per process)")]
    AlreadyInstalled,
}

/// Install the process-global, sink-agnostic capture hook (D-03).
///
/// The hook receives every converted [`NvtxEvent`]. It is stored in a
/// [`OnceLock`], so it can be installed exactly once per process — matching the
/// one-shot nature of NVTX injection.
///
/// # Errors
/// Returns [`InstallHookError::AlreadyInstalled`] if a hook was already set.
pub fn install_hook<F>(hook: F) -> Result<(), InstallHookError>
where
    F: Fn(NvtxEvent) + Send + Sync + 'static,
{
    HOOK.set(Box::new(hook))
        .map_err(|_| InstallHookError::AlreadyInstalled)
}

/// Dispatch a converted event to the installed hook, if any. Events that arrive
/// before a hook is installed are dropped (Pitfall 1).
pub(crate) fn dispatch(event: NvtxEvent) {
    if let Some(hook) = HOOK.get() {
        hook(event);
    }
}

/// NVTX injection entry point.
///
/// NVTX loads this cdylib via `NVTX_INJECTION64_PATH` and calls this exactly
/// once, lazily, before the first NVTX call, passing its export-table accessor.
/// Returns `1` on success per the NVTX ABI. All state is constructed here under
/// a [`OnceLock`] — never in a `#[ctor]` — to avoid static-init-order hazards
/// (RESEARCH R-03 / Pitfall 6).
///
/// # Safety
/// `get_export_table` must be the accessor NVTX supplies; calling this with an
/// arbitrary pointer is undefined behavior. NVTX itself upholds this contract.
#[unsafe(no_mangle)]
pub extern "C" fn InitializeInjectionNvtx2(get_export_table: NvtxGetExportTableFunc_t) -> c_int {
    let installed = *INITIALIZED.get_or_init(|| {
        // SAFETY: `get_export_table` is the NVTX-provided export-table accessor.
        unsafe { install_callbacks(get_export_table) }
    });
    c_int::from(installed)
}

/// The `GetModuleFunctionTable` accessor type from [`NvtxExportTableCallbacks`].
type GetModuleTableFn = unsafe extern "C" fn(
    callback_module: NvtxCallbackModule::Type,
    out_table: *mut NvtxFunctionTable,
    out_size: *mut c_uint,
) -> c_int;

/// Subscribe `$cb` (typed `$sig`) at `$cbid` in the already-fetched `$table`.
///
/// Casts each typed subscriber to the ABI's generic function-pointer type. All
/// NVTX subscribers share the C ABI, so this transmute is the intended
/// subscription mechanism (mirrors the sample-injection `reinterpret_cast`).
macro_rules! subscribe {
    ($table:expr, $size:expr, $cbid:expr, $cb:path, $sig:ty) => {{
        // SAFETY: fn pointers are all the same width; the callee's real signature
        // (`$sig`) matches what NVTX invokes for `$cbid`.
        let erased: NvtxFunctionPointer =
            Some(unsafe { transmute::<$sig, unsafe extern "C" fn()>($cb) });
        // SAFETY: `$table` has `$size` slots per the ABI contract.
        unsafe { set_callback($table, $size, $cbid, erased) };
    }};
}

/// Retrieve the CORE and CORE2 function tables and install every capture
/// callback. CORE2 (the domain-scoped surface) is required for success; the CORE
/// table (only `NameOsThreadA` lives there for our surface) is best-effort.
///
/// # Captured surface (WR-03 — Phase 1 scope)
/// Phase 1 subscribes **only** the domain-scoped ASCII CORE2 surface (mark,
/// range start/end/push/pop, domain/register-string/name-category/resource) plus
/// CORE `NameOsThreadA`. The classic default-domain API
/// (`nvtxRangePushA`/`nvtxMarkA`/`nvtxRangeStartA`/`nvtxRangeStartEx`) and every
/// wide-char (`*W` / Unicode) variant are intentionally **NOT** subscribed yet:
/// coverage is deferred because the target is Linux-only and the v1 target
/// libraries (libcudf, cuCascade) are domain-scoped. An app that only uses the
/// non-domain or wide-char surface will produce an empty/partial capture; a
/// one-time runtime diagnostic on the unsupported message path (see
/// [`crate::convert`] `decode_message`) surfaces that gap.
///
/// # Safety
/// `get_export_table` must be the accessor NVTX passes to
/// [`InitializeInjectionNvtx2`].
unsafe fn install_callbacks(get_export_table: NvtxGetExportTableFunc_t) -> bool {
    let Some(get_export_table) = get_export_table else {
        return false;
    };

    // SAFETY: NVTX ABI — NVTX_ETID_CALLBACKS yields a `*const NvtxExportTableCallbacks`.
    let callbacks_table = unsafe { get_export_table(NvtxExportTableID::NVTX_ETID_CALLBACKS) }
        .cast::<NvtxExportTableCallbacks>();
    if callbacks_table.is_null() {
        return false;
    }

    // SAFETY: non-null per the check above; NVTX owns the table for the process.
    let Some(get_module_table) = (unsafe { *callbacks_table }).GetModuleFunctionTable else {
        return false;
    };

    // SAFETY: `get_module_table` is the ABI accessor from the callbacks table.
    let core2 = unsafe { install_core2(get_module_table) };
    // Best-effort: thread naming is a nice-to-have, not required for success.
    // SAFETY: as above.
    unsafe { install_core(get_module_table) };
    core2
}

/// Fetch a module's function table, or `None` if the ABI declines it.
///
/// # Safety
/// `get_module_table` must be the NVTX `GetModuleFunctionTable` accessor.
unsafe fn module_table(
    get_module_table: GetModuleTableFn,
    module: NvtxCallbackModule::Type,
) -> Option<(NvtxFunctionTable, c_uint)> {
    let mut table: NvtxFunctionTable = std::ptr::null_mut();
    let mut size: c_uint = 0;
    // SAFETY: ABI call; `table` and `size` are valid out-parameters.
    let rc = unsafe { get_module_table(module, &mut table, &mut size) };
    if rc == 0 || table.is_null() {
        return None;
    }
    Some((table, size))
}

/// Install every CORE2 (domain-scoped) capture callback. Returns `false` if the
/// CORE2 table is unavailable.
///
/// # Safety
/// See [`module_table`].
unsafe fn install_core2(get_module_table: GetModuleTableFn) -> bool {
    let Some((table, size)) =
        (unsafe { module_table(get_module_table, NvtxCallbackModule::NVTX_CB_MODULE_CORE2) })
    else {
        return false;
    };
    use NvtxCallbackIdCore2 as Cb;

    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainMarkEx,
        callbacks::on_domain_mark_ex,
        extern "C" fn(nvtxDomainHandle_t, *const nvtxEventAttributes_t) -> c_int
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainRangeStartEx,
        callbacks::on_domain_range_start_ex,
        extern "C" fn(nvtxDomainHandle_t, *const nvtxEventAttributes_t) -> nvtxRangeId_t
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainRangeEnd,
        callbacks::on_domain_range_end,
        extern "C" fn(nvtxDomainHandle_t, nvtxRangeId_t) -> c_int
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainRangePushEx,
        callbacks::on_domain_range_push_ex,
        extern "C" fn(nvtxDomainHandle_t, *const nvtxEventAttributes_t) -> c_int
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainRangePop,
        callbacks::on_domain_range_pop,
        extern "C" fn(nvtxDomainHandle_t) -> c_int
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainResourceCreate,
        callbacks::on_domain_resource_create,
        extern "C" fn(nvtxDomainHandle_t, *const nvtxResourceAttributes_t) -> nvtxResourceHandle_t
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainResourceDestroy,
        callbacks::on_domain_resource_destroy,
        extern "C" fn(nvtxResourceHandle_t) -> c_int
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainNameCategoryA,
        callbacks::on_domain_name_category_a,
        extern "C" fn(nvtxDomainHandle_t, u32, *const c_char) -> c_int
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainRegisterStringA,
        callbacks::on_domain_register_string_a,
        extern "C" fn(nvtxDomainHandle_t, *const c_char) -> nvtxStringHandle_t
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainCreateA,
        callbacks::on_domain_create_a,
        extern "C" fn(*const c_char) -> nvtxDomainHandle_t
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE2_DomainDestroy,
        callbacks::on_domain_destroy,
        extern "C" fn(nvtxDomainHandle_t) -> c_int
    );
    true
}

/// Install the CORE (non-domain) capture callbacks. Only `NameOsThreadA` is in
/// our capture surface; best-effort (absence just means no thread names).
///
/// # Safety
/// See [`module_table`].
unsafe fn install_core(get_module_table: GetModuleTableFn) {
    let Some((table, size)) =
        (unsafe { module_table(get_module_table, NvtxCallbackModule::NVTX_CB_MODULE_CORE) })
    else {
        // Silent-disable path (WR-05): without the CORE table, OS thread naming
        // is never captured. The cdylib installs no tracing subscriber, so emit
        // an unconditional diagnostic instead of failing quietly.
        eprintln!(
            "quent-nvtx: NVTX CORE callback table unavailable; OS thread names will not be captured"
        );
        return;
    };
    subscribe!(
        table,
        size,
        NvtxCallbackIdCore::NVTX_CBID_CORE_NameOsThreadA,
        callbacks::on_name_os_thread_a,
        extern "C" fn(u32, *const c_char) -> c_int
    );
}

/// Write `func` into `table[cbid]` (the ABI stores an array of pointers-to-
/// function-pointers, so we set `*table[cbid]`), bounds-checked against `size`.
///
/// # Safety
/// `table` must be a valid `NvtxFunctionTable` with at least `size` slots.
unsafe fn set_callback(
    table: NvtxFunctionTable,
    size: c_uint,
    cbid: c_uint,
    func: NvtxFunctionPointer,
) {
    let index = cbid as usize;
    if index >= size as usize {
        return;
    }
    // SAFETY: `index < size`; `table` holds that many slot pointers.
    let slot = unsafe { *table.add(index) };
    if slot.is_null() {
        return;
    }
    // SAFETY: `slot` points to a writable `NvtxFunctionPointer` cell owned by NVTX.
    unsafe { *slot = func };
}
