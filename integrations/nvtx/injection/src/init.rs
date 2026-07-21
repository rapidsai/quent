// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The NVTX injection entry point, one-shot table fill, and the sink-agnostic
//! hook installation surface.

use std::mem::transmute;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use quent_nvtx_events::NvtxEvent;
use thiserror::Error;

use crate::bindings::{
    NvtxCallbackIdCore, NvtxCallbackIdCore2, NvtxCallbackModule, NvtxExportTableCallbacks,
    NvtxExportTableID, NvtxFunctionPointer, NvtxFunctionTable, NvtxGetExportTableFunc_t,
    nvtxDomainHandle_t, nvtxEventAttributes_t, nvtxRangeId_t, nvtxResourceAttributes_t,
    nvtxResourceHandle_t, nvtxStringHandle_t,
};
use crate::callbacks;

/// The stored capture hook. Sink-agnostic: it depends only on [`NvtxEvent`].
type Hook = Box<dyn Fn(NvtxEvent) + Send + Sync + 'static>;

static HOOK: OnceLock<Hook> = OnceLock::new();

/// Monotonic source of the NVTX handles/ids the injection layer synthesizes and
/// hands back to the application: domain, registered-string, and resource handles
/// plus range ids. Starts at `1` so `0` stays reserved for the default/NULL
/// domain. In injection mode these values are opaque to NVTX (never
/// dereferenced), so a synthetic counter is a valid handle source; they are
/// captured verbatim.
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

/// Return a fresh, process-unique, nonzero handle/id.
pub(crate) fn next_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

thread_local! {
    /// Per-thread, per-domain count of currently-open push/pop ranges.
    ///
    /// `nvtxDomainRangePushEx` returns the 0-based level of the range being
    /// started and `nvtxDomainRangePop` the level of the range being ended;
    /// the nesting stack is per-thread and per-domain. We mirror those return
    /// values so an app that reads them observes faithful behavior instead of a
    /// constant.
    static RANGE_DEPTH: std::cell::RefCell<std::collections::HashMap<u64, i32>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// The 0-based nesting level of a `DomainRangePushEx` on this thread for
/// `domain`, then increment the open-range depth. Returns the level of the
/// range being started (NVTX `nvtxDomainRangePushEx` return semantics).
pub(crate) fn range_push_level(domain: u64) -> c_int {
    RANGE_DEPTH.with(|depth| {
        let mut map = depth.borrow_mut();
        let level = map.entry(domain).or_insert(0);
        let started = *level;
        *level += 1;
        started
    })
}

/// Decrement this thread's open-range depth for `domain` and return the 0-based
/// level of the range being ended (NVTX `nvtxDomainRangePop` return semantics).
/// An unbalanced pop saturates at level `0`.
pub(crate) fn range_pop_level(domain: u64) -> c_int {
    RANGE_DEPTH.with(|depth| {
        let mut map = depth.borrow_mut();
        let level = map.entry(domain).or_insert(0);
        *level = (*level - 1).max(0);
        let result = *level;
        // Drop the entry once this domain's stack is empty so the map tracks
        // only domains with currently-open ranges, not every domain ever seen.
        if result == 0 {
            map.remove(&domain);
        }
        result
    })
}

/// Error returned by [`install_hook`].
#[derive(Debug, Error)]
pub enum InstallHookError {
    /// A hook was already installed; installation is one-shot per process.
    #[error("an NVTX capture hook is already installed (install_hook is one-shot per process)")]
    AlreadyInstalled,
}

/// Install the process-global, sink-agnostic capture hook.
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
/// before a hook is installed are dropped.
pub(crate) fn dispatch(event: NvtxEvent) {
    // Guard against hook-induced re-entry: if the hook (or code it calls) emits
    // NVTX, it would recurse into this synchronous dispatch path and overflow
    // the stack, bypassing the callbacks' panic barriers. Drop nested events.
    // The RAII reset clears the flag even if the hook unwinds.
    thread_local! {
        static IN_DISPATCH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if IN_DISPATCH.with(|g| g.replace(true)) {
        return;
    }
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            IN_DISPATCH.with(|g| g.set(false));
        }
    }
    let _reset = Reset;

    if let Some(hook) = HOOK.get() {
        hook(event);
    }
}

