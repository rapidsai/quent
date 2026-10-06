// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use nvtx_schema::{
    CAPTURE_EVENT_NAMES, ComposeError, MESSAGE_KIND_REGISTERED_HANDLE, MESSAGE_KIND_STRING,
    NvtxConstraint, NvtxError, PAYLOAD_VALUE_KIND_DOUBLE, PAYLOAD_VALUE_KIND_FLOAT,
    PAYLOAD_VALUE_KIND_INT32, PAYLOAD_VALUE_KIND_INT64, PAYLOAD_VALUE_KIND_POINTER,
    PAYLOAD_VALUE_KIND_UNSIGNED_INT32, PAYLOAD_VALUE_KIND_UNSIGNED_INT64, attributes_path,
    attributes_record, bound_process, color_path, color_record, compose, message_path,
    message_record, nvtx_event_entity, nvtx_event_path, payload_path, payload_record,
    validated_bindings,
};
use quent_constraints::{Constraint as _, validate};
use quent_os::{OsConstraint, process_path, process_record, thread_path, thread_record};
use quent_ref_target::RefTargetConstraint;
use quent_ref_tree::RefTreeConstraint;
use quent_schema::{
    Annotations, Cardinality, DataType, Entity, Field, Path, Record, Schema,
    builder::{AnnotationsBuilder, EntityBuilder, EventBuilder, RecordBuilder, SchemaBuilder},
    test_utils::{field, ident, path},
};

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

fn process_entity(name: &str) -> Entity {
    EntityBuilder::new(path(name))
        .with_event(event(
            "started",
            Cardinality::Once,
            [field("process", DataType::Record(process_path()))],
        ))
        .build()
        .unwrap()
}

fn scoped_process_entity(name: &str, parent: &str) -> Entity {
    let parent = DataType::EntityRef {
        data: None,
        annotations: AnnotationsBuilder::new()
            .with_constraint(RefTargetConstraint::NAME, Some(parent.to_string()))
            .with_constraint(RefTreeConstraint::NAME, None)
            .build()
            .unwrap(),
    };
    EntityBuilder::new(path(name))
        .with_event(event(
            "started",
            Cardinality::Once,
            [
                field("process", DataType::Record(process_path())),
                field("parent", parent),
            ],
        ))
        .build()
        .unwrap()
}

fn combined_process_thread_entity(name: &str) -> Entity {
    EntityBuilder::new(path(name))
        .with_event(event(
            "started",
            Cardinality::Once,
            [
                field("process", DataType::Record(process_path())),
                field("thread", DataType::Record(thread_path())),
            ],
        ))
        .build()
        .unwrap()
}

fn ordinary_entity(name: &str) -> Entity {
    EntityBuilder::new(path(name))
        .with_event(event(
            "observed",
            Cardinality::Multi,
            [field("value", DataType::U64)],
        ))
        .build()
        .unwrap()
}

fn schema(entities: impl IntoIterator<Item = Entity>, records: Vec<Record>) -> Schema {
    SchemaBuilder::new(ident("Application"))
        .with_entities(entities)
        .with_records(records)
        .build()
        .unwrap()
}

fn process_schema(name: &str) -> Schema {
    schema([process_entity(name)], vec![process_record()])
}

fn assert_fully_valid(schema: &Schema) {
    let report = validate::<(
        OsConstraint,
        NvtxConstraint,
        RefTargetConstraint,
        RefTreeConstraint,
    )>(schema);
    assert!(
        report.base_constraints.is_ok(),
        "{:?}",
        report.base_constraints
    );
    assert!(
        report.unregistered_constraints.is_empty(),
        "{:?}",
        report.unregistered_constraints
    );
    let (os, nvtx, ref_target, ref_tree) = report.results;
    assert!(os.is_ok(), "{os:?}");
    assert!(nvtx.is_ok(), "{nvtx:?}");
    assert!(ref_target.is_ok(), "{ref_target:?}");
    assert!(ref_tree.is_ok(), "{ref_tree:?}");
}

