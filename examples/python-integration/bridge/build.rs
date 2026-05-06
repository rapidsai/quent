// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_codegen::PyO3Options;

fn main() {
    let builder = quent_readme_example::AppModel::build("App");

    let options = PyO3Options {
        module_name: "quent_readme".into(),
        instrumentation_crate: "quent_readme_example".into(),
    };

    let files = quent_codegen::emit_pyo3(&builder, &options);
    quent_codegen::write_pyo3_files(&files);

    let stub_files = quent_codegen::emit_pyo3_stubs(&builder, &options);
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let project_dir = manifest_dir
        .parent()
        .expect("bridge crate should live below the Python project directory");
    quent_codegen::write_generated_files(&stub_files, project_dir);
}
