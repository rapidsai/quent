// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

/// Find the NVTX C include directory by locating the `nvtx-sys` crate source
/// in the workspace dependency graph (pulled from the NVIDIA/NVTX git repo).
/// The C headers live at `<nvtx-repo>/c/include` relative to nvtx-sys's
/// manifest at `<nvtx-repo>/rust/crates/nvtx-sys/Cargo.toml`.
fn find_nvtx_include() -> PathBuf {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("failed to run cargo metadata");

    let nvtx_sys = metadata
        .packages
        .iter()
        .find(|p| p.name == "nvtx-sys")
        .expect("nvtx-sys not found in dependency graph");

    // nvtx-sys manifest is at <repo>/rust/crates/nvtx-sys/Cargo.toml
    // NVTX C headers are at <repo>/c/include
    let nvtx_sys_dir = nvtx_sys
        .manifest_path
        .parent()
        .expect("nvtx-sys has no parent dir");
    let nvtx_repo_root = nvtx_sys_dir
        .join("../../..")
        .canonicalize()
        .expect("failed to resolve NVTX repo root");
    let include_dir = nvtx_repo_root.join("c/include");

    assert!(
        include_dir.join("nvtx3/nvToolsExt.h").exists(),
        "NVTX headers not found at {}",
        include_dir.display()
    );

    include_dir.into()
}

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let nvtx_include = find_nvtx_include();

    // Compile the C file that provides the strong InitializeInjectionNvtx2_fnptr symbol.
    cc::Build::new()
        .file(format!("{manifest_dir}/c/symbol.c"))
        .compile("quent_nvtx_symbol");

    // Force-load the archive so the linker includes our strong
    // InitializeInjectionNvtx2_fnptr symbol, overriding the weak
    // definition from nvtx-sys in the final binary.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-force_load,{out_dir}/libquent_nvtx_symbol.a");
    } else {
        println!("cargo:rustc-link-arg=-Wl,--whole-archive");
        println!("cargo:rustc-link-arg={out_dir}/libquent_nvtx_symbol.a");
        println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
    }

    // Generate Rust bindings from NVTX headers.
    let bindings = bindgen::Builder::default()
        .header(format!("{manifest_dir}/wrapper.h"))
        .clang_arg(format!("-I{}", nvtx_include.display()))
        // Public types
        .allowlist_type("nvtxEventAttributes_v2")
        .allowlist_type("nvtxResourceAttributes_v0")
        .allowlist_type("nvtxMessageValue_t")
        // Injection internal types
        .allowlist_type("NvtxExportTableCallbacks")
        .allowlist_type("NvtxCallbackModule")
        .allowlist_type("NvtxCallbackIdCore")
        .allowlist_type("NvtxCallbackIdCore2")
        // Constants
        .allowlist_var("NVTX_VERSION")
        .allowlist_var("NVTX_COLOR_.*")
        .allowlist_var("NVTX_MESSAGE_.*")
        .allowlist_var("NVTX_PAYLOAD_TYPE_.*")
        .allowlist_var("NVTX_RESOURCE_TYPE_.*")
        .allowlist_var("NVTX_ETID_.*")
        .allowlist_var("NVTX_CB_MODULE_.*")
        .allowlist_var("NVTX_CBID_.*")
        .derive_debug(true)
        .derive_default(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate NVTX bindings");

    let out_path = PathBuf::from(&out_dir);
    bindings
        .write_to_file(out_path.join("nvtx_bindings.rs"))
        .expect("Failed to write NVTX bindings");
}