/// NVTX injection entry point.
///
/// NVTX loads this cdylib via `NVTX_INJECTION64_PATH` and calls this **once per
/// NVTX-using image** in the process — the executable and each instrumented
/// shared library keep their own NVTX state (per-image `nvtxGlobals`) and
/// initialize lazily before that image's first NVTX call, each passing its own
/// export-table accessor. We must therefore install callbacks into *every*
/// caller's tables, not just the first: a later image left uninstalled has its
/// NVTX functions turned into silent no-ops by NVTX, dropping its events. All
/// state is constructed under [`OnceLock`]s — never in a `#[ctor]` — to avoid
/// static-init-order hazards. Returns `1` on success per the NVTX ABI.
///
/// # Safety
/// `get_export_table` must be the accessor NVTX supplies; calling this with an
/// arbitrary pointer is undefined behavior. NVTX itself upholds this contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn InitializeInjectionNvtx2(
    get_export_table: NvtxGetExportTableFunc_t,
) -> c_int {
    // Contain any panic in the init path (e.g. a diagnostic write failing) so it
    // never unwinds across the C ABI and aborts the host application. Runs on
    // every call: `install_callbacks` writes only into the caller-supplied
    // tables and touches no shared capture state, so per-image (and concurrent,
    // or repeat) calls are independent and safe.
    let installed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: `get_export_table` is the NVTX-provided export-table accessor.
        unsafe { install_callbacks(get_export_table) }
    }))
    .unwrap_or(false);
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
/// Expands to a `bool`: whether the callback was installed (see [`set_callback`]).
macro_rules! subscribe {
    ($table:expr, $size:expr, $cbid:expr, $cb:path, $sig:ty) => {{
        // SAFETY: fn pointers are all the same width; the callee's real signature
        // (`$sig`) matches what NVTX invokes for `$cbid`.
        let erased: NvtxFunctionPointer =
            Some(unsafe { transmute::<$sig, unsafe extern "C" fn()>($cb) });
        // SAFETY: `$table` has `$size` slots per the ABI contract.
        unsafe { set_callback($table, $size, $cbid, erased) }
    }};
}

