// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_codegen::CxxOptions;

fn main() {
    let builder = quent_qe_cpp_instrumentation::QueryEngineModel::build("QueryEngine");

    let options = CxxOptions {
        crate_name: "quent-qe-bridge".into(),
        instrumentation_crate: "quent_qe_cpp_instrumentation".into(),
        ..Default::default()
    };

    let files = quent_codegen::emit_cxx(&builder, &options);
    let bridge_files = quent_codegen::write_bridge_files(&files, &options);

    cxx_build::bridges(bridge_files)
        .std("c++20")
        .compile("quent_qe_bridge");

    quent_codegen::copy_cxx_headers();
}
