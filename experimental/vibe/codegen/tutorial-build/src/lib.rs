// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared build support for the generated C++ and Python tutorials.

use std::path::{Path, PathBuf};

use quent_instrumentation_build::{GenerateError, Options, generate};

/// Generates and compiles a C++ tutorial bridge for the canonical YAML tutorial.
pub fn build_cpp(tutorial: &str) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = manifest_dir()?;
    let schema = load_schema(&manifest_dir, tutorial)?;
    generate_instrumentation(&schema)?;

    let package = std::env::var("CARGO_PKG_NAME")?;
    let options = quent_schema_codegen_cpp::Options {
        crate_name: package.clone(),
        instrumentation_path: "crate::instrumentation".to_owned(),
        ..Default::default()
    };
    let files = quent_schema_codegen_cpp::emit(&schema, &options)?;
    let bridges = quent_schema_codegen_cpp::write_bridge_files(&files, &options)?;
    let example = manifest_dir.join("../main.cpp");
    println!("cargo:rerun-if-changed={}", example.display());
    let mut build = cxx_build::bridges(bridges);
    let include_dir = quent_schema_codegen_cpp::stage_cxx_headers(&options)?;
    build
        .include(&include_dir)
        .file(example)
        .std("c++20")
        .compile(&format!("{}_native", package.replace('-', "_")));
    println!("cargo:include={}", include_dir.display());
    Ok(())
}

/// Generates a Python tutorial bridge and its type stubs for the canonical YAML tutorial.
pub fn build_python(tutorial: &str, module_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = manifest_dir()?;
    let schema = load_schema(&manifest_dir, tutorial)?;
    generate_instrumentation(&schema)?;

    let options = quent_schema_codegen_python::Options {
        module_name: module_name.to_owned(),
        instrumentation_path: "crate::instrumentation".to_owned(),
        ..Default::default()
    };
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
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

fn manifest_dir() -> Result<PathBuf, std::env::VarError> {
    std::env::var("CARGO_MANIFEST_DIR").map(PathBuf::from)
}

fn load_schema(
    manifest_dir: &Path,
    tutorial: &str,
) -> Result<quent_schema_codegen_cpp::quent_schema::Schema, Box<dyn std::error::Error>> {
    let repository = repository_root(manifest_dir).ok_or("could not locate the repository root")?;
    let model = repository
        .join("crates/yaml/examples")
        .join(tutorial)
        .join("model.yaml");
    println!("cargo:rerun-if-changed={}", model.display());
    let parsed = quent_yaml::parse_from_file(model)?;
    for warning in parsed.warnings {
        println!("cargo:warning={warning}");
    }
    Ok(parsed.schema)
}

fn repository_root(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|path| path.join("crates/yaml/examples").is_dir())
}

fn generate_instrumentation(
    schema: &quent_schema_codegen_cpp::quent_schema::Schema,
) -> Result<(), GenerateError> {
    generate(
        schema,
        &Options {
            file_name: Some("instrumentation.rs".to_owned()),
            ..Default::default()
        },
    )
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn language_tutorials_mirror_canonical_yaml_tutorials() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repository = repository_root(manifest_dir).unwrap();
        let expected = tutorial_names(&repository.join("crates/yaml/examples"));

        for language in ["cpp", "python"] {
            let tutorials = repository
                .join("experimental/vibe/codegen")
                .join(language)
                .join("example/tutorial");
            assert_eq!(
                directory_names(&tutorials),
                expected,
                "{language} tutorials"
            );
            for name in &expected {
                assert!(!tutorials.join(name).join("model.yaml").exists());
            }
        }
    }

    fn tutorial_names(root: &Path) -> BTreeSet<String> {
        std::fs::read_dir(root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().join("model.yaml").is_file())
            .map(|entry| entry.file_name().into_string().unwrap())
            .collect()
    }

    fn directory_names(root: &Path) -> BTreeSet<String> {
        std::fs::read_dir(root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().into_string().unwrap())
            .collect()
    }
}
