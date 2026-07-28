// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_fsm::FsmConstraint;
use quent_resource::{Resource, ResourceConstraint, ResourceError};
use quent_schema::{
    Annotations, DataType, Entity, Record, Schema,
    builder::{AnnotationsBuilder, EntityBuilder, RecordBuilder, SchemaBuilder},
    test_utils::{event, field, ident, path, record_type, schema},
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
    Resource::Usage {
        resource: path(resource),
    }
    .constraint_data()
    .unwrap()
}

fn bounds_data(resource: &str) -> String {
    Resource::Bounds {
        resource: path(resource),
    }
    .constraint_data()
    .unwrap()
}

/// Create resource constraint annotations carrying `data`.
fn resource_annotations(data: String) -> Annotations {
    AnnotationsBuilder::new()
        .with_constraint(ResourceConstraint::NAME, Some(data))
        .build()
        .unwrap()
}

fn fsm_annotations() -> Annotations {
    AnnotationsBuilder::new()
        .with_constraint(FsmConstraint::NAME, None)
        .build()
        .unwrap()
}

/// Create a record carrying resource `data` with a `U64` field for each name.
fn resource_record(name: &str, data: String, fields: &[&str]) -> Record {
    let mut builder = RecordBuilder::new(path(name)).with_annotations(resource_annotations(data));
    for &f in fields {
        builder = builder.with_field(field(f, DataType::U64));
    }
    builder.build().unwrap()
}

fn usage_record(name: &str, resource: &str, claims: &[&str]) -> Record {
    resource_record(name, usage_data(resource), claims)
}

fn bounds_record(name: &str, resource: &str, fields: &[&str]) -> Record {
    resource_record(name, bounds_data(resource), fields)
}

/// Create a resource entity with a bounds event referencing `bounds` when given.
fn resource_entity(name: &str, capacities: &[(&str, &str, bool)], bounds: Option<&str>) -> Entity {
    let bounds = bounds.map(record_type);
    resource_entity_with_bounds_type(name, capacities, bounds)
}

fn resource_entity_with_bounds_type(
    name: &str,
    capacities: &[(&str, &str, bool)],
    bounds: Option<DataType>,
) -> Entity {
    let mut builder = EntityBuilder::new(path(name))
        .with_annotations(resource_annotations(definition_data(capacities)));
    let fields = bounds.map(|bounds| field("bounds", bounds)).into_iter();
    builder = builder.with_event(event("operating", fields));
    builder.build().unwrap()
}

/// Create an entity with one event referencing `record`.
///
/// It is an FSM iff `fsm`, and the record rides on an entity reference iff
/// `on_ref`.
fn user_entity(name: &str, fsm: bool, record: &str, on_ref: bool) -> Entity {
    let record = record_type(record);
    user_entity_with_data_type(name, fsm, record, on_ref)
}

fn user_entity_with_data_type(name: &str, fsm: bool, data: DataType, on_ref: bool) -> Entity {
    let ty = if on_ref {
        DataType::EntityRef {
            data: Some(Box::new(data)),
            annotations: Annotations::default(),
        }
    } else {
        data
    };
    let mut builder =
        EntityBuilder::new(path(name)).with_event(event("using", [field("claim", ty)]));
    if fsm {
        builder = builder.with_annotations(fsm_annotations());
    }
    builder.build().unwrap()
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

#[test]
fn qualified_resource_paths_pass() {
    let memory = resource_entity(
        "Foo::Memory",
        &[("bytes", "occupancy", true)],
        Some("Foo::MemoryBounds"),
    );
    let bounds = bounds_record("Foo::MemoryBounds", "Foo::Memory", &["bytes"]);
    let usage = usage_record("Foo::MemoryUsage", "Foo::Memory", &["bytes"]);
    let worker = user_entity("Bar::Worker", true, "Foo::MemoryUsage", true);

    assert!(resource_errors(&schema("App", [memory, worker], [bounds, usage])).is_empty());
}

/// A resource with no capacities is accepted as a unit resource.
#[test]
fn unit_resource_is_accepted() {
    let thread = resource_entity("Thread", &[], None);
    assert!(resource_errors(&schema("App", vec![thread], vec![])).is_empty());
}

// Requirement 1 is enforced by definition map keys. Repeated builder inputs
// are covered by `builder::tests::rejects_duplicate_capacities`.

/// Requirement 2: bounds exist exactly for bounded resources and cover all
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

/// Requirements 3–6 govern resource usage.
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

    // Requirement 3: only an FSM entity may use a resource.
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ResourceError::NonFsmUser { entity, .. } if entity == "Worker"))
    );

    // Requirement 4: usage and bounds records name declared resources.
    assert!(errors.iter().any(
        |e| matches!(e, ResourceError::UnknownResource { resource, .. } if resource == "Ghost")
    ));
    assert!(errors.iter().any(
        |e| matches!(e, ResourceError::UnknownResource { resource, .. } if resource == "Phantom")
    ));

    // Requirement 5: a usage claims only its resource's capacities.
    assert!(errors.iter().any(
        |e| matches!(e, ResourceError::UndeclaredCapacity { capacity, .. } if capacity == "watts")
    ));

    // Requirement 6: a usage record rides on an entity reference.
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ResourceError::UsageNotOnReference { .. }))
    );
}

