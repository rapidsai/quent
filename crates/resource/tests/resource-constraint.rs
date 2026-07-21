// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_fsm::FsmConstraint;
use quent_resource::{ResourceConstraint, ResourceError};
use quent_schema::{
    Annotations, DataType, Entity, Record, Schema,
    builder::{AnnotationsBuilder, EntityBuilder, RecordBuilder, SchemaBuilder},
    test_utils::{event, field, ident, schema},
};

/// Create resource definition data from `(name, kind, bounded)` tuples.
fn definition_data(capacities: &[(&str, &str, bool)]) -> String {
    let capacities: serde_json::Map<String, serde_json::Value> = capacities
        .iter()
        .map(|&(name, kind, bounded)| {
            (
                name.to_string(),
                serde_json::json!({ "kind": kind, "bounded": bounded }),
            )
        })
        .collect();
    serde_json::json!({ "definition": serde_json::Value::Object(capacities) }).to_string()
}

fn usage_data(resource: &str) -> String {
    serde_json::json!({ "usage": { "resource": resource } }).to_string()
}

fn bounds_data(resource: &str) -> String {
    serde_json::json!({ "bounds": { "resource": resource } }).to_string()
}

/// Create resource constraint annotations carrying `data`.
fn resource_annotations(data: String) -> Annotations {
    AnnotationsBuilder::new()
        .try_with_constraint(ResourceConstraint::NAME, Some(data))
        .unwrap()
        .build()
}

fn fsm_annotations() -> Annotations {
    AnnotationsBuilder::new()
        .try_with_constraint(FsmConstraint::NAME, None)
        .unwrap()
        .build()
}

/// Create a record carrying resource `data` with a `U64` field for each name.
fn resource_record(name: &str, data: String, fields: &[&str]) -> Record {
    let mut builder = RecordBuilder::new(ident(name)).with_annotations(resource_annotations(data));
    for &f in fields {
        builder = builder.try_with_field(field(f, DataType::U64)).unwrap();
    }
    builder.build()
}

fn usage_record(name: &str, resource: &str, claims: &[&str]) -> Record {
    resource_record(name, usage_data(resource), claims)
}

fn bounds_record(name: &str, resource: &str, fields: &[&str]) -> Record {
    resource_record(name, bounds_data(resource), fields)
}

/// Create a resource entity with a bounds event referencing `bounds` when given.
fn resource_entity(name: &str, capacities: &[(&str, &str, bool)], bounds: Option<&str>) -> Entity {
    let mut builder = EntityBuilder::new(ident(name))
        .with_annotations(resource_annotations(definition_data(capacities)));
    if let Some(bounds) = bounds {
        builder = builder
            .try_with_event(event(
                "operating",
                [field("bounds", DataType::Record(ident(bounds)))],
            ))
            .unwrap();
    }
    builder.build()
}

/// Create an entity with one event referencing `record`.
///
/// It is an FSM iff `fsm`, and the record rides on an entity reference iff
/// `on_ref`.
fn user_entity(name: &str, fsm: bool, record: &str, on_ref: bool) -> Entity {
    let record = DataType::Record(ident(record));
    let ty = if on_ref {
        DataType::EntityRef {
            data: Some(Box::new(record)),
            annotations: Annotations::default(),
        }
    } else {
        record
    };
    let mut builder = EntityBuilder::new(ident(name))
        .try_with_event(event("using", [field("claim", ty)]))
        .unwrap();
    if fsm {
        builder = builder.with_annotations(fsm_annotations());
    }
    builder.build()
}

fn resource_errors(schema: &Schema) -> Vec<ResourceError> {
    match quent_constraints::validate::<(ResourceConstraint,)>(schema)
        .results
        .0
    {
        Ok(()) => Vec::new(),
        Err(ResourceError::Multiple(errors)) => errors,
        Err(single) => vec![single],
    }
}

/// A resource, its bounds and usage records, and an FSM user of it.
#[test]
fn valid_resource_passes() {
    let memory = resource_entity(
        "Memory",
        &[("bytes", "occupancy", true)],
        Some("MemoryBounds"),
    );
    let worker = user_entity("Worker", true, "MemoryUsage", true);
    let bounds = bounds_record("MemoryBounds", "Memory", &["bytes"]);
    let usage = usage_record("MemoryUsage", "Memory", &["bytes"]);
    assert!(resource_errors(&schema("App", vec![memory, worker], vec![bounds, usage])).is_empty());
}

/// Requirement 1: a resource has at least one capacity.
#[test]
fn resource_without_capacity_is_rejected() {
    let memory = resource_entity("Memory", &[], None);
    let errors = resource_errors(&schema("App", vec![memory], vec![]));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ResourceError::NoCapacities { .. }))
    );
}

// Requirement 2 is enforced by definition map keys. Repeated builder inputs
// are covered by `builder::tests::rejects_duplicate_capacities`.

