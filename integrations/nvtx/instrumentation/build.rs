// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build script for `quent-nvtx`.
//!
//! Default builds do nothing but declare the C source dependency. Under the
//! `e2e` feature it compiles `c/emit.c` — a tiny NVTX v3 client shim used by the
//! deterministic `nvtx_test_app` — against the NVTX headers. Header sourcing
//! mirrors `quent-nvtx-injection/build.rs`: the `NVTX_INCLUDE_DIR` env override
//! if set, otherwise the pinned `nvidia-nvtx` git-dep checkout located via
//! `cargo_metadata` (`<checkout>/c/include`, D-13).

fn main() {
    println!("cargo::rerun-if-changed=c/emit.c");

    #[cfg(feature = "e2e")]
    compile_emit_shim();
}

/// Compile the NVTX client emitter shim for the `nvtx_test_app` binary.
#[cfg(feature = "e2e")]
fn compile_emit_shim() {
    println!("cargo::rerun-if-env-changed=NVTX_INCLUDE_DIR");
    let include = nvtx_include_dir();
    cc::Build::new()
        .file("c/emit.c")
        .include(&include)
        .compile("quent_nvtx_emit");
}

/// Resolve the directory that contains `nvtx3/nvToolsExt.h`.
#[cfg(feature = "e2e")]
fn nvtx_include_dir() -> std::path::PathBuf {
    use std::path::PathBuf;

    if let Ok(dir) = std::env::var("NVTX_INCLUDE_DIR") {
        return PathBuf::from(dir);
    }

    // `nvidia-nvtx` is an *optional* build-dep of `quent-nvtx-injection` (gated
    // behind its `regenerate-bindings` feature), so it only appears in the
    // resolve graph with features activated. Ask for all features so the pinned
    // checkout shows up; this does not build anything.
    let metadata = cargo_metadata::MetadataCommand::new()
        .features(cargo_metadata::CargoOpt::AllFeatures)
        .exec()
        .expect("run `cargo metadata` to locate the nvidia-nvtx headers (e2e)");
    let pkg = metadata
        .packages
        .iter()
        .find(|p| p.name.as_str() == "nvidia-nvtx")
        .expect(
            "nvidia-nvtx git-dep present in the workspace graph (declared by quent-nvtx-injection)",
        );
    // manifest_path: <checkout>/rust/Cargo.toml → repo root two levels up.
    let repo_root = pkg
        .manifest_path
        .parent()
        .and_then(|p| p.parent())
        .expect("unexpected nvidia-nvtx manifest layout");
    repo_root.as_std_path().join("c/include")
}
