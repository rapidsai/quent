// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Deliver NVTX calls to a Rust hook as owned [`Record`] values.
//!
//! Register a process-lifetime hook with [`install_hook`]. Once NVTX initializes
//! the linked injection code, each supported NVTX call copies borrowed data into an
//! [`Record`] and invokes the hook on the calling thread. The consumer decides
//! how to interpret event text and payloads. The library also returns the
//! nesting levels and opaque handles required by NVTX calls.
//!
//! # In-process setup
//!
//! Enable the `static-injection` feature and link this crate into the application.
//! Runtime loading through `NVTX_INJECTION64_PATH` is not supported because a
//! separately loaded library would not share the application's installed hook.
//!
//! # Example
//!
//! ```
//! # fn main() -> Result<(), nvtx_injection::InstallHookError> {
//! let (tx, rx) = std::sync::mpsc::channel();
//! nvtx_injection::install_hook(move |event| {
//!     let _ = tx.send(event);
//! })?;
//! // `rx` receives owned events as NVTX calls occur.
//! # drop(rx);
//! # Ok(())
//! # }
//! ```
//!
//! If push/pop processing needs the caller's thread ID, capture it in the hook
//! before forwarding the event to another thread.

// Linux 64-bit only. Static injection relies on an ELF weak-symbol override;
// Windows and 32-bit are out of scope.
#[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
compile_error!("nvtx-injection supports Linux 64-bit only");

mod callbacks;
mod init;
pub mod record;

pub use record::Record;

pub use init::{
    InstallHookError, initialize_injection_nvtx2 as InitializeInjectionNvtx2, install_hook,
};
