// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

const MODEL: &str = r#"
quent: '0.1.0'
model: cpp_list_test

records:
  Item:
    fields:
      value: u32

entities:
  Batch:
    events:
      recorded:
        attributes:
          flags: { list: bool }
          ids: { list: uuid }
          items: { list: Item }
          extras: { list: dynamic }
          matrix: { list: { list: u16 } }
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = quent_yaml::parse_from_str(MODEL, None)?.schema;
    quent_instrumentation_build::generate(
        &schema,
        &quent_instrumentation_build::Options {
            file_name: Some("instrumentation.rs".to_owned()),
            ..Default::default()
        },
    )?;
    let options = quent_schema_codegen_cpp::Options {
        crate_name: "quent-cpp-list-test".to_owned(),
        instrumentation_path: "crate::instrumentation".to_owned(),
        ..Default::default()
    };
    let files = quent_schema_codegen_cpp::emit(&schema, &options)?;
    let bridges = quent_schema_codegen_cpp::write_bridge_files(&files, &options)?;
    let mut build = cxx_build::bridges(bridges);
    let include_dir = quent_schema_codegen_cpp::stage_cxx_headers(&options)?;
    build
        .include(include_dir)
        .file("test.cpp")
        .std("c++20")
        .compile("quent_cpp_list_test");
    Ok(())
}
