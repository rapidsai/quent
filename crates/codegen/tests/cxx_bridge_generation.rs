// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for CXX bridge code generation.

use quent_codegen::{CxxOptions, emit_cxx};
use quent_model::ModelComponent;

#[test]
fn generate_query_engine_cxx_bridge() {
    let mut builder = quent_model::ModelBuilder::new();
    quent_query_engine_model::query::Query::collect(&mut builder);
    quent_query_engine_model::engine::Engine::collect(&mut builder);
    quent_query_engine_model::worker::Worker::collect(&mut builder);
    quent_query_engine_model::operator::Operator::collect(&mut builder);
    quent_query_engine_model::port::Port::collect(&mut builder);
    quent_query_engine_model::plan::Plan::collect(&mut builder);
    quent_query_engine_model::query_group::QueryGroup::collect(&mut builder);

    let options = CxxOptions {
        namespace: "quent::qe".to_string(),
        ..Default::default()
    };
    let files = emit_cxx(&builder, &options);

    // Should generate: uuid.rs + 6 entities + 1 FSM + lib.rs = 9 files
    assert!(files.len() >= 9, "expected at least 9 files, got {}", files.len());

    // Check uuid bridge exists
    assert!(files.iter().any(|f| f.name == "uuid.rs"));

    // Check entity bridges exist
    assert!(files.iter().any(|f| f.name == "engine.rs"));
    assert!(files.iter().any(|f| f.name == "operator.rs"));
    assert!(files.iter().any(|f| f.name == "port.rs"));

    // Check FSM bridge exists
    assert!(files.iter().any(|f| f.name == "query.rs"));

    // Check lib.rs
    let lib = files.iter().find(|f| f.name == "lib.rs").unwrap();
    assert!(lib.content.contains("pub mod uuid;"));
    assert!(lib.content.contains("pub mod engine;"));
    assert!(lib.content.contains("pub mod query;"));

    // Print for inspection
    for file in &files {
        println!("=== {} ===\n{}\n", file.name, file.content);
    }
}

#[test]
fn generate_task_fsm_cxx_bridge() {
    let mut builder = quent_model::ModelBuilder::new();
    quent_simulator_model::task::Task::collect(&mut builder);

    let options = CxxOptions::default();
    let files = emit_cxx(&builder, &options);

    let task_file = files.iter().find(|f| f.name == "task.rs").unwrap();

    // Should have the handle type
    assert!(task_file.content.contains("TaskHandle"));

    // Should have bridge module
    assert!(task_file.content.contains("#[cxx::bridge"));

    // Should have state structs for states with attributes
    assert!(task_file.content.contains("Queueing"));
    assert!(task_file.content.contains("Computing"));

    println!("=== task.rs ===\n{}", task_file.content);
}
