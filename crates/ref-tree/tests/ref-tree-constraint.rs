// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::{Constraint, validate as run_constraints};
use quent_ref_target::RefTargetConstraint;
use quent_ref_tree::{RefTreeConstraint, RefTreeError};
use quent_schema::{
    Annotations, DataType, Entity, Record, Schema,
    builder::AnnotationsBuilder,
    test_utils::{entity, event, field, ident, record, schema},
};

/// A type-erased tree-forming reference (no target).
fn tree_ref() -> DataType {
    DataType::EntityRef {
        data: None,
        annotations: AnnotationsBuilder::new()
            .constraint(RefTreeConstraint::NAME, None)
            .unwrap()
            .build(),
    }
}

/// A tree-forming reference restricted to a specific parent entity type.
fn tree_ref_to(target: &str) -> DataType {
    let data = serde_json::to_string(&ident(target)).unwrap();
    DataType::EntityRef {
        data: None,
        annotations: AnnotationsBuilder::new()
            .constraint(RefTreeConstraint::NAME, None)
            .unwrap()
            .constraint(RefTargetConstraint::NAME, Some(data))
            .unwrap()
            .build(),
    }
}

/// A plain (non-tree) entity reference carrying `data` as its payload.
fn ref_carrying(data: DataType) -> DataType {
    DataType::EntityRef {
        data: Some(Box::new(data)),
        annotations: Annotations::default(),
    }
}

/// An entity whose event carries no tree-forming reference, so it is a root.
fn root(name: &str) -> Entity {
    entity(
        name,
        vec![event("created", vec![field("x", DataType::U64)])],
    )
}

/// An entity whose single event carries one tree-forming reference `ty`.
fn child(name: &str, ty: DataType) -> Entity {
    entity(name, vec![event("created", vec![field("parent", ty)])])
}

fn schema_with(entities: Vec<Entity>) -> Schema {
    schema("S", entities, vec![])
}

fn schema_with_records(entities: Vec<Entity>, records: Vec<Record>) -> Schema {
    schema("S", entities, records)
}

fn validate(schema: &Schema) -> Vec<RefTreeError> {
    let report = run_constraints::<(RefTreeConstraint, RefTargetConstraint)>(schema);
    match report.results.0 {
        Ok(()) => Vec::new(),
        Err(RefTreeError::Multiple(errors)) => errors,
        Err(single) => vec![single],
    }
}

/// Assert the schema satisfies the constraint.
fn assert_valid(schema: &Schema) {
    assert!(validate(schema).is_empty());
}

/// Assert the schema produces exactly one violation, and return it.
fn single_error(schema: &Schema) -> RefTreeError {
    let mut errors = validate(schema);
    assert_eq!(errors.len(), 1);
    errors.pop().unwrap()
}

#[test]
fn target_chain_to_root_passes() {
    let schema = schema_with(vec![
        root("Cluster"),
        child("Worker", tree_ref_to("Cluster")),
        child("Task", tree_ref_to("Worker")),
    ]);
    assert_valid(&schema);
}

#[test]
fn single_child_under_root_passes() {
    let schema = schema_with(vec![
        root("Cluster"),
        child("Worker", tree_ref_to("Cluster")),
    ]);
    assert_valid(&schema);
}

#[test]
fn no_tree_ref_anywhere_passes() {
    // The constraint only forms a tree when at least one reference uses it.
    let schema = schema_with(vec![child("Solo", DataType::U64)]);
    assert_valid(&schema);
}

#[test]
fn option_nested_tree_ref_is_found() {
    let nested = DataType::Option(Box::new(tree_ref_to("Cluster")));
    let schema = schema_with(vec![root("Cluster"), child("Worker", nested)]);
    assert_valid(&schema);
}

#[test]
fn list_nested_tree_ref_is_found() {
    let nested = DataType::List(Box::new(tree_ref_to("Cluster")));
    let schema = schema_with(vec![root("Cluster"), child("Worker", nested)]);
    assert_valid(&schema);
}

#[test]
fn tree_ref_in_reference_payload_is_found() {
    let nested = ref_carrying(tree_ref_to("Cluster"));
    let schema = schema_with(vec![root("Cluster"), child("Worker", nested)]);
    assert_valid(&schema);
}

#[test]
fn tree_ref_via_record_field_resolves_parent() {
    // A parent reference reached through a record-typed event field counts.
    let meta = record("Meta", vec![field("owner", tree_ref_to("Cluster"))]);
    let worker = child("Worker", DataType::Record(ident("Meta")));
    let schema = schema_with_records(vec![root("Cluster"), worker], vec![meta]);
    assert_valid(&schema);
}

#[test]
fn multiple_refs_in_record_is_rejected() {
    // Req. 2: a record declares the parent at most once across its fields.
    let meta = record(
        "Meta",
        vec![
            field("a", tree_ref_to("Cluster")),
            field("b", tree_ref_to("Cluster")),
        ],
    );
    let worker = child("Worker", DataType::Record(ident("Meta")));
    let schema = schema_with_records(vec![root("Cluster"), worker], vec![meta]);
    assert!(matches!(
        single_error(&schema),
        RefTreeError::MultipleRefsInRecord { .. }
    ));
}

