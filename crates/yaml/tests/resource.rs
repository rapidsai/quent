// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Resource tests: a `resource:` block declares capacities (or `true` for a
//! unit resource), `uses:` claims a resource, and `sets-resource-bounds: true`
//! marks an attribute carrying the generated bounds record.

use quent_schema::test_utils::{ident, path, record_type};
use quent_schema::{DataType, Schema};
use quent_yaml::parse_from_str;

const RESOURCE: &str = "quent.resource.v0.1.0";
const REF_TARGET: &str = "quent.ref-target.v0.1.0";
const FSM: &str = "quent.fsm.v0.1.0";

fn schema_of(src: &str) -> Schema {
    parse_from_str(src, None).expect("parses").schema
}

fn errors_of(src: &str) -> String {
    match parse_from_str(src, None) {
        Ok(_) => panic!("expected errors, but parsing succeeded"),
        Err(e) => e.to_string(),
    }
}

/// A memory resource used by a one-state task FSM. Memory is bounded, so it
/// carries a bounds event.
const MEMORY_AND_TASK: &str = "\
quent: alpha
model: m
entities:
  Memory:
    resource:
      bandwidth: { kind: rate, known-bounds: true }
    events:
      resized:
        multi: true
        attributes:
          limits: { sets-resource-bounds: true }
fsms:
  Task:
    states:
      running:
        initial: true
        attributes:
          mem: { uses: Memory }
        to: [exit]
";

#[test]
fn resource_declaration_generates_records_and_carries_bounds() {
    let schema = schema_of(MEMORY_AND_TASK);

    // The definition rides on the resource entity.
    let memory = schema.entity(&path("Memory")).unwrap();
    assert!(memory.annotations().has_constraint(RESOURCE));
    let definition = memory
        .annotations()
        .constraint(RESOURCE)
        .unwrap()
        .data()
        .unwrap();
    assert!(definition.contains(r#""kind":"rate""#), "{definition}");

    // The builder generates the usage and bounds records.
    let usage = schema.record(&path("MemoryUsage")).unwrap();
    assert!(usage.field(&ident("bandwidth")).is_some());
    let bounds = schema.record(&path("MemoryBounds")).unwrap();
    assert!(bounds.field(&ident("bandwidth")).is_some());

    let field = memory
        .event(&ident("resized"))
        .unwrap()
        .field(&ident("limits"))
        .unwrap();
    assert_eq!(field.ty(), &record_type("MemoryBounds"));
}

#[test]
fn unit_resource_generates_an_empty_usage_record() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  Thread:
    resource: true
    events:
      registered: {}
",
    );
    assert!(
        schema
            .entity(&path("Thread"))
            .unwrap()
            .annotations()
            .has_constraint(RESOURCE)
    );
    let usage = schema.record(&path("ThreadUsage")).unwrap();
    assert_eq!(usage.fields().count(), 0);
    assert!(schema.record(&path("ThreadBounds")).is_none());
}

#[test]
fn an_fsm_can_also_be_a_unit_resource() {
    let schema = schema_of(
        "\
quent: alpha
model: m
fsms:
  Worker:
    resource: true
    states:
      running:
        initial: true
        to: [exit]
",
    );
    let worker = schema.entity(&path("Worker")).unwrap();
    assert!(worker.annotations().has_constraint(FSM));
    assert!(worker.annotations().has_constraint(RESOURCE));
}

#[test]
fn an_fsm_resource_carries_its_bounds_on_a_transition() {
    let schema = schema_of(
        "\
quent: alpha
model: m
fsms:
  Pool:
    resource:
      slots: { kind: occupancy, known-bounds: true }
    states:
      resizing:
        initial: true
        attributes:
          limits: { sets-resource-bounds: true }
        to: [resizing, exit]
",
    );
    let pool = schema.entity(&path("Pool")).unwrap();
    assert!(pool.annotations().has_constraint(FSM));
    assert!(pool.annotations().has_constraint(RESOURCE));
    let field = pool
        .event(&ident("resizing"))
        .unwrap()
        .field(&ident("limits"))
        .unwrap();
    assert_eq!(field.ty(), &record_type("PoolBounds"));
}

