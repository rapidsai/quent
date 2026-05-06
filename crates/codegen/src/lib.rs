// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Code generation from Quent model definitions.
//!
//! Takes a `ModelBuilder` (populated by derive macros or deserialized from
//! YAML/JSON) and emits target-language code.

pub mod cxx_bridge;
pub mod pyo3_bridge;
pub mod pyo3_stub;

mod common;

use quent_model::ModelBuilder;

/// Configuration for the CXX bridge backend.
pub struct CxxOptions {
    /// C++ namespace for generated types (e.g., "myapp::quent").
    pub namespace: String,
    /// The Rust crate name for CXX include paths (e.g., "quent-bridge").
    pub crate_name: String,
    /// Directory for generated bridge files, relative to CARGO_MANIFEST_DIR.
    pub bridge_path: String,
    /// Rust path to the instrumentation crate (e.g., "quent_readme_example").
    pub instrumentation_crate: String,
}

impl CxxOptions {
    /// The fully qualified event type path. Requires the model name.
    pub fn event_type(&self, model_name: &str) -> String {
        common::event_type_path(model_name, self)
    }
}

impl Default for CxxOptions {
    fn default() -> Self {
        Self {
            namespace: "quent".to_string(),
            crate_name: "quent-bridge".to_string(),
            bridge_path: "gen".to_string(),
            instrumentation_crate: "instrumentation".to_string(),
        }
    }
}

/// Generate CXX bridge Rust source code from a model.
///
/// Returns a map of filename → source code content.
pub fn emit_cxx(model: &ModelBuilder, options: &CxxOptions) -> Vec<GeneratedFile> {
    cxx_bridge::emit(model, options)
}

/// Configuration for the PyO3 bridge backend.
pub struct PyO3Options {
    /// Python extension module name (e.g., "quent_readme").
    pub module_name: String,
    /// Rust path to the instrumentation crate (e.g., "quent_readme_example").
    pub instrumentation_crate: String,
}

impl PyO3Options {
    /// The fully qualified event type path. Requires the model name.
    pub fn event_type(&self, model_name: &str) -> String {
        common::event_type_path(model_name, self)
    }
}

impl Default for PyO3Options {
    fn default() -> Self {
        Self {
            module_name: "quent_model".to_string(),
            instrumentation_crate: "instrumentation".to_string(),
        }
    }
}

/// Generate PyO3 bridge Rust source code from a model.
///
/// The generated file is intended to be included at the crate root of a PyO3
/// `cdylib` crate.
pub fn emit_pyo3(model: &ModelBuilder, options: &PyO3Options) -> Vec<GeneratedFile> {
    pyo3_bridge::emit(model, options)
}

/// Generate Python type stub files for a PyO3 bridge.
pub fn emit_pyo3_stubs(model: &ModelBuilder, options: &PyO3Options) -> Vec<GeneratedFile> {
    pyo3_stub::emit(model, options)
}

/// A generated source file.
pub struct GeneratedFile {
    /// Filename (e.g., "engine.rs").
    pub name: String,
    /// Source code content.
    pub content: String,
}

/// Write generated bridge files to disk and prepare for cxx_build.
///
/// - Writes each generated `.rs` file (except `lib.rs`) to `bridge_path`
///   under `manifest_dir`.
/// - Writes `bridge_mod.rs` to `out_dir` for `include!` in `lib.rs`.
/// - Returns the list of relative file paths to pass to `cxx_build::bridges()`.
///
/// Expects `CARGO_MANIFEST_DIR` and `OUT_DIR` env vars (available in build scripts).
pub fn write_bridge_files(files: &[GeneratedFile], options: &CxxOptions) -> Vec<String> {
    use std::fs;
    use std::path::PathBuf;

    let manifest_dir: PathBuf = std::env::var("CARGO_MANIFEST_DIR").unwrap().into();
    let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    let gen_dir = manifest_dir.join(&options.bridge_path);
    fs::create_dir_all(&gen_dir).unwrap();

    let mut bridge_files = Vec::new();
    let mut mod_lines = Vec::new();

    for file in files {
        let path = gen_dir.join(&file.name);
        fs::write(&path, &file.content).unwrap();
        bridge_files.push(format!("{}/{}", options.bridge_path, file.name));

        let mod_name = file.name.trim_end_matches(".rs");
        mod_lines.push(format!(
            "#[path = \"{}/{}\"]\npub mod {};",
            gen_dir.display(),
            file.name,
            mod_name
        ));
    }

    fs::write(out_dir.join("bridge_mod.rs"), mod_lines.join("\n")).unwrap();

    bridge_files
}

/// Write generated PyO3 bridge files to `OUT_DIR`.
///
/// The PyO3 backend currently emits one root-level Rust source file. Include it
/// from the extension crate with:
///
/// ```ignore
/// include!(concat!(env!("OUT_DIR"), "/pyo3_bridge.rs"));
/// ```
pub fn write_pyo3_files(files: &[GeneratedFile]) {
    use std::fs;
    use std::path::PathBuf;

    let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    for file in files {
        fs::write(out_dir.join(&file.name), &file.content).unwrap();
    }
}

/// Write generated files to a specific directory.
pub fn write_generated_files(files: &[GeneratedFile], dir: impl AsRef<std::path::Path>) {
    use std::fs;

    let dir = dir.as_ref();
    fs::create_dir_all(dir).unwrap();
    for file in files {
        let path = dir.join(&file.name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, &file.content).unwrap();
    }
}

/// Copy CXX-generated headers to `include/` under the crate root.
///
/// Call this *after* `cxx_build::bridges().compile()` so the headers exist.
/// The copied headers can then be referenced by CMake via
/// `target_include_directories`.
pub fn copy_cxx_headers() {
    use std::path::PathBuf;

    let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    let manifest_dir: PathBuf = std::env::var("CARGO_MANIFEST_DIR").unwrap().into();
    let cxx_include = out_dir.join("cxxbridge").join("include");
    let header_dir = manifest_dir.join("include");
    if cxx_include.exists() {
        copy_dir_recursive(&cxx_include, &header_dir);
    }
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) {
    use std::fs;
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest);
        } else {
            fs::copy(&path, &dest).unwrap();
        }
    }
}
