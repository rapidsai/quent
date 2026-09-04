// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../../../examples/readme/model.yaml");
    println!("cargo:rerun-if-changed={}", model.display());
    let schema = quent_yaml::parse_from_file(model)?.schema;
    let options = quent_schema_codegen_cpp::Options {
        crate_name: "quent-demo-cpp-bridge".to_owned(),
        instrumentation_path: "quent_readme_example".to_owned(),
        exporters: quent_schema_codegen_cpp::Exporters {
            ndjson: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let files = quent_schema_codegen_cpp::emit(&schema, &options)?;
    let bridges = quent_schema_codegen_cpp::write_bridge_files(&files, &options)?;
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../main.cpp");
    println!("cargo:rerun-if-changed={}", example.display());
    let mut build = cxx_build::bridges(bridges);
    let include_dir = quent_schema_codegen_cpp::stage_cxx_headers(&options)?;
    build
        .include(&include_dir)
        .file(example)
        .define("QUENT_DEMO_LIBRARY", None)
        .std("c++20")
        .compile("quent_demo_cpp_bridge");
    println!("cargo:include={}", include_dir.display());
    Ok(())
}