#[test]
fn uses_carries_the_usage_record_on_a_targeted_reference() {
    let schema = schema_of(MEMORY_AND_TASK);
    let field = schema
        .entity(&path("Task"))
        .unwrap()
        .event(&ident("running"))
        .unwrap()
        .field(&ident("mem"))
        .unwrap();
    let DataType::EntityRef { data, annotations } = field.ty() else {
        panic!("expected an entity ref, got {:?}", field.ty());
    };
    assert_eq!(data.as_deref(), Some(&record_type("MemoryUsage")));
    assert_eq!(
        annotations.constraint(REF_TARGET).unwrap().data(),
        Some("Memory")
    );
}

#[test]
fn uses_target_must_declare_a_resource() {
    let errors = errors_of(
        "\
quent: alpha
model: m
records:
  WorkerUsage:
    fields: {}
entities:
  Worker:
    events:
      registered: {}
fsms:
  Task:
    states:
      running:
        initial: true
        attributes:
          worker: { uses: Worker }
        to: [exit]
",
    );

    assert!(
        errors.contains("`Worker` does not declare a resource"),
        "{errors}"
    );
}

#[test]
fn generated_record_names_can_be_overridden() {
    let schema = schema_of(
        "\
quent: alpha
model: m
records:
  MemoryUsage:
    fields: {}
entities:
  Memory:
    resource:
      capacities:
        bytes: { kind: occupancy, known-bounds: true }
      usage-record: MemoryClaim
      bounds-record: MemoryLimits
    events:
      resized:
        multi: true
        attributes:
          limits: { sets-resource-bounds: true }
fsms:
  Task:
    states:
      running:
        initial: true
        attributes:
          memory: { uses: Memory }
        to: [exit]
",
    );

    assert!(schema.record(&path("MemoryUsage")).is_some());
    assert!(
        schema
            .record(&path("MemoryClaim"))
            .unwrap()
            .field(&ident("bytes"))
            .is_some()
    );
    assert!(
        schema
            .record(&path("MemoryLimits"))
            .unwrap()
            .field(&ident("bytes"))
            .is_some()
    );

    let usage = schema
        .entity(&path("Task"))
        .unwrap()
        .event(&ident("running"))
        .unwrap()
        .field(&ident("memory"))
        .unwrap();
    let DataType::EntityRef { data, .. } = usage.ty() else {
        panic!("expected an entity ref");
    };
    assert_eq!(data.as_deref(), Some(&record_type("MemoryClaim")));

    let bounds = schema
        .entity(&path("Memory"))
        .unwrap()
        .event(&ident("resized"))
        .unwrap()
        .field(&ident("limits"))
        .unwrap();
    assert_eq!(bounds.ty(), &record_type("MemoryLimits"));
}

#[test]
fn resource_bounds_without_known_bounds_are_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Threads:
    resource:
      slots: { kind: occupancy }
    events:
      resized:
        multi: true
        attributes:
          limits: { sets-resource-bounds: true }
",
    );
    assert!(errors.contains("bounds are known"), "{errors}");
}

#[test]
fn generated_record_name_collision_is_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
records:
  MemoryUsage:
    fields: {}
entities:
  Memory:
    resource: true
    events:
      registered: {}
",
    );
    assert!(
        errors.contains("duplicate type path `MemoryUsage`"),
        "{errors}"
    );
}

#[test]
fn resource_bounds_marker_must_be_true() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Memory:
    resource:
      bytes: { kind: occupancy, known-bounds: true }
    events:
      resized:
        multi: true
        attributes:
          limits: { sets-resource-bounds: false }
",
    );
    assert!(errors.contains("must be `true`"), "{errors}");
}

#[test]
fn a_hand_written_resource_constraint_is_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Memory:
    constraints:
      quent.resource.v0.1.0: '{\"definition\":{}}'
",
    );
    assert!(errors.contains("resource:` block"), "{errors}");
}

#[test]
fn a_non_fsm_entity_using_a_resource_is_rejected() {
    // Requirement 3: only an FSM entity may use a resource.
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Thread:
    resource: true
    events:
      registered: {}
  Worker:
    events:
      run:
        attributes:
          thread: { uses: Thread }
",
    );
    assert!(errors.contains("not an FSM"), "{errors}");
}
