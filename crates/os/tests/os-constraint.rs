// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::{Constraint as _, utils::RecordValidationError};
use quent_os::{OsConstraint, OsError, process_path, process_record, thread_path, thread_record};
use quent_ref_target::RefTargetConstraint;
use quent_ref_tree::RefTreeConstraint;
use quent_schema::{
    Annotations, Cardinality, DataType, Entity, Field, Record, Schema,
    builder::{AnnotationsBuilder, EntityBuilder, EventBuilder, RecordBuilder, SchemaBuilder},
    test_utils::{field, ident, path},
};

fn os_record(path: quent_schema::Path, fields: impl IntoIterator<Item = Field>) -> Record {
    RecordBuilder::new(path)
        .with_fields(fields)
        .build()
        .unwrap()
}

fn event(
    name: &str,
    cardinality: Cardinality,
    fields: impl IntoIterator<Item = Field>,
) -> quent_schema::Event {
    EventBuilder::new(ident(name), cardinality)
        .with_fields(fields)
        .build()
        .unwrap()
}

fn entity(name: &str, events: impl IntoIterator<Item = quent_schema::Event>) -> Entity {
    EntityBuilder::new(path(name))
        .with_events(events)
        .build()
        .unwrap()
}

fn schema(
    entities: impl IntoIterator<Item = Entity>,
    records: impl IntoIterator<Item = Record>,
) -> Schema {
    SchemaBuilder::new(ident("App"))
        .with_entities(entities)
        .with_records(records)
        .build()
        .unwrap()
}

fn validate(schema: &Schema) -> Vec<OsError> {
    let report =
        quent_constraints::validate::<(OsConstraint, RefTargetConstraint, RefTreeConstraint)>(
            schema,
        );
    assert!(report.base_constraints.is_ok());
    assert!(report.unregistered_constraints.is_empty());
    let (os, ref_target, ref_tree) = report.results;
    assert!(ref_target.is_ok(), "{ref_target:?}");
    assert!(ref_tree.is_ok(), "{ref_tree:?}");
    match os {
        Ok(()) => Vec::new(),
        Err(OsError::Multiple(errors)) => errors,
        Err(error) => vec![error],
    }
}

fn scope_ref(target: &str) -> DataType {
    DataType::EntityRef {
        data: None,
        annotations: AnnotationsBuilder::new()
            .with_constraint(RefTargetConstraint::NAME, Some(target.to_string()))
            .with_constraint(RefTreeConstraint::NAME, None)
            .build()
            .unwrap(),
    }
}

#[test]
fn once_events_with_os_records_mark_processes_and_threads() {
    let process = process_record();
    let thread = thread_record();
    let process_entity = entity(
        "MyProcess",
        [
            event(
                "Init",
                Cardinality::Once,
                [field("process", DataType::Record(process_path()))],
            ),
            event(
                "Sample",
                Cardinality::Multi,
                [field("value", DataType::U64)],
            ),
        ],
    );
    let thread_entity = entity(
        "MyThread",
        [event(
            "Init",
            Cardinality::Once,
            [
                field("thread", DataType::Record(thread_path())),
                field("process", scope_ref("MyProcess")),
            ],
        )],
    );

    assert_eq!(process.path(), &process_path());
    assert_eq!(thread.path(), &thread_path());
    assert!(validate(&schema([process_entity, thread_entity], [process, thread])).is_empty());
}

#[test]
fn canonical_record_shapes_are_validated() {
    let invalid_process = os_record(process_path(), [field("native_id", DataType::I32)]);
    let invalid_thread = os_record(thread_path(), [field("other", DataType::U64)]);
    let errors = validate(&schema([], [invalid_process, invalid_thread]));

    assert!(errors.iter().any(|error| matches!(
        error,
        OsError::InvalidOsRecord(RecordValidationError::InvalidFieldType { .. })
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        OsError::InvalidOsRecord(RecordValidationError::MissingField { field, .. })
            if field == "native_id"
    )));
}

#[test]
fn thread_may_be_transitively_scoped_under_process() {
    let process_entity = entity(
        "MyProcess",
        [event(
            "Init",
            Cardinality::Once,
            [field("process", DataType::Record(process_path()))],
        )],
    );
    let worker = entity(
        "Worker",
        [event(
            "Init",
            Cardinality::Once,
            [field("process", scope_ref("MyProcess"))],
        )],
    );
    let thread_entity = entity(
        "MyThread",
        [event(
            "Init",
            Cardinality::Once,
            [
                field("thread", DataType::Record(thread_path())),
                field("worker", scope_ref("Worker")),
            ],
        )],
    );

    assert!(
        validate(&schema(
            [process_entity, worker, thread_entity],
            [process_record(), thread_record()],
        ))
        .is_empty()
    );
}

#[test]
fn thread_must_be_scoped_under_process() {
    let root = entity("Root", [event("Init", Cardinality::Once, [])]);
    let process_entity = entity(
        "MyProcess",
        [event(
            "Init",
            Cardinality::Once,
            [
                field("process", DataType::Record(process_path())),
                field("root", scope_ref("Root")),
            ],
        )],
    );
    let thread_entity = entity(
        "MyThread",
        [event(
            "Init",
            Cardinality::Once,
            [
                field("thread", DataType::Record(thread_path())),
                field("root", scope_ref("Root")),
            ],
        )],
    );

    assert!(
        validate(&schema(
            [root, process_entity, thread_entity],
            [process_record(), thread_record()],
        ))
        .iter()
        .any(|error| matches!(error, OsError::ThreadOutsideProcessScope { .. }))
    );
}

#[test]
fn os_record_event_must_be_once() {
    let process = process_record();
    let process_entity = entity(
        "MyProcess",
        [event(
            "Init",
            Cardinality::Multi,
            [field("process", DataType::Record(process_path()))],
        )],
    );

    assert!(
        validate(&schema([process_entity], [process]))
            .iter()
            .any(|error| matches!(error, OsError::OsRecordEventNotOnce { .. }))
    );
}

#[test]
fn os_record_may_be_carried_by_only_one_event() {
    let process = process_record();
    let process_entity = entity(
        "MyProcess",
        [
            event(
                "Started",
                Cardinality::Once,
                [field("process", DataType::Record(process_path()))],
            ),
            event(
                "Observed",
                Cardinality::Once,
                [field("process", DataType::Record(process_path()))],
            ),
        ],
    );

    assert!(
        validate(&schema([process_entity], [process]))
            .iter()
            .any(|error| matches!(error, OsError::OsRecordUsedByMultipleEvents { .. }))
    );
}

#[test]
fn entity_cannot_represent_both_process_and_thread() {
    let process = process_record();
    let thread = thread_record();
    let entity = entity(
        "Ambiguous",
        [event(
            "Init",
            Cardinality::Once,
            [
                field("process", DataType::Record(process_path())),
                field("thread", DataType::Record(thread_path())),
            ],
        )],
    );

    assert!(
        validate(&schema([entity], [process, thread]))
            .iter()
            .any(|error| matches!(error, OsError::ConflictingEntityRoles { .. }))
    );
}

#[test]
fn os_record_may_only_be_used_by_an_entity_event() {
    let process = process_record();
    let wrapper = RecordBuilder::new(path("Wrapper"))
        .with_field(Field::new(
            ident("process"),
            DataType::Record(process_path()),
            Annotations::default(),
        ))
        .build()
        .unwrap();

    assert!(
        validate(&schema([], [process, wrapper]))
            .iter()
            .any(|error| matches!(error, OsError::OsRecordOutsideEvent { .. }))
    );
}
