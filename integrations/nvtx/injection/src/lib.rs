// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Quent-agnostic NVTX injection cdylib.
//!
//! NVTX loads this library via `NVTX_INJECTION64_PATH` and calls the exported
//! [`InitializeInjectionNvtx2`](crate::InitializeInjectionNvtx2) entry, which
//! installs the CORE/CORE2 callback tables one-shot (D-15). Push/pop calls are
//! converted to verbatim [`NvtxEvent`](quent_nvtx_events::NvtxEvent)s and handed
//! to a sink-agnostic `Fn(NvtxEvent)` hook installed via
//! [`install_hook`](crate::install_hook) (D-03). This crate depends on nothing
//! Quent-internal except `quent-nvtx-events`, so it stays separable/upstreamable.

// D-04: Linux 64-bit only. NVTX injection relies on the ELF weak-symbol /
// NVTX_INJECTION64_PATH mechanism; Windows and 32-bit are out of scope.
#[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
compile_error!("quent-nvtx-injection supports Linux 64-bit only");

mod bindings;
mod callbacks;
mod convert;
mod init;

pub use init::{InitializeInjectionNvtx2, InstallHookError, install_hook};