/// Requirement 3: bounds exist exactly for bounded resources and cover all
/// bounded capacities.
#[test]
fn bounds_match_resource_boundedness() {
    let disk = resource_entity(
        "Disk",
        &[
            ("blocks", "occupancy", true),
            ("sectors", "occupancy", true),
            ("watts", "rate", false),
        ],
        Some("DiskBounds"),
    );
    let disk_bounds = bounds_record("DiskBounds", "Disk", &["sectors", "watts"]);
    let memory = resource_entity("Memory", &[("bytes", "occupancy", true)], None);
    let memory_bounds = bounds_record("MemoryBounds", "Memory", &["bytes"]);
    let network = resource_entity(
        "Network",
        &[("bytes", "rate", false)],
        Some("NetworkBounds"),
    );
    let network_bounds = bounds_record("NetworkBounds", "Network", &[]);
    let errors = resource_errors(&schema(
        "App",
        [disk, memory, network],
        [disk_bounds, memory_bounds, network_bounds],
    ));

    assert!(errors.iter().any(
        |e| matches!(e, ResourceError::UncoveredCapacity { capacity, .. } if capacity == "blocks")
    ));
    assert!(errors.iter().any(
        |e| matches!(e, ResourceError::UnboundedCapacity { capacity, .. } if capacity == "watts")
    ));
    assert!(
        errors.iter().any(
            |e| matches!(e, ResourceError::MissingBounds { resource } if resource == "Memory")
        )
    );
    assert!(errors.iter().any(
        |e| matches!(e, ResourceError::UnexpectedBounds { resource, .. } if resource == "Network")
    ));
}

/// Requirements 4–7 govern resource usage.
///
/// Resource users must be FSM entities. Usage and bounds records must name
/// declared resources, claims must name declared capacities, and usage records
/// must be carried by entity references.
#[test]
fn invalid_usages_are_rejected() {
    let memory = resource_entity("Memory", &[("bytes", "occupancy", false)], None);
    let worker = user_entity("Worker", false, "MemoryUsage", true);
    let machine = user_entity("Machine", true, "OffRefUsage", false);
    let usage = usage_record("MemoryUsage", "Memory", &["bytes"]);
    let bad_claim = usage_record("BadClaim", "Memory", &["watts"]);
    let off_ref = usage_record("OffRefUsage", "Memory", &["bytes"]);
    let unknown_usage = usage_record("GhostUsage", "Ghost", &[]);
    let unknown_bounds = bounds_record("PhantomBounds", "Phantom", &[]);
    let errors = resource_errors(&schema(
        "App",
        [memory, worker, machine],
        [usage, bad_claim, off_ref, unknown_usage, unknown_bounds],
    ));

    // Requirement 4: only an FSM entity may use a resource.
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ResourceError::NonFsmUser { entity, .. } if entity == "Worker"))
    );

    // Requirement 5: usage and bounds records name declared resources.
    assert!(errors.iter().any(
        |e| matches!(e, ResourceError::UnknownResource { resource, .. } if resource == "Ghost")
    ));
    assert!(errors.iter().any(
        |e| matches!(e, ResourceError::UnknownResource { resource, .. } if resource == "Phantom")
    ));

    // Requirement 6: a usage claims only its resource's capacities.
    assert!(errors.iter().any(
        |e| matches!(e, ResourceError::UndeclaredCapacity { capacity, .. } if capacity == "watts")
    ));

    // Requirement 7: a usage record rides on an entity reference.
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ResourceError::UsageNotOnReference { .. }))
    );
}

/// Requirement 8: a bounds record appears only on its own resource's events.
#[test]
fn bounds_record_used_by_a_foreign_entity_is_rejected() {
    let memory = resource_entity(
        "Memory",
        &[("bytes", "occupancy", true)],
        Some("MemoryBounds"),
    );
    let bounds = bounds_record("MemoryBounds", "Memory", &["bytes"]);
    let intruder = user_entity("Intruder", false, "MemoryBounds", false);
    let errors = resource_errors(&schema("App", vec![memory, intruder], vec![bounds]));
    assert!(errors.iter().any(
        |e| matches!(e, ResourceError::ForeignBounds { resource, .. } if resource == "Memory")
    ));
}

fn assert_misplaced_resource(case: &str, schema: &Schema) {
    assert!(
        resource_errors(schema)
            .iter()
            .any(|e| matches!(e, ResourceError::MisplacedRole { .. })),
        "{case}"
    );
}

/// Resource annotations and usage references are rejected in unsupported locations.
#[test]
fn misplaced_resources_are_rejected() {
    let bad_record = resource_record(
        "Memory",
        definition_data(&[("bytes", "occupancy", false)]),
        &[],
    );
    let definition_on_record = schema("App", [], [bad_record]);

    let bad_entity = EntityBuilder::new(ident("Worker"))
        .with_annotations(resource_annotations(usage_data("Memory")))
        .build();
    let usage_on_entity = schema("App", [bad_entity], []);

    let usage_on_schema = SchemaBuilder::new(ident("App"))
        .with_annotations(resource_annotations(usage_data("Memory")))
        .build();

    let memory = resource_entity("Memory", &[("bytes", "occupancy", false)], None);
    let usage = usage_record("MemoryUsage", "Memory", &["bytes"]);
    let carrier = DataType::EntityRef {
        data: Some(Box::new(DataType::Record(ident("MemoryUsage")))),
        annotations: Annotations::default(),
    };
    let wrapper = RecordBuilder::new(ident("Wrapper"))
        .try_with_field(field("carried", carrier))
        .unwrap()
        .build();
    let usage_without_entity = schema("App", [memory], [usage, wrapper]);

    for (case, schema) in [
        ("definition on record", definition_on_record),
        ("usage on entity", usage_on_entity),
        ("usage on schema", usage_on_schema),
        ("usage without an enclosing entity", usage_without_entity),
    ] {
        assert_misplaced_resource(case, &schema);
    }
}
