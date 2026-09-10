// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../../../examples/readme/model.yaml");
    println!("cargo:rerun-if-changed={}", model.display());
    let schema = quent_yaml::parse_from_file(model)?.schema;
    let options = quent_schema_codegen_python::Options {
        module_name: "quent_demo".to_owned(),
        instrumentation_path: "quent_readme_example".to_owned(),
        exporters: quent_schema_codegen_python::Exporters {
            ndjson: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    quent_schema_codegen_python::write_generated_files(
        &quent_schema_codegen_python::emit(&schema, &options)?,
        &out_dir,
    )?;
    quent_schema_codegen_python::write_generated_files(
        &quent_schema_codegen_python::emit_stubs(&schema, &options)?,
        &out_dir,
    )?;
    Ok(())
}
