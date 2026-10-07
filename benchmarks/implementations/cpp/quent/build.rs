// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("OUT_DIR is missing")?);
    let mut bridges = vec![PathBuf::from("src/main.rs")];
    let mut options = Vec::new();
    let mut modules = String::new();
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
        let config = quent_schema_codegen_cpp::Options {
            namespace: format!("quent_bench::{name}"),
            crate_name: env!("CARGO_PKG_NAME").to_owned(),
            bridge_path: format!("gen/{name}"),
            instrumentation_path: format!("crate::models::{name}"),
            exporters: quent_schema_codegen_cpp::Exporters {
                ndjson: true,
                msgpack: true,
                postcard: true,
                collector: false,
            },
            ..Default::default()
        };
        let files = quent_schema_codegen_cpp::emit(&parsed.schema, &config)?;
        bridges.extend(quent_schema_codegen_cpp::write_bridge_files(
            &files, &config,
        )?);
        // Each generator invocation replaces the shared module include file.
        std::fs::copy(
            out.join("bridge_mod.rs"),
            out.join(format!("{name}_bridge.rs")),
        )?;
        modules.push_str(&format!(
            "pub mod {name} {{ include!(concat!(env!(\"OUT_DIR\"), \"/{name}.rs\")); }}\n\
             mod {name}_bridge {{ include!(concat!(env!(\"OUT_DIR\"), \"/{name}_bridge.rs\")); }}\n"
        ));
        options.push(config);
    }
    std::fs::write(out.join("models.rs"), modules)?;
    let mut build = cxx_build::bridges(bridges);
    for config in &options {
        build.include(quent_schema_codegen_cpp::stage_cxx_headers(config)?);
    }
    build
        .include("include")
        .file("src/measure.cpp")
        .std("c++20")
        .compile("quent_bench_cpp_quent");
    for path in [
        "build.rs",
        "src/main.rs",
        "src/measure.cpp",
        "include",
        "../common/include",
        "../common/src/lib.rs",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    Ok(())
}