/// Requirement 7: a bounds record appears only on its own resource's events.
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

#[test]
fn optional_resource_records_are_accepted() {
    let memory = resource_entity_with_bounds_type(
        "Memory",
        &[("bytes", "occupancy", true)],
        Some(DataType::Option(Box::new(record_type("MemoryBounds")))),
    );
    let worker = user_entity_with_data_type(
        "Worker",
        true,
        DataType::Option(Box::new(record_type("MemoryUsage"))),
        true,
    );
    let bounds = bounds_record("MemoryBounds", "Memory", &["bytes"]);
    let usage = usage_record("MemoryUsage", "Memory", &["bytes"]);

    assert!(resource_errors(&schema("App", [memory, worker], [bounds, usage])).is_empty());
}

#[test]
fn listed_resource_references_with_usage_are_accepted() {
    let memory = resource_entity("Memory", &[("bytes", "occupancy", false)], None);
    let reference = DataType::EntityRef {
        data: Some(Box::new(record_type("MemoryUsage"))),
        annotations: Annotations::default(),
    };
    let worker =
        user_entity_with_data_type("Worker", true, DataType::List(Box::new(reference)), false);
    let usage = usage_record("MemoryUsage", "Memory", &["bytes"]);

    assert!(resource_errors(&schema("App", [memory, worker], [usage])).is_empty());
}

#[test]
fn usage_records_in_list_valued_reference_data_are_rejected() {
    let memory = resource_entity("Memory", &[("bytes", "occupancy", false)], None);
    let worker = user_entity_with_data_type(
        "Worker",
        true,
        DataType::List(Box::new(record_type("MemoryUsage"))),
        true,
    );
    let usage = usage_record("MemoryUsage", "Memory", &["bytes"]);
    let errors = resource_errors(&schema("App", [memory, worker], [usage]));

    assert!(
        errors
            .iter()
            .any(|error| matches!(error, ResourceError::UsageInList { .. }))
    );
}

#[test]
fn listed_bounds_records_are_rejected() {
    let memory = resource_entity_with_bounds_type(
        "Memory",
        &[("bytes", "occupancy", true)],
        Some(DataType::List(Box::new(record_type("MemoryBounds")))),
    );
    let bounds = bounds_record("MemoryBounds", "Memory", &["bytes"]);
    let errors = resource_errors(&schema("App", [memory], [bounds]));

    assert!(
        errors
            .iter()
            .any(|error| matches!(error, ResourceError::BoundsInList { .. }))
    );
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
        .with_event(event("using", []))
        .with_annotations(resource_annotations(usage_data("Memory")))
        .build()
        .unwrap();
    let usage_on_entity = schema("App", [bad_entity], []);

    let usage_on_schema = SchemaBuilder::new(ident("App"))
        .with_annotations(resource_annotations(usage_data("Memory")))
        .build()
        .unwrap();

    let memory = resource_entity("Memory", &[("bytes", "occupancy", false)], None);
    let usage = usage_record("MemoryUsage", "Memory", &["bytes"]);
    let carrier = DataType::EntityRef {
        data: Some(Box::new(record_type("MemoryUsage"))),
        annotations: Annotations::default(),
    };
    let wrapper = RecordBuilder::new(ident("Wrapper"))
        .with_field(field("carried", carrier))
        .build()
        .unwrap();
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
