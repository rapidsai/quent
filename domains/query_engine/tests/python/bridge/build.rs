// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_codegen::PyO3Options;

fn main() {
    let builder = quent_qe_python_instrumentation::QueryEngineModel::build("QueryEngine");

    let options = PyO3Options {
        module_name: "quent_qe".into(),
        instrumentation_crate: "quent_qe_python_instrumentation".into(),
    };

    let files = quent_codegen::emit_pyo3(&builder, &options);
    quent_codegen::write_pyo3_files(&files);

    let stub_files = quent_codegen::emit_pyo3_stubs(&builder, &options);
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    quent_codegen::write_generated_files(&stub_files, out_dir);
}
