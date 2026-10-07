// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pyo3_build_config::add_extension_module_link_args();
    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("OUT_DIR is missing")?);
    let mut modules = String::new();
    let mut registration = String::new();
    for name in ["empty", "u8", "u64", "short_string", "long_string", "all"] {
        let model = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../models")
            .join(format!("{name}.yaml"));
        println!("cargo:rerun-if-changed={}", model.display());
        let parsed = quent_yaml::parse_from_file(model)?;
        for warning in parsed.warnings {
            println!("cargo:warning={warning}");
        }
        quent_instrumentation_build::generate(
            &parsed.schema,
            &quent_instrumentation_build::Options {
                file_name: Some(format!("{name}.rs")),
                serde: true,
                ..Default::default()
            },
        )?;
        let options = quent_schema_codegen_python::Options {
            module_name: format!("quent_bench_python.{name}"),
            instrumentation_path: format!("crate::models::{name}"),
            exporters: quent_schema_codegen_python::Exporters {
                ndjson: true,
                msgpack: true,
                postcard: true,
                collector: false,
            },
            ..Default::default()
        };
        quent_schema_codegen_python::write_generated_files(
            &quent_schema_codegen_python::emit(&parsed.schema, &options)?,
            out.join(name),
        )?;
        modules.push_str(&format!(
            "pub mod {name} {{ include!(concat!(env!(\"OUT_DIR\"), \"/{name}.rs\")); }}\n\
             mod {name}_bridge {{\n\
                 use pyo3::prelude::*;\n\
                 include!(concat!(env!(\"OUT_DIR\"), \"/{name}/pyo3_bridge.rs\"));\n\
                 pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {{\n\
                     let child = pyo3::wrap_pymodule!(__quent_pyo3_bridge::quent_bench_python_{name})(parent.py());\n\
                     parent.add_submodule(child.bind(parent.py()))\n\
                 }}\n\
             }}\n"
        ));
        registration.push_str(&format!("{name}_bridge::register(module)?;\n"));
    }
    modules.push_str(&format!(
        "pub fn register(module: &pyo3::Bound<'_, pyo3::types::PyModule>) -> pyo3::PyResult<()> {{\n\
         {registration}Ok(())\n}}\n"
    ));
    std::fs::write(out.join("models.rs"), modules)?;
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