#[test]
fn composition_adds_the_exact_canonical_vocabulary() {
    let process = path("ApplicationProcess");
    let input = process_schema("ApplicationProcess");

    let composed = compose(&input, &process).unwrap();
    let bindings = validated_bindings(&composed).unwrap().unwrap();

    assert_eq!(bindings.entity, &nvtx_event_entity(&process));
    assert_eq!(bindings.initialized.cardinality(), Cardinality::Once);
    assert_eq!(bindings.process_target, process);
    assert_eq!(bindings.records.color, &color_record());
    assert_eq!(bindings.records.message, &message_record());
    assert_eq!(bindings.records.payload, &payload_record());
    assert_eq!(bindings.records.attributes, &attributes_record());

    let event_names: Vec<_> = bindings
        .events
        .iter()
        .map(|event| event.name().to_string())
        .collect();
    assert_eq!(
        event_names,
        CAPTURE_EVENT_NAMES.map(str::to_string).to_vec()
    );
    assert!(
        bindings
            .events
            .iter()
            .all(|event| event.cardinality() == Cardinality::Multi)
    );
    assert_eq!(composed.entities().count(), input.entities().count() + 1);
    assert_eq!(composed.records().count(), input.records().count() + 4);
    assert_eq!(
        bound_process(&composed).unwrap(),
        Some(path("ApplicationProcess"))
    );
    assert_fully_valid(&composed);
}

#[test]
fn canonical_records_preserve_native_tags_and_bits() {
    assert_eq!(
        [MESSAGE_KIND_STRING, MESSAGE_KIND_REGISTERED_HANDLE],
        [0, 1]
    );
    assert_eq!(
        [
            PAYLOAD_VALUE_KIND_UNSIGNED_INT64,
            PAYLOAD_VALUE_KIND_INT64,
            PAYLOAD_VALUE_KIND_DOUBLE,
            PAYLOAD_VALUE_KIND_UNSIGNED_INT32,
            PAYLOAD_VALUE_KIND_INT32,
            PAYLOAD_VALUE_KIND_FLOAT,
            PAYLOAD_VALUE_KIND_POINTER,
        ],
        [0, 1, 2, 3, 4, 5, 6]
    );

    let color = color_record();
    assert_eq!(color.path(), &color_path());
    assert_eq!(
        color.field(&ident("color_type")).unwrap().ty(),
        &DataType::I32
    );
    assert_eq!(color.field(&ident("value")).unwrap().ty(), &DataType::U32);

    let message = message_record();
    assert_eq!(message.path(), &message_path());
    assert_eq!(message.field(&ident("kind")).unwrap().ty(), &DataType::U8);
    assert_eq!(
        message.field(&ident("string")).unwrap().ty(),
        &DataType::Option(Box::new(DataType::String))
    );
    assert_eq!(
        message.field(&ident("registered_handle")).unwrap().ty(),
        &DataType::Option(Box::new(DataType::U64))
    );

    let payload = payload_record();
    assert_eq!(payload.path(), &payload_path());
    assert_eq!(
        payload.field(&ident("payload_type")).unwrap().ty(),
        &DataType::I32
    );
    assert_eq!(
        payload.field(&ident("value_kind")).unwrap().ty(),
        &DataType::U8
    );
    assert_eq!(
        payload.field(&ident("value_bits")).unwrap().ty(),
        &DataType::U64
    );

    let attributes = attributes_record();
    assert_eq!(attributes.path(), &attributes_path());
    assert_eq!(
        attributes.field(&ident("category")).unwrap().ty(),
        &DataType::U32
    );
    assert_eq!(
        attributes.field(&ident("color")).unwrap().ty(),
        &DataType::Option(Box::new(DataType::Record(color_path())))
    );
    assert_eq!(
        attributes.field(&ident("message")).unwrap().ty(),
        &DataType::Option(Box::new(DataType::Record(message_path())))
    );
    assert_eq!(
        attributes.field(&ident("payload")).unwrap().ty(),
        &DataType::Option(Box::new(DataType::Record(payload_path())))
    );
}

