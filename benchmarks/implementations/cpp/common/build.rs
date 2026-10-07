// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() {
    let include = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("include");
    cxx_build::CFG.exported_header_dirs.push(&include);
    cxx_build::bridge("src/lib.rs")
        .include(&include)
        .file("src/empty_loop.cpp")
        .std("c++20")
        .compile("quent_bench_cpp_common");
    for path in ["src/lib.rs", "src/empty_loop.cpp", "include"] {
        println!("cargo:rerun-if-changed={path}");
    }
}
