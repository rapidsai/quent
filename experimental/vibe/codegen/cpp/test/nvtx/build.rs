// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let model = manifest_dir.join("../../../test/nvtx/model.yaml");
    println!("cargo:rerun-if-changed={}", model.display());
    println!("cargo:rerun-if-changed=test.cpp");

    let schema = quent_yaml::parse_from_file(model)?.schema;
    let options = quent_schema_codegen_cpp::Options {
        crate_name: env!("CARGO_PKG_NAME").to_owned(),
        instrumentation_path: "quent_codegen_nvtx_test_instrumentation::model".to_owned(),
        exporters: quent_schema_codegen_cpp::Exporters {
            ndjson: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let files = quent_schema_codegen_cpp::emit(&schema, &options)?;
    let bridges = quent_schema_codegen_cpp::write_bridge_files(&files, &options)?;
    let mut build = cxx_build::bridges(bridges);
    let include_dir = quent_schema_codegen_cpp::stage_cxx_headers(&options)?;
    build.include(include_dir).file("test.cpp").std("c++20");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux")
        && std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH").as_deref() == Ok("64")
    {
        build.define("QUENT_NVTX_LIVE_CAPTURE", None);
    }
    build.compile("quent_codegen_cpp_nvtx_test");
    Ok(())
}