#[test]
fn composition_is_idempotent_for_the_same_target() {
    let process = path("ProcessType");
    let once = compose(&process_schema("ProcessType"), &process).unwrap();
    let twice = compose(&once, &process).unwrap();

    assert_eq!(twice, once);
}

#[test]
fn exact_preexisting_definitions_are_reused() {
    let process = path("ProcessType");
    let input = schema(
        [process_entity("ProcessType"), nvtx_event_entity(&process)],
        vec![
            process_record(),
            color_record(),
            message_record(),
            payload_record(),
            attributes_record(),
        ],
    );

    assert_eq!(compose(&input, &process).unwrap(), input);
}

#[test]
fn exact_partial_definitions_are_completed() {
    let process = path("ProcessType");
    let input = schema(
        [process_entity("ProcessType")],
        vec![process_record(), color_record(), message_record()],
    );

    let composed = compose(&input, &process).unwrap();

    assert!(composed.record(&color_path()).is_some());
    assert!(composed.record(&message_path()).is_some());
    assert!(composed.record(&payload_path()).is_some());
    assert!(composed.record(&attributes_path()).is_some());
    assert!(composed.entity(&nvtx_event_path()).is_some());
    assert_fully_valid(&composed);
}

#[test]
fn conflicting_reserved_record_is_rejected() {
    let process = path("ProcessType");
    let conflicting_color = RecordBuilder::new(color_path())
        .with_field(field("value", DataType::U64))
        .build()
        .unwrap();
    let input = schema(
        [process_entity("ProcessType")],
        vec![process_record(), conflicting_color],
    );

    assert!(matches!(
        compose(&input, &process),
        Err(ComposeError::ConflictingRecord { record }) if record == color_path()
    ));
}

#[test]
fn conflicting_reserved_entity_is_rejected() {
    let process = path("ProcessType");
    let conflicting = EntityBuilder::new(nvtx_event_path())
        .with_event(event("other", Cardinality::Multi, []))
        .build()
        .unwrap();
    let input = schema(
        [process_entity("ProcessType"), conflicting],
        vec![process_record()],
    );

    assert!(matches!(
        compose(&input, &process),
        Err(ComposeError::ConflictingEntity { entity }) if entity == nvtx_event_path()
    ));
}

#[test]
fn existing_stream_cannot_be_rebound_to_another_process_type() {
    let first = path("FirstProcess");
    let second = path("SecondProcess");
    let input = schema(
        [
            process_entity("FirstProcess"),
            scoped_process_entity("SecondProcess", "FirstProcess"),
        ],
        vec![process_record()],
    );
    let composed = compose(&input, &first).unwrap();

    assert!(matches!(
        compose(&composed, &second),
        Err(ComposeError::DifferentProcessTarget { existing, requested })
            if existing == first && requested == second
    ));
}

#[test]
fn selected_target_must_be_an_os_process() {
    let target = path("Worker");
    let input = schema([ordinary_entity("Worker")], vec![process_record()]);

    assert!(matches!(
        compose(&input, &target),
        Err(ComposeError::InvalidProcessTarget(error))
            if contains_target_not_process(&error, &target)
    ));
}

fn contains_target_not_process(error: &NvtxError, target: &Path) -> bool {
    match error {
        NvtxError::TargetIsNotOsProcess { target: actual } => actual == target,
        NvtxError::Multiple(errors) => errors
            .iter()
            .any(|error| contains_target_not_process(error, target)),
        _ => false,
    }
}

#[test]
fn combined_process_and_main_thread_is_a_valid_target() {
    let target = path("MainThread");
    let input = schema(
        [combined_process_thread_entity("MainThread")],
        vec![process_record(), thread_record()],
    );

    let composed = compose(&input, &target).unwrap();

    assert_eq!(bound_process(&composed).unwrap(), Some(target));
    assert_fully_valid(&composed);
}

