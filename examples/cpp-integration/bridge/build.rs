// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_codegen::CxxOptions;

fn main() {
    let builder = quent_readme_example::AppModel::build("App");

    let options = CxxOptions {
        crate_name: "quent-bridge".into(),
        instrumentation_crate: "quent_readme_example".into(),
        ..Default::default()
    };

    let files = quent_codegen::emit_cxx(&builder, &options);
    let bridge_files = quent_codegen::write_bridge_files(&files, &options);

    cxx_build::bridges(bridge_files)
        .std("c++20")
        .compile("quent_bridge");

    quent_codegen::copy_cxx_headers();
}
