// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("CARGO_CFG_TARGET_OS")? != "linux"
        || std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH")? != "64"
    {
        return Ok(());
    }
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rerun-if-changed=model.yaml");
    println!("cargo:rerun-if-changed=test.cpp");
    let schema = quent_yaml::parse_from_file(manifest.join("model.yaml"))?.schema;
    quent_instrumentation_build::generate(
        &schema,
        &quent_instrumentation_build::Options {
            events: quent_instrumentation_build::EventOptions {
                serde: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    let options = quent_schema_codegen_cpp::Options {
        crate_name: "quent-nvtx-cpp-test".to_owned(),
        instrumentation_path: "crate::instrumentation".to_owned(),
        exporters: quent_schema_codegen_cpp::Exporters {
            ndjson: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let files = quent_schema_codegen_cpp::emit(&schema, &options)?;
    let bridges = quent_schema_codegen_cpp::write_bridge_files(&files, &options)?;
    cxx_build::bridges(bridges)
        .include(quent_schema_codegen_cpp::stage_cxx_headers(&options)?)
        .file(manifest.join("test.cpp"))
        .std("c++20")
        .compile("quent_nvtx_cpp_test");
    Ok(())
}
