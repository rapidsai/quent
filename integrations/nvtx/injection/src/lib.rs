// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Application-agnostic NVTX injection library.
//!
//! On attach, NVTX calls the exported [`InitializeInjectionNvtx2`] entry, which
//! installs the CORE/CORE2 callback tables one-shot. Push/pop calls are
//! converted to verbatim [`NvtxEvent`](nvtx_events::NvtxEvent)s and handed to a
//! sink-agnostic `Fn(NvtxEvent)` hook installed via [`install_hook`]. This
//! crate depends on nothing
//! product-specific except `nvtx-events`, so it stays separable/upstreamable.
//!
//! # Attach modes
//!
//! - **Runtime (default):** built as a cdylib; NVTX `dlopen`s it via
//!   `NVTX_INJECTION64_PATH` and calls the injection entry.
//! - **In-process (`static-injection` feature):** a strong
//!   `InitializeInjectionNvtx2_fnptr` is linked into the consuming image and
//!   points at Quent's internal initializer, overriding NVTX's weak pointer so
//!   that image initializes injection at its first NVTX call.

// Linux 64-bit only. NVTX injection relies on the ELF weak-symbol /
// NVTX_INJECTION64_PATH mechanism; Windows and 32-bit are out of scope.
#[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
compile_error!("nvtx-injection supports Linux 64-bit only");

// NVTX's injection ABI has the same layout on x86_64 and aarch64 Linux:
// both targets use 64-bit pointers/size_t and 32-bit C enums/wchar_t. Keep these
// assertions in every build so a target-specific representation change fails
// at compile time instead of silently changing a callback boundary.
const _: () = {
    use std::mem::{align_of, offset_of, size_of};

    use nvtx_sys::ffi;

    assert!(size_of::<ffi::wchar_t>() == 4);
    assert!(size_of::<ffi::NvtxCallbackModule>() == 4);
    assert!(size_of::<ffi::NvtxFunctionPointer>() == 8);
    assert!(size_of::<ffi::NvtxGetExportTableFunc_t>() == 8);

    assert!(size_of::<ffi::nvtxMessageValue_t>() == 8);
    assert!(align_of::<ffi::nvtxMessageValue_t>() == 8);
    assert!(size_of::<ffi::nvtxEventAttributes_v2_payload_t>() == 8);
    assert!(align_of::<ffi::nvtxEventAttributes_v2_payload_t>() == 8);

    assert!(size_of::<ffi::nvtxEventAttributes_t>() == 48);
    assert!(align_of::<ffi::nvtxEventAttributes_t>() == 8);
    assert!(offset_of!(ffi::nvtxEventAttributes_t, payload) == 24);
    assert!(offset_of!(ffi::nvtxEventAttributes_t, messageType) == 32);
    assert!(offset_of!(ffi::nvtxEventAttributes_t, message) == 40);

    assert!(size_of::<ffi::nvtxResourceAttributes_v0_identifier_t>() == 8);
    assert!(align_of::<ffi::nvtxResourceAttributes_v0_identifier_t>() == 8);
    assert!(size_of::<ffi::nvtxResourceAttributes_t>() == 32);
    assert!(align_of::<ffi::nvtxResourceAttributes_t>() == 8);
    assert!(offset_of!(ffi::nvtxResourceAttributes_t, identifier) == 8);
    assert!(offset_of!(ffi::nvtxResourceAttributes_t, messageType) == 16);
    assert!(offset_of!(ffi::nvtxResourceAttributes_t, message) == 24);

    assert!(size_of::<ffi::NvtxExportTableCallbacks>() == 16);
    assert!(align_of::<ffi::NvtxExportTableCallbacks>() == 8);
    assert!(offset_of!(ffi::NvtxExportTableCallbacks, GetModuleFunctionTable) == 8);
};
mod callbacks;
mod convert;
mod init;

pub use init::{
    InstallHookError, initialize_injection_nvtx2 as InitializeInjectionNvtx2, install_hook,
};
