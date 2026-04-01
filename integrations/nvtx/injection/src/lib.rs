// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX injection library.
//!
//! Hooks all NVTX API calls and forwards them as [`nvtx_events::NvtxEvent`]
//! values through a user-provided callback.
//!
//! ```ignore
//! nvtx_injection::install_hook(|event| {
//!     println!("{event:?}");
//! });
//! ```

#[allow(
    dead_code,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    clippy::unnecessary_operation,
    clippy::identity_op
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/nvtx_bindings.rs"));
}

mod callbacks;
mod convert;
mod init;
mod symbol;

use std::sync::OnceLock;

use quent_nvtx_events::NvtxEvent;

static SENDER: OnceLock<Box<dyn Fn(NvtxEvent) + Send + Sync>> = OnceLock::new();

/// Install a hook that receives all NVTX events.
///
/// Must be called before the first NVTX API call. Events emitted before
/// `install_hook()` is called are silently dropped.
///
/// Can only be called once per process (NVTX initialization is one-shot).
pub fn install_hook(hook: impl Fn(NvtxEvent) + Send + Sync + 'static) {
    SENDER.set(Box::new(hook)).ok();
}

/// Emit an NvtxEvent through the installed hook.
/// No-op if [`install_hook()`] has not been called.
#[inline]
fn emit(event: NvtxEvent) {
    if let Some(sender) = SENDER.get() {
        sender(event);
    }
}
