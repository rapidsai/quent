// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build script: runs quent-codegen to generate CXX bridge modules, then
//! runs cxx_build to generate C++ headers from them.

use quent_codegen::CxxOptions;
use quent_cpp_example_instrumentation::{Job, Task, ThreadPool};
use quent_model::ModelComponent;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir: PathBuf = std::env::var("CARGO_MANIFEST_DIR").unwrap().into();
    let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();

    // Collect model metadata
    let mut builder = quent_model::ModelBuilder::new();
    Job::collect(&mut builder);
    ThreadPool::collect(&mut builder);
    Task::collect(&mut builder);

    // Generate CXX bridge Rust source files into gen/
    // C++ include paths will be: quent-bridge/gen/{file}.rs.h
    let gen_dir = manifest_dir.join("gen");
    fs::create_dir_all(&gen_dir).unwrap();

    let options = CxxOptions {
        crate_name: "quent-bridge".into(),
        bridge_path: "gen".into(),
        model_crate: "quent_cpp_example_instrumentation".into(),
        event_type: "quent_cpp_example_instrumentation::ExampleEvent".into(),
        ..Default::default()
    };
    let files = quent_codegen::emit_cxx(&builder, &options);

    let mut bridge_files = Vec::new();
    let mut mod_lines = Vec::new();
    for file in &files {
        if file.name == "lib.rs" {
            continue; // We provide our own lib.rs
        }
        let path = gen_dir.join(&file.name);
        fs::write(&path, &file.content).unwrap();
        bridge_files.push(format!("gen/{}", file.name));

        let mod_name = file.name.trim_end_matches(".rs");
        mod_lines.push(format!(
            "#[path = \"{}/{}\"]\npub mod {};",
            gen_dir.display(),
            file.name,
            mod_name
        ));
    }

    // Write the bridge_mod.rs that lib.rs includes
    fs::write(out_dir.join("bridge_mod.rs"), mod_lines.join("\n")).unwrap();

    // Run cxx_build on the generated bridge files to produce C++ headers
    cxx_build::bridges(bridge_files)
        .std("c++20")
        .compile("quent_bridge");

    // Copy CXX-generated headers to a stable location for CMake.
    let cxx_include = out_dir.join("cxxbridge").join("include");
    let header_dir = manifest_dir.join("include");
    if cxx_include.exists() {
        copy_dir_recursive(&cxx_include, &header_dir);
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) {
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
