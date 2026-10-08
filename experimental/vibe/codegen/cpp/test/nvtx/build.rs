// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("CARGO_CFG_TARGET_OS")? != "linux"
        || std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH")? != "64"
    {
        println!(
            "cargo:warning=compiled NVTX fixture requires Linux64; capture tests are not built"
        );
        return Ok(());
    }
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let model = manifest.join("../../../../../../examples/readme/model.yaml");
    println!("cargo:rerun-if-changed={}", model.display());
    println!("cargo:rerun-if-changed=test.cpp");
    let schema = quent_yaml::parse_from_file(model)?.schema;
    let options = quent_schema_codegen_cpp::Options {
        crate_name: "quent-nvtx-cpp-test".to_owned(),
        instrumentation_path: "quent_readme_example".to_owned(),
        exporters: quent_schema_codegen_cpp::Exporters::all(),
        nvtx: quent_schema_codegen_cpp::NvtxSupport::Enabled,
        ..Default::default()
    };
    let files = quent_schema_codegen_cpp::emit(&schema, &options)?;
    let bridges = quent_schema_codegen_cpp::write_bridge_files(&files, &options)?;
    let mut build = cxx_build::bridges(bridges);
    let includes = quent_schema_codegen_cpp::stage_cxx_headers(&options)?;
    build
        .include(includes)
        .file(manifest.join("test.cpp"))
        .std("c++20")
        .compile("quent_nvtx_cpp_test");
    Ok(())
}