#[test]
fn recursive_record_does_not_loop() {
    // A record that nests itself (via Option) must not send the walker into an
    // infinite descent.
    let meta = record(
        "Meta",
        vec![
            field("owner", tree_ref_to("Cluster")),
            field(
                "nested",
                DataType::Option(Box::new(DataType::Record(ident("Meta")))),
            ),
        ],
    );
    let worker = child("Worker", DataType::Record(ident("Meta")));
    let schema = schema_with_records(vec![root("Cluster"), worker], vec![meta]);
    assert_valid(&schema);
}

#[test]
fn same_parent_across_events_passes() {
    // Req. 2 permits the parent reference on any number of events.
    let task = entity(
        "Task",
        vec![
            event("created", vec![field("a", tree_ref_to("Cluster"))]),
            event("moved", vec![field("b", tree_ref_to("Cluster"))]),
        ],
    );
    let schema = schema_with(vec![root("Cluster"), task]);
    assert_valid(&schema);
}

#[test]
fn type_erased_tree_ref_is_rejected() {
    // Req. 3: a tree-forming reference must be target-constrained.
    let schema = schema_with(vec![root("Cluster"), child("Worker", tree_ref())]);
    assert!(matches!(
        single_error(&schema),
        RefTreeError::NotTargetConstrained { .. }
    ));
}

#[test]
fn type_erased_tree_ref_via_record_is_rejected() {
    // Req. 3 also reaches references hidden behind a record-typed field.
    let meta = record("Meta", vec![field("owner", tree_ref())]);
    let worker = child("Worker", DataType::Record(ident("Meta")));
    let schema = schema_with_records(vec![root("Cluster"), worker], vec![meta]);
    assert!(matches!(
        single_error(&schema),
        RefTreeError::NotTargetConstrained { .. }
    ));
}

#[test]
fn conflicting_parents_is_rejected() {
    // Req. 2: a non-root declaring two distinct parent types across events.
    let task = entity(
        "Task",
        vec![
            event("created", vec![field("a", tree_ref_to("Worker"))]),
            event("moved", vec![field("b", tree_ref_to("Cluster"))]),
        ],
    );
    let schema = schema_with(vec![
        root("Cluster"),
        child("Worker", tree_ref_to("Cluster")),
        task,
    ]);
    assert!(matches!(
        single_error(&schema),
        RefTreeError::ConflictingParents { entity, .. } if entity == "Task"
    ));
}

#[test]
fn two_tree_refs_in_one_event_is_rejected() {
    let task = entity(
        "Task",
        vec![event(
            "created",
            vec![
                field("a", tree_ref_to("Cluster")),
                field("b", tree_ref_to("Cluster")),
            ],
        )],
    );
    let schema = schema_with(vec![root("Cluster"), task]);
    assert!(matches!(
        single_error(&schema),
        RefTreeError::MultiplePerEvent { .. }
    ));
}

#[test]
fn direct_and_record_refs_in_one_event_is_rejected() {
    let meta = record("Meta", vec![field("owner", tree_ref_to("Cluster"))]);
    let worker = entity(
        "Worker",
        vec![event(
            "created",
            vec![
                field("direct", tree_ref_to("Cluster")),
                field("meta", DataType::Record(ident("Meta"))),
            ],
        )],
    );
    let schema = schema_with_records(vec![root("Cluster"), worker], vec![meta]);
    assert!(matches!(
        single_error(&schema),
        RefTreeError::MultiplePerEvent { .. }
    ));
}

#[test]
fn no_root_is_rejected() {
    // Req. 1: every entity has a parent, so none is a root.
    let schema = schema_with(vec![
        child("A", tree_ref_to("B")),
        child("B", tree_ref_to("A")),
    ]);
    assert!(matches!(single_error(&schema), RefTreeError::NoRoot));
}

#[test]
fn multiple_roots_is_rejected() {
    // Req. 1: two entities carry no tree-forming reference.
    let schema = schema_with(vec![
        root("Cluster"),
        root("Other"),
        child("Worker", tree_ref_to("Cluster")),
    ]);
    assert!(matches!(
        single_error(&schema),
        RefTreeError::MultipleRoots { .. }
    ));
}

#[test]
fn unknown_target_is_rejected() {
    // Req. 5: a reference targeting a non-existent entity is rejected at the
    // reference site.
    let schema = schema_with(vec![root("Cluster"), child("A", tree_ref_to("Ghost"))]);
    assert!(matches!(
        single_error(&schema),
        RefTreeError::UnknownTarget { target, .. } if target == "Ghost"
    ));
}

#[test]
fn target_cycle_is_unreachable() {
    // Req. 4: A and B form a cycle with no path to the root.
    let schema = schema_with(vec![
        root("Cluster"),
        child("A", tree_ref_to("B")),
        child("B", tree_ref_to("A")),
    ]);
    let errors = validate(&schema);
    assert_eq!(errors.len(), 2);
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, RefTreeError::Unreachable { entity } if entity == "A")),
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, RefTreeError::Unreachable { entity } if entity == "B")),
    );
}
