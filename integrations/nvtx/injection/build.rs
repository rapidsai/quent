// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build script for `nvtx-injection`.
//!
//! Runs `bindgen` over the NVTX injection ABI and writes the result to
//! `$OUT_DIR/bindings.rs`, which `lib.rs` `include!`s. The NVTX headers and
//! `libclang` come from the pixi-pinned `nvtx-c` / `libclang` packages, so a
//! `pixi run cargo …` build is hermetic and needs nothing checked in.
//! The `static-injection` feature additionally compiles the strong-symbol C
//! shim.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_bindings()?;

    #[cfg(feature = "static-injection")]
    compile_symbol_shim();

    Ok(())
}

/// Generate the NVTX injection bindings into `$OUT_DIR/bindings.rs`.
///
/// Headers resolve from the active pixi environment (`$CONDA_PREFIX/include`,
/// populated by the `nvtx-c` package); `libclang` is located under
/// `$CONDA_PREFIX/lib`. Both are pinned in `pixi.toml` for `linux-64`, the only
/// target this crate supports.
fn generate_bindings() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=wrapper.h");
    println!("cargo::rerun-if-env-changed=CONDA_PREFIX");
    println!("cargo::rerun-if-env-changed=LIBCLANG_PATH");

    let prefix = PathBuf::from(std::env::var("CONDA_PREFIX").map_err(|_| {
        "CONDA_PREFIX is unset — build this crate inside the pixi env (e.g. \
         `pixi run cargo build -p nvtx-injection`) so the pinned nvtx-c \
         headers and libclang are on the path"
    })?);

    // Point clang-sys at the pixi-provided libclang unless the caller already
    // pinned one, so bindgen never falls back to a system LLVM.
    if std::env::var_os("LIBCLANG_PATH").is_none() {
        // SAFETY: single-threaded build-script context; no other thread reads
        // the environment concurrently.
        unsafe { std::env::set_var("LIBCLANG_PATH", prefix.join("lib")) };
    }

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", prefix.join("include").display()))
        .allowlist_type("nvtxEventAttributes_t")
        .allowlist_type("nvtxEventAttributes_v2")
        .allowlist_type("nvtxMessageValue_t")
        .allowlist_type("nvtxDomainHandle_t")
        .allowlist_type("nvtxDomainRegistration")
        .allowlist_type("nvtxStringHandle_t")
        .allowlist_type("nvtxStringRegistration")
        .allowlist_type("NvtxExportTableCallbacks")
        .allowlist_type("NvtxExportTableID")
        .allowlist_type("NvtxCallbackModule")
        .allowlist_type("NvtxCallbackIdCore")
        .allowlist_type("NvtxCallbackIdCore2")
        .allowlist_type("NvtxGetExportTableFunc_t")
        .allowlist_type("NvtxFunctionTable")
        .allowlist_type("NvtxFunctionPointer")
        .allowlist_type("nvtxPayloadType_t")
        .allowlist_type("nvtxMessageType_t")
        .allowlist_type("nvtxColorType_t")
        // The resource / range-id ABI surface the CORE2 callbacks receive. These
        // were hand-declared before autogen; bindgen now owns them.
        .allowlist_type("nvtxResourceAttributes_t")
        .allowlist_type("nvtxResourceAttributes_v0")
        .allowlist_type("nvtxResourceHandle_t")
        .allowlist_type("nvtxRangeId_t")
        .allowlist_var("NVTX_.*")
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .layout_tests(false)
        .generate_comments(false)
        .generate()?;

    let out = PathBuf::from(std::env::var("OUT_DIR")?).join("bindings.rs");
    bindings.write_to_file(&out)?;
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
