// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Routes NVTX events to a process-wide hook.
//!
//! The hook stays installed for the process lifetime. Consumers that need to
//! shut down can capture a channel sender and release its receiver on shutdown.

use std::sync::OnceLock;

use crate::RawEvent;
use thiserror::Error;

/// Receives owned NVTX records on the calling thread.
pub(crate) type Hook = dyn Fn(RawEvent) + Send + Sync + 'static;

static HOOK: OnceLock<Box<Hook>> = OnceLock::new();

/// Error returned by [`install_hook`].
#[derive(Debug, Error)]
pub enum InstallHookError {
    /// A hook was already installed. Installation is one-shot per process.
    #[error("an NVTX capture hook is already installed (install_hook is one-shot per process)")]
    AlreadyInstalled,
}

/// Install a hook that receives owned NVTX records for the process lifetime.
///
/// The hook runs synchronously on the emitting thread and may be called
/// concurrently. Hooks must not emit NVTX events synchronously, including
/// through code they call, because this recursively invokes the hook. Captured
/// values remain owned by the hook until process exit.
///
/// # Errors
/// Returns [`InstallHookError::AlreadyInstalled`] if a hook was already set.
pub fn install_hook<F>(hook: F) -> Result<(), InstallHookError>
where
    F: Fn(RawEvent) + Send + Sync + 'static,
{
    HOOK.set(Box::new(hook))
        .map_err(|_| InstallHookError::AlreadyInstalled)
}

/// Returns the installed hook, which remains valid for the process lifetime.
#[inline]
pub(crate) fn hook() -> Option<&'static Hook> {
    HOOK.get().map(Box::as_ref)
}
