// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let vendor_include = format!("{manifest_dir}/../vendor/nvtx-repo/c/include");

    // Compile the C file that provides the strong InitializeInjectionNvtx2_fnptr symbol.
    cc::Build::new()
        .file(format!("{manifest_dir}/c/symbol.c"))
        .compile("quent_nvtx_symbol");

    // The static injection library must be linked with --whole-archive / -force_load
    // so the strong symbol overrides NVTX's weak definition.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-force_load,{out_dir}/libquent_nvtx_symbol.a");
    } else {
        println!("cargo:rustc-link-arg=-Wl,--whole-archive");
        println!("cargo:rustc-link-arg={out_dir}/libquent_nvtx_symbol.a");
        println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
    }

    // Generate Rust bindings from vendored NVTX headers.
    let bindings = bindgen::Builder::default()
        .header(format!("{manifest_dir}/wrapper.h"))
        .clang_arg(format!("-I{vendor_include}"))
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

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("nvtx_bindings.rs"))
        .expect("Failed to write NVTX bindings");
}
