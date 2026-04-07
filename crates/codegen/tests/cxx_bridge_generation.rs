// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for CXX bridge code generation.

use quent_codegen::{CxxOptions, emit_cxx};

#[test]
fn generate_query_engine_cxx_bridge() {
    let builder = quent_query_engine_model::QueryEngineModel::build();

    let options = CxxOptions {
        namespace: "quent::qe".into(),
        instrumentation_crate: "quent_query_engine_model".into(),
        model_name: "QueryEngine".into(),
        ..Default::default()
    };
    let files = emit_cxx(&builder, &options);

    // uuid + context + 6 entities + 1 FSM + lib.rs = 10 files
    assert!(
        files.len() >= 10,
        "expected at least 10 files, got {}",
        files.len()
    );

    assert!(files.iter().any(|f| f.name == "uuid.rs"));
    assert!(files.iter().any(|f| f.name == "engine.rs"));
    assert!(files.iter().any(|f| f.name == "operator.rs"));
    assert!(files.iter().any(|f| f.name == "port.rs"));
    assert!(files.iter().any(|f| f.name == "query.rs"));

    let lib = files.iter().find(|f| f.name == "lib.rs").unwrap();
    assert!(lib.content.contains("pub mod uuid;"));
    assert!(lib.content.contains("pub mod engine;"));
    assert!(lib.content.contains("pub mod query;"));

    for file in &files {
        if file.name.ends_with(".rs") {
            syn::parse_file(&file.content).unwrap_or_else(|e| panic!("{}: {}", file.name, e));
        }
    }
}

#[test]
fn generate_task_fsm_cxx_bridge() {
    let builder = quent_simulator_instrumentation::SimulatorModel::build();

    let options = CxxOptions {
        instrumentation_crate: "quent_simulator_instrumentation".into(),
        model_name: "Simulator".into(),
        ..Default::default()
    };
    let files = emit_cxx(&builder, &options);

    let task_file = files.iter().find(|f| f.name == "task.rs").unwrap();
    assert!(task_file.content.contains("TaskHandle"));
    assert!(task_file.content.contains("#[cxx::bridge"));
    assert!(task_file.content.contains("Queueing"));

    for file in &files {
        if file.name.ends_with(".rs") {
            syn::parse_file(&file.content).unwrap_or_else(|e| panic!("{}: {}", file.name, e));
        }
    }
}