/// Retrieve the CORE and CORE2 function tables and install every capture
/// callback. CORE2 (the domain-scoped surface) is required for success; the CORE
/// (default-domain) table is best-effort.
///
/// # Captured surface
/// This layer subscribes both the domain-scoped ASCII CORE2 surface (mark,
/// range start/end/push/pop, domain/register-string/name-category/resource) and
/// the classic default-domain ASCII CORE surface
/// (`nvtxMarkA`/`nvtxMarkEx`, `nvtxRangePushA`/`nvtxRangePushEx`, `nvtxRangePop`,
/// `nvtxRangeStartA`/`nvtxRangeStartEx`/`nvtxRangeEnd`, `nvtxNameCategoryA`,
/// `nvtxNameOsThreadA`), captured on the default domain (`0`). The wide-char
/// (`*W` / Unicode) variants are subscribed with warn-once stubs — Unicode
/// capture is still deferred, but the calls emit a one-time diagnostic (see
/// [`callbacks`] `warn_wide_surface_once`) and keep range nesting/ids valid
/// instead of silently becoming no-ops.
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

    // Every required CORE2 (domain) callback. `subscribe!` reports whether each
    // slot installed; a `false` is a cbid the running NVTX's table does not
    // expose (out of range for `size`, or a null slot).
    let installed = [
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainMarkEx,
            callbacks::on_domain_mark_ex,
            extern "C" fn(nvtxDomainHandle_t, *const nvtxEventAttributes_t)
        ),
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainRangeStartEx,
            callbacks::on_domain_range_start_ex,
            extern "C" fn(nvtxDomainHandle_t, *const nvtxEventAttributes_t) -> nvtxRangeId_t
        ),
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainRangeEnd,
            callbacks::on_domain_range_end,
            extern "C" fn(nvtxDomainHandle_t, nvtxRangeId_t)
        ),
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainRangePushEx,
            callbacks::on_domain_range_push_ex,
            extern "C" fn(nvtxDomainHandle_t, *const nvtxEventAttributes_t) -> c_int
        ),
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainRangePop,
            callbacks::on_domain_range_pop,
            extern "C" fn(nvtxDomainHandle_t) -> c_int
        ),
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainResourceCreate,
            callbacks::on_domain_resource_create,
            extern "C" fn(
                nvtxDomainHandle_t,
                *const nvtxResourceAttributes_t,
            ) -> nvtxResourceHandle_t
        ),
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainResourceDestroy,
            callbacks::on_domain_resource_destroy,
            extern "C" fn(nvtxResourceHandle_t)
        ),
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainNameCategoryA,
            callbacks::on_domain_name_category_a,
            extern "C" fn(nvtxDomainHandle_t, u32, *const c_char)
        ),
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainRegisterStringA,
            callbacks::on_domain_register_string_a,
            extern "C" fn(nvtxDomainHandle_t, *const c_char) -> nvtxStringHandle_t
        ),
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainCreateA,
            callbacks::on_domain_create_a,
            extern "C" fn(*const c_char) -> nvtxDomainHandle_t
        ),
        subscribe!(
            table,
            size,
            Cb::NVTX_CBID_CORE2_DomainDestroy,
            callbacks::on_domain_destroy,
            extern "C" fn(nvtxDomainHandle_t)
        ),
    ];

    let done = installed.iter().filter(|&&ok| ok).count();
    if done < installed.len() {
        // The cdylib installs no tracing subscriber, so surface the partial
        // install rather than capturing a silently incomplete domain surface.
        eprintln!(
            "quent-nvtx: installed {done}/{} CORE2 domain callbacks (table reports {size} \
             slots); the rest are domain calls the running NVTX does not expose and will not \
             be captured",
            installed.len()
        );
    }

    // The CORE2 table was obtained, so injection is viable. Report success even
    // on a partial install: un-installed slots are calls the running NVTX lacks,
    // so there is nothing there to capture, and failing here would make NVTX
    // discard the injection entirely. The diagnostic above surfaces any gap.
    true
}

