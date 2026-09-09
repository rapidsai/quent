// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build script for `nvtx-injection`.
//!
//! Ordinary builds only validate the supported target. `nvtx-sys` owns NVTX's
//! generated ABI bindings; the `static-injection` feature additionally compiles
//! Quent's strong-symbol C shim.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // NVTX injection relies on ELF weak-symbol override / NVTX_INJECTION64_PATH,
    // which is Linux 64-bit only. ARM Linux (aarch64) is supported — gettid and
    // the ELF mechanism work there too.
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let bits = std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap_or_default();
    if os != "linux" || bits != "64" {
        return Err(format!(
            "nvtx-injection supports Linux 64-bit only (got os={os}, pointer_width={bits})"
        )
        .into());
    }

    #[cfg(feature = "static-injection")]
    compile_symbol_shim();

    Ok(())
}

/// Compile the strong-symbol C shim for the static-injection attach path.
///
/// The shim's *strong* `InitializeInjectionNvtx2_fnptr` overrides NVTX's *weak*
/// one, but only if its object is linked in. Nothing references it from Rust and
/// `-u` would bind to NVTX's weak def, so `+whole-archive` forces it in —
/// otherwise the linker drops the override and injection never initializes.
#[cfg(feature = "static-injection")]
fn compile_symbol_shim() {
    println!("cargo::rerun-if-changed=c/symbol.c");
    cc::Build::new()
        .file("c/symbol.c")
        .link_lib_modifier("+whole-archive")
        .compile("nvtx_symbol");
}
