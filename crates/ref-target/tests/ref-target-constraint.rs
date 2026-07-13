// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_ref_target::{RefTargetConstraint, RefTargetError};
use quent_schema::{
    DataType, Entity, Schema,
    builder::AnnotationsBuilder,
    test_utils::{entity, event, field, schema},
};

fn entity_ref(target: Option<&str>) -> DataType {
    DataType::EntityRef {
        data: None,
        annotations: AnnotationsBuilder::new()
            .try_with_constraint(RefTargetConstraint::NAME, target.map(ToString::to_string))
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
            vec![field("on", entity_ref(Some("Worker")))],
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
            vec![field("on", entity_ref(Some("ghost")))],
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
fn missing_target_is_rejected() {
    let bad = entity_ref(None);
    let task = entity("Task", vec![event("created", vec![field("on", bad)])]);
    let errors = validate(&schema_with(vec![task]));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, RefTargetError::InvalidData { .. })),
    );
}

#[test]
fn invalid_target_identifier_is_rejected() {
    let bad = entity_ref(Some("{ trash"));
    let task = entity("Task", vec![event("created", vec![field("on", bad)])]);
    let errors = validate(&schema_with(vec![task]));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, RefTargetError::InvalidData { .. })),
    );
}
