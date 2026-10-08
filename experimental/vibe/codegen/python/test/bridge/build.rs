// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../../../examples/readme/model.yaml");
    println!("cargo:rerun-if-changed={}", model.display());
    let schema = quent_yaml::parse_from_file(model)?.schema;
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    quent_instrumentation_build::generate(
        &schema,
        &quent_instrumentation_build::Options {
            serde: true,
            collector_sink: true,
            out_dir: out_dir.clone(),
            file_name: Some("model.rs".to_owned()),
            ..Default::default()
        },
    )?;
    let options = quent_schema_codegen_python::Options {
        module_name: "quent_codegen_test".to_owned(),
        instrumentation_path: "crate".to_owned(),
        exporters: quent_schema_codegen_python::Exporters {
            ndjson: true,
            collector: true,
            ..Default::default()
        },
        collector_server: true,
        ..Default::default()
    };
    quent_schema_codegen_python::write_generated_files(
        &quent_schema_codegen_python::emit(&schema, &options)?,
        &out_dir,
    )?;
    Ok(())
}
