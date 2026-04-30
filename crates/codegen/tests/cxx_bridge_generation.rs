// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for CXX bridge code generation.

use quent_codegen::{CxxOptions, emit_cxx};

#[test]
fn generate_query_engine_cxx_bridge() {
    let builder = quent_query_engine_model::QueryEngineModel::build("QueryEngine");

    let options = CxxOptions {
        namespace: "quent::qe".into(),
        instrumentation_crate: "quent_query_engine_model".into(),
        ..Default::default()
    };
    let files = emit_cxx(&builder, &options);

    // uuid + context + custom_attributes + 6 entities + 1 FSM + lib.rs
    assert!(
        files.len() >= 10,
        "expected at least 10 files, got {}",
        files.len()
    );

    // All entity/FSM bridges must exist
    for name in [
        "uuid",
        "context",
        "engine",
        "worker",
        "query_group",
        "query",
        "plan",
        "operator",
        "port",
    ] {
        assert!(
            files.iter().any(|f| f.name == format!("{name}.rs")),
            "missing bridge file: {name}.rs"
        );
    }

    // Verify all generated Rust files are valid syntax
    for file in &files {
        if file.name.ends_with(".rs") {
            syn::parse_file(&file.content).unwrap_or_else(|e| panic!("{}: {}", file.name, e));
        }
    }

    // Verify nested structs are generated for complex types
    let plan_file = files.iter().find(|f| f.name == "plan.rs").unwrap();
    assert!(
        plan_file.content.contains("pub struct Parent"),
        "plan.rs should contain Parent shared struct (from PlanParent)"
    );
    assert!(
        plan_file.content.contains("pub struct Edges"),
        "plan.rs should contain Edges shared struct (from Vec<Edge>)"
    );

    let engine_file = files.iter().find(|f| f.name == "engine.rs").unwrap();
    assert!(
        engine_file.content.contains("pub struct Implementation"),
        "engine.rs should contain Implementation shared struct"
    );

    // Verify Option<Ref<T>> becomes UUID (nil = None)
    assert!(
        plan_file.content.contains("worker_id"),
        "plan.rs should have worker_id field"
    );

    // Verify Vec<Ref<T>> becomes Vec<UUID>
    let operator_file = files.iter().find(|f| f.name == "operator.rs").unwrap();
    assert!(
        operator_file.content.contains("parent_operator_ids"),
        "operator.rs should have parent_operator_ids"
    );

    // Verify CustomAttributes bridge is generated
    assert!(
        files.iter().any(|f| f.name == "custom_attributes.rs"),
        "custom_attributes.rs should be generated"
    );
}

#[test]
fn generate_simulator_cxx_bridge() {
    let builder = quent_simulator_instrumentation::SimulatorModel::build("Simulator");

    let options = CxxOptions {
        instrumentation_crate: "quent_simulator_instrumentation".into(),
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