#[test]
fn one_of_multiple_process_types_is_selected_explicitly() {
    let selected = path("SecondProcess");
    let input = schema(
        [
            process_entity("FirstProcess"),
            scoped_process_entity("SecondProcess", "FirstProcess"),
        ],
        vec![process_record()],
    );

    let composed = compose(&input, &selected).unwrap();
    let bindings = validated_bindings(&composed).unwrap().unwrap();

    assert_eq!(bindings.process_target, selected);
    assert!(composed.entity(&path("FirstProcess")).is_some());
    assert!(composed.entity(&path("SecondProcess")).is_some());
    assert_fully_valid(&composed);
}

#[test]
fn schemas_without_nvtx_have_no_binding() {
    let input = process_schema("ProcessType");

    assert!(validated_bindings(&input).unwrap().is_none());
    assert_eq!(bound_process(&input).unwrap(), None);
}

#[test]
fn partial_reserved_vocabulary_is_invalid_until_composed() {
    let input = schema(
        [process_entity("ProcessType")],
        vec![process_record(), color_record()],
    );

    assert!(matches!(
        validated_bindings(&input),
        Err(NvtxError::Multiple(_))
    ));
}

#[test]
fn initialized_binding_to_non_process_fails_constraint_validation() {
    let target = path("Worker");
    let input = schema(
        [ordinary_entity("Worker"), nvtx_event_entity(&target)],
        vec![
            process_record(),
            color_record(),
            message_record(),
            payload_record(),
            attributes_record(),
        ],
    );

    let error = validated_bindings(&input).unwrap_err();
    assert!(contains_target_not_process(&error, &target));
}

#[test]
fn input_schema_annotations_and_declaration_order_are_preserved() {
    let process = path("ProcessType");
    let annotations = quent_schema::builder::AnnotationsBuilder::new()
        .with_docs("application docs")
        .build()
        .unwrap();
    let input = SchemaBuilder::new(ident("Application"))
        .with_annotations(annotations.clone())
        .with_entities([process_entity("ProcessType"), ordinary_entity("Existing")])
        .with_record(process_record())
        .build()
        .unwrap();

    let composed = compose(&input, &process).unwrap();
    let entity_paths: Vec<_> = composed
        .entities()
        .map(|entity| entity.path().clone())
        .collect();
    let record_paths: Vec<_> = composed
        .records()
        .map(|record| record.path().clone())
        .collect();

    assert_eq!(composed.annotations(), &annotations);
    assert_eq!(
        entity_paths,
        vec![path("ProcessType"), path("Existing"), nvtx_event_path()]
    );
    assert_eq!(
        record_paths,
        vec![
            process_path(),
            color_path(),
            message_path(),
            payload_path(),
            attributes_path(),
        ]
    );
}

#[test]
fn wrong_type_at_reserved_record_path_is_rejected() {
    let process = path("ProcessType");
    let wrong_kind = EntityBuilder::new(color_path())
        .with_event(event("observed", Cardinality::Multi, []))
        .build()
        .unwrap();
    let input = schema(
        [process_entity("ProcessType"), wrong_kind],
        vec![process_record()],
    );

    assert!(matches!(
        compose(&input, &process),
        Err(ComposeError::ReservedPathHasWrongKind { path, expected: "record" })
            if path == color_path()
    ));
}

#[test]
fn process_reference_carries_no_payload_data() {
    let process = path("ProcessType");
    let composed = compose(&process_schema("ProcessType"), &process).unwrap();
    let bindings = validated_bindings(&composed).unwrap().unwrap();

    assert!(matches!(
        bindings.process_field.ty(),
        DataType::EntityRef {
            data: None,
            annotations,
        } if annotations.has_constraint(RefTargetConstraint::NAME)
            && annotations.has_constraint(RefTreeConstraint::NAME)
    ));
    assert_eq!(
        bindings.process_field.annotations(),
        &Annotations::default()
    );
}
