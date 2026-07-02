// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_ref_target::{RefTargetConstraint, RefTargetError};
use quent_schema::{
    DataType, Entity, Schema,
    builder::AnnotationsBuilder,
    test_utils::{entity, event, field, ident, schema},
};

// RefTarget is a transparent newtype over Identifier, so the wire format is just
// the bare entity name.
fn target_data(target: &str) -> String {
    serde_json::to_string(&ident(target)).unwrap()
}

fn ref_with(data: Option<String>) -> DataType {
    DataType::EntityRef {
        data: None,
        annotations: AnnotationsBuilder::new()
            .constraint(RefTargetConstraint::NAME, data)
            .unwrap()
            .build(),
    }
}

fn schema_with(entities: Vec<Entity>) -> Schema {
    schema("S", entities, vec![])
}

fn validate(schema: &Schema) -> Vec<RefTargetError> {
    let report = quent_constraints::validate::<(RefTargetConstraint,)>(schema);
    match report.results.0 {
        Ok(()) => Vec::new(),
        Err(RefTargetError::Multiple(errors)) => errors,
        Err(single) => vec![single],
    }
}

#[test]
fn ref_to_existing_entity_passes() {
    let worker = entity("Worker", vec![]);
    let task = entity(
        "Task",
        vec![event(
            "created",
            vec![field("on", ref_with(Some(target_data("Worker"))))],
        )],
    );
    assert!(validate(&schema_with(vec![worker, task])).is_empty());
}

#[test]
fn ref_to_unknown_entity_is_rejected() {
    let task = entity(
        "Task",
        vec![event(
            "created",
            vec![field("on", ref_with(Some(target_data("ghost"))))],
        )],
    );
    let errors = validate(&schema_with(vec![task]));
    assert!(
        errors.iter().any(
            |e| matches!(e, RefTargetError::UnknownTarget { target, .. } if target == "ghost")
        ),
    );
}

#[test]
fn missing_data_is_rejected() {
    let bad = ref_with(None);
    let task = entity("Task", vec![event("created", vec![field("on", bad)])]);
    let errors = validate(&schema_with(vec![task]));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, RefTargetError::InvalidData { .. })),
    );
}

#[test]
fn invalid_json_is_rejected() {
    let bad = ref_with(Some("{ trash".to_string()));
    let task = entity("Task", vec![event("created", vec![field("on", bad)])]);
    let errors = validate(&schema_with(vec![task]));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, RefTargetError::InvalidData { .. })),
    );
}
