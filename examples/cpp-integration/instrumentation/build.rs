// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build script: runs quent-codegen to generate CXX bridge modules, then
//! runs cxx_build to generate C++ headers from them.

use quent_codegen::CxxOptions;
use quent_cpp_example_model::{Job, Task, ThreadPool};
use quent_model::{AttributeDef, ModelComponent, ValueType};
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir: PathBuf = std::env::var("CARGO_MANIFEST_DIR").unwrap().into();
    let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    let gen_dir = out_dir.join("bridge");
    fs::create_dir_all(&gen_dir).unwrap();

    // Collect model metadata
    let mut builder = quent_model::ModelBuilder::new();
    Job::collect(&mut builder);
    ThreadPool::collect(&mut builder);
    Task::collect(&mut builder);

    // Patch metadata that the derive macros do not yet extract automatically.

    // Entity event attributes: the Entity derive macro records event type names
    // but does not introspect event struct fields.
    for entity in &mut builder.entities {
        match entity.name.as_str() {
            "job" => {
                for event in &mut entity.events {
                    if event.name == "submit" {
                        event.attributes = vec![
                            AttributeDef {
                                name: "name".to_string(),
                                value_type: ValueType::String,
                                optional: false,
                            },
                            AttributeDef {
                                name: "num_tasks".to_string(),
                                value_type: ValueType::U32,
                                optional: false,
                            },
                        ];
                    }
                    // complete is a unit event — no attributes needed
                }
            }
            "thread_pool" => {
                for event in &mut entity.events {
                    if event.name == "thread_pool_init" {
                        event.attributes = vec![AttributeDef {
                            name: "num_threads".to_string(),
                            value_type: ValueType::U32,
                            optional: false,
                        }];
                    }
                }
            }
            _ => {}
        }
    }

    // FSM state metadata: the State derive macro uses placeholder ValueTypes
    // and leaves resource_name empty. Patch both.
    for fsm in &mut builder.fsms {
        if fsm.name == "task" {
            for state in &mut fsm.states {
                match state.name.as_str() {
                    "queued" => {
                        // Fix job_id type: Uuid, not String
                        for attr in &mut state.attributes {
                            if attr.name == "job_id" {
                                attr.value_type = ValueType::Uuid;
                            }
                        }
                    }
                    "running" => {
                        for usage in &mut state.usages {
                            if usage.field_name == "thread" {
                                usage.resource_name = "processor".to_string();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Generate CXX bridge Rust source files
    let options = CxxOptions {
        namespace: "telemetry".to_string(),
        crate_name: "quent-cpp-example-instrumentation".to_string(),
        bridge_path: "src/bridge".to_string(),
        model_crate: "quent_cpp_example_model".to_string(),
        event_type: "quent_cpp_example_model::ExampleEvent".to_string(),
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
        bridge_files.push(path.display().to_string());

        // Collect module declarations for bridge_mod.rs
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
        .compile("quent_cpp_example_instrumentation");

    // Copy CXX-generated headers to a stable location for CMake.
    // cxx_build puts them in OUT_DIR/cxxbridge/include/ but Corrosion
    // doesn't propagate these paths. Copy to instrumentation/include/.
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