/// Install the CORE (default-domain, non-domain-scoped) capture callbacks.
///
/// The classic NVTX API (`nvtxMarkA`, `nvtxRangePushA`, `nvtxRangePop`, …)
/// dispatches through this table, not the CORE2 domain surface, so we capture it
/// on the default domain (`0`). The wide-char (`*W`) entries are subscribed with
/// warn-once stubs so they signal instead of silently becoming no-ops; ASCII is
/// captured. Best-effort: if the CORE table is unavailable, the default-domain
/// surface simply isn't hooked (the domain surface is what gates init success).
///
/// # Safety
/// See [`module_table`].
unsafe fn install_core(get_module_table: GetModuleTableFn) {
    let Some((table, size)) =
        (unsafe { module_table(get_module_table, NvtxCallbackModule::NVTX_CB_MODULE_CORE) })
    else {
        // The cdylib installs no tracing subscriber, so emit an unconditional
        // diagnostic instead of failing quietly.
        eprintln!(
            "quent-nvtx: NVTX CORE callback table unavailable; default-domain and OS-thread-name \
             events will not be captured"
        );
        return;
    };
    use NvtxCallbackIdCore as Cb;

    // Default-domain ASCII surface, captured verbatim on domain `0`.
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_MarkEx,
        callbacks::on_mark_ex,
        extern "C" fn(*const nvtxEventAttributes_t)
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_MarkA,
        callbacks::on_mark_a,
        extern "C" fn(*const c_char)
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_RangeStartEx,
        callbacks::on_range_start_ex,
        extern "C" fn(*const nvtxEventAttributes_t) -> nvtxRangeId_t
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_RangeStartA,
        callbacks::on_range_start_a,
        extern "C" fn(*const c_char) -> nvtxRangeId_t
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_RangeEnd,
        callbacks::on_range_end,
        extern "C" fn(nvtxRangeId_t)
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_RangePushEx,
        callbacks::on_range_push_ex,
        extern "C" fn(*const nvtxEventAttributes_t) -> c_int
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_RangePushA,
        callbacks::on_range_push_a,
        extern "C" fn(*const c_char) -> c_int
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_RangePop,
        callbacks::on_range_pop,
        extern "C" fn() -> c_int
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_NameCategoryA,
        callbacks::on_name_category_a,
        extern "C" fn(u32, *const c_char)
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_NameOsThreadA,
        callbacks::on_name_os_thread_a,
        extern "C" fn(u32, *const c_char)
    );

    // Wide-char (Unicode) surface: subscribed with warn-once stubs so the calls
    // signal and keep range nesting/ids valid instead of silently no-op'ing.
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_MarkW,
        callbacks::on_mark_w,
        extern "C" fn(*const c_void)
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_RangeStartW,
        callbacks::on_range_start_w,
        extern "C" fn(*const c_void) -> nvtxRangeId_t
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_RangePushW,
        callbacks::on_range_push_w,
        extern "C" fn(*const c_void) -> c_int
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_NameCategoryW,
        callbacks::on_name_category_w,
        extern "C" fn(u32, *const c_void)
    );
    subscribe!(
        table,
        size,
        Cb::NVTX_CBID_CORE_NameOsThreadW,
        callbacks::on_name_os_thread_w,
        extern "C" fn(u32, *const c_void)
    );
}

/// Write `func` into `table[cbid]` (the ABI stores an array of pointers-to-
/// function-pointers, so we set `*table[cbid]`), bounds-checked against `size`.
///
/// Returns `true` if the callback was installed, `false` if the slot is out of
/// range for the table's reported `size` or the slot pointer is null (a cbid the
/// running NVTX does not expose).
///
/// # Safety
/// `table` must be a valid `NvtxFunctionTable` with at least `size` slots.
unsafe fn set_callback(
    table: NvtxFunctionTable,
    size: c_uint,
    cbid: c_uint,
    func: NvtxFunctionPointer,
) -> bool {
    let index = cbid as usize;
    if index >= size as usize {
        return false;
    }
    // SAFETY: `index < size`; `table` holds that many slot pointers.
    let slot = unsafe { *table.add(index) };
    if slot.is_null() {
        return false;
    }
    // SAFETY: `slot` points to a writable `NvtxFunctionPointer` cell owned by NVTX.
    unsafe { *slot = func };
    true
}

#[cfg(test)]
mod tests {
    use super::{range_pop_level, range_push_level};

    #[test]
    fn push_and_pop_report_zero_based_nesting_levels() {
        let d = 0xD0;
        // Nested pushes report increasing levels of the range being started...
        assert_eq!(range_push_level(d), 0);
        assert_eq!(range_push_level(d), 1);
        assert_eq!(range_push_level(d), 2);
        // ...and each pop reports the level of the range being ended, unwinding.
        assert_eq!(range_pop_level(d), 2);
        assert_eq!(range_pop_level(d), 1);
        assert_eq!(range_pop_level(d), 0);
        // An unbalanced pop saturates at level 0.
        assert_eq!(range_pop_level(d), 0);
    }

    #[test]
    fn nesting_levels_are_independent_per_domain() {
        let (a, b) = (0xA1, 0xB2);
        assert_eq!(range_push_level(a), 0);
        assert_eq!(range_push_level(a), 1);
        // A different domain keeps its own independent stack.
        assert_eq!(range_push_level(b), 0);
        assert_eq!(range_pop_level(a), 1);
        assert_eq!(range_pop_level(b), 0);
    }
}
