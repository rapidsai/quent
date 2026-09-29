// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Canonical schema vocabulary for captured NVTX events.
//!
//! [`compose`] adds a single model-level [`NvtxEvent`](nvtx_event_path)
//! entity and its lossless records to an application schema. Each runtime
//! stream entity binds to an OS-process entity instance through its once-only
//! `initialized.process` reference. [`validated_bindings`] gives generators a
//! verified, typed view of those definitions.

use quent_constraints::{
    Constraint,
    utils::{RecordValidationError, RecordValidator, bullet_list},
};
use quent_os::{OsConstraint, OsError, process_path as os_process_path};
use quent_ref_target::RefTargetConstraint;
use quent_schema::{
    Cardinality, DataType, Entity, Event, Field, Identifier, Path, Record, Schema,
    builder::{BuilderError, SchemaBuilder},
    visitor::{Cursor, Element, Visitor},
};
use thiserror::Error;

mod definition;

pub use definition::{
    CAPTURE_EVENT_NAMES, INITIALIZED_EVENT, MESSAGE_KIND_REGISTERED_HANDLE, MESSAGE_KIND_STRING,
    PAYLOAD_VALUE_KIND_DOUBLE, PAYLOAD_VALUE_KIND_FLOAT, PAYLOAD_VALUE_KIND_INT32,
    PAYLOAD_VALUE_KIND_INT64, PAYLOAD_VALUE_KIND_POINTER, PAYLOAD_VALUE_KIND_UNSIGNED_INT32,
    PAYLOAD_VALUE_KIND_UNSIGNED_INT64, attributes_path, attributes_record, color_path,
    color_record, message_path, message_record, nvtx_event_entity, nvtx_event_path, payload_path,
    payload_record,
};

/// Validates the canonical NVTX entity, records, and process binding.
///
/// Validation must be registered together with [`OsConstraint`],
/// [`RefTargetConstraint`], and [`quent_ref_tree::RefTreeConstraint`], and
/// callers must also check base constraints.
#[derive(Default)]
pub struct NvtxConstraint {
    schema: Option<Schema>,
}

impl Constraint for NvtxConstraint {
    const NAME: &'static str = "quent.nvtx.v0.1.0";
}

impl Visitor for NvtxConstraint {
    type Output = Result<(), NvtxError>;

    fn visit(&mut self, cursor: &Cursor) {
        if let Element::Schema(schema) = cursor.current() {
            self.schema = Some(schema.clone());
        }
    }

    fn finish(self) -> Self::Output {
        let Some(schema) = self.schema else {
            return Ok(());
        };
        validated_bindings(&schema).map(|_| ())
    }
}

/// Canonical captured NVTX events, in semantic rather than declaration form.
#[derive(Debug, Clone, Copy)]
pub struct NvtxEvents<'a> {
    pub range_push: &'a Event,
    pub range_pop: &'a Event,
    pub range_start: &'a Event,
    pub range_end: &'a Event,
    pub mark: &'a Event,
    pub domain_create: &'a Event,
    pub domain_destroy: &'a Event,
    pub register_string: &'a Event,
    pub name_category: &'a Event,
    pub name_thread: &'a Event,
    pub resource_create: &'a Event,
    pub resource_destroy: &'a Event,
}

impl<'a> NvtxEvents<'a> {
    /// Iterate over captured events in canonical declaration order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'a Event> {
        [
            self.range_push,
            self.range_pop,
            self.range_start,
            self.range_end,
            self.mark,
            self.domain_create,
            self.domain_destroy,
            self.register_string,
            self.name_category,
            self.name_thread,
            self.resource_create,
            self.resource_destroy,
        ]
        .into_iter()
    }
}

/// Canonical lossless NVTX records.
#[derive(Debug, Clone, Copy)]
pub struct NvtxRecords<'a> {
    pub color: &'a Record,
    pub message: &'a Record,
    pub payload: &'a Record,
    pub attributes: &'a Record,
}

/// A fully validated canonical NVTX schema extension.
///
/// Generators should consume this descriptor instead of rediscovering event or
/// record roles from names and field shapes.
#[derive(Debug)]
pub struct NvtxBindings<'a> {
    pub entity: &'a Entity,
    pub initialized: &'a Event,
    pub process_field: &'a Field,
    pub process_target: Path,
    pub events: NvtxEvents<'a>,
    pub records: NvtxRecords<'a>,
}

/// Validate and return the canonical NVTX definitions in `schema`.
///
/// Returns `Ok(None)` when the schema contains none of the reserved NVTX
/// definitions. A partial extension is an error. The selected process target
/// must participate in a valid [`OsConstraint`] schema and directly carry the
/// canonical process record in a once-only event.
pub fn validated_bindings(schema: &Schema) -> Result<Option<NvtxBindings<'_>>, NvtxError> {
    if !has_any_reserved_definition(schema) {
        return Ok(None);
    }

    let mut errors = Vec::new();
    let entity = match schema.entity(&nvtx_event_path()) {
        Some(entity) => Some(entity),
        None => {
            errors.push(NvtxError::MissingCanonicalEntity {
                entity: nvtx_event_path(),
            });
            None
        }
    };

    let color = checked_record(schema, color_record(), &mut errors);
    let message = checked_record(schema, message_record(), &mut errors);
    let payload = checked_record(schema, payload_record(), &mut errors);
    let attributes = checked_record(schema, attributes_record(), &mut errors);

    let mut initialized = None;
    let mut process_field = None;
    let mut process_target = None;
    let mut events = None;

    if let Some(entity) = entity {
        if !entity.annotations().has_constraint(NvtxConstraint::NAME) {
            errors.push(NvtxError::MissingConstraintAnnotation {
                definition: entity.path().clone(),
            });
        }

        match extract_process_binding(entity) {
            Ok((event, field, target)) => {
                let expected = nvtx_event_entity(&target);
                if entity != &expected {
                    errors.push(NvtxError::InvalidCanonicalEntity {
                        entity: entity.path().clone(),
                    });
                }
                initialized = Some(event);
                process_field = Some(field);
                process_target = Some(target);
            }
            Err(error) => errors.push(NvtxError::InvalidProcessBinding(error)),
        }

        events = capture_events(entity);
    }

    if let Some(target) = process_target.as_ref()
        && let Err(error) = validate_process_target(schema, target)
    {
        errors.push(error);
    }

    if !errors.is_empty() {
        return Err(combine_errors(errors));
    }

    Ok(Some(NvtxBindings {
        entity: entity.expect("missing entity produced an error"),
        initialized: initialized.expect("invalid binding produced an error"),
        process_field: process_field.expect("invalid binding produced an error"),
        process_target: process_target.expect("invalid binding produced an error"),
        events: events.expect("invalid entity produced an error"),
        records: NvtxRecords {
            color: color.expect("missing record produced an error"),
            message: message.expect("missing record produced an error"),
            payload: payload.expect("missing record produced an error"),
            attributes: attributes.expect("missing record produced an error"),
        },
    }))
}

/// Return the process entity type selected by a validated NVTX extension.
pub fn bound_process(schema: &Schema) -> Result<Option<Path>, NvtxError> {
    Ok(validated_bindings(schema)?.map(|bindings| bindings.process_target))
}

fn has_any_reserved_definition(schema: &Schema) -> bool {
    schema.entity(&nvtx_event_path()).is_some()
        || [
            color_path(),
            message_path(),
            payload_path(),
            attributes_path(),
        ]
        .iter()
        .any(|path| schema.record(path).is_some() || schema.entity(path).is_some())
        || schema.record(&nvtx_event_path()).is_some()
}

fn checked_record<'a>(
    schema: &'a Schema,
    expected: Record,
    errors: &mut Vec<NvtxError>,
) -> Option<&'a Record> {
    if schema.entity(expected.path()).is_some() {
        errors.push(NvtxError::ReservedPathHasWrongKind {
            path: expected.path().clone(),
            expected: "record",
        });
        return None;
    }
    let Some(actual) = schema.record(expected.path()) else {
        errors.push(NvtxError::MissingCanonicalRecord {
            record: expected.path().clone(),
        });
        return None;
    };
    if !actual.annotations().has_constraint(NvtxConstraint::NAME) {
        errors.push(NvtxError::MissingConstraintAnnotation {
            definition: actual.path().clone(),
        });
    }
    errors.extend(
        RecordValidator::new(&expected)
            .validate_exact(actual)
            .into_iter()
            .map(NvtxError::InvalidCanonicalRecord),
    );
    Some(actual)
}

fn extract_process_binding(entity: &Entity) -> Result<(&Event, &Field, Path), ProcessBindingError> {
    let initialized_name = Identifier::try_new(INITIALIZED_EVENT).expect("canonical name is valid");
    let initialized = entity
        .event(&initialized_name)
        .ok_or(ProcessBindingError::MissingInitialized)?;
    if initialized.cardinality() != Cardinality::Once {
        return Err(ProcessBindingError::InitializedNotOnce);
    }
    let process_name = Identifier::try_new("process").expect("canonical name is valid");
    let process = initialized
        .field(&process_name)
        .ok_or(ProcessBindingError::MissingProcessField)?;
    let DataType::EntityRef { data, annotations } = process.ty() else {
        return Err(ProcessBindingError::ProcessFieldNotEntityRef);
    };
    if data.is_some() {
        return Err(ProcessBindingError::ProcessReferenceCarriesData);
    }
    let constraint = annotations
        .constraint(RefTargetConstraint::NAME)
        .ok_or(ProcessBindingError::MissingRefTarget)?;
    let raw = constraint
        .data()
        .ok_or(ProcessBindingError::MissingRefTargetData)?;
    let target = raw
        .parse::<Path>()
        .map_err(|_| ProcessBindingError::InvalidRefTarget(raw.to_string()))?;
    Ok((initialized, process, target))
}

fn capture_events(entity: &Entity) -> Option<NvtxEvents<'_>> {
    let event = |name: &str| {
        entity.event(&Identifier::try_new(name).expect("canonical event name is valid"))
    };
    Some(NvtxEvents {
        range_push: event("range_push")?,
        range_pop: event("range_pop")?,
        range_start: event("range_start")?,
        range_end: event("range_end")?,
        mark: event("mark")?,
        domain_create: event("domain_create")?,
        domain_destroy: event("domain_destroy")?,
        register_string: event("register_string")?,
        name_category: event("name_category")?,
        name_thread: event("name_thread")?,
        resource_create: event("resource_create")?,
        resource_destroy: event("resource_destroy")?,
    })
}

fn validate_process_target(schema: &Schema, target: &Path) -> Result<(), NvtxError> {
    if schema.entity(target).is_none() {
        return Err(NvtxError::UnknownProcessTarget {
            target: target.clone(),
        });
    }

    schema
        .walk(OsConstraint::default())
        .map_err(|error| NvtxError::InvalidOsSchema(Box::new(error)))?;

    let process_record_declared = schema.record(&os_process_path()).is_some();
    let has_process_role = schema
        .entity(target)
        .into_iter()
        .flat_map(Entity::events)
        .filter(|event| event.cardinality() == Cardinality::Once)
        .flat_map(Event::fields)
        .any(|field| field.ty() == &DataType::Record(os_process_path()));

    if !process_record_declared || !has_process_role {
        return Err(NvtxError::TargetIsNotOsProcess {
            target: target.clone(),
        });
    }
    Ok(())
}

/// Add the canonical NVTX extension to `schema`, bound to `process_target`.
///
/// Existing definitions are retained in declaration order and missing NVTX
/// definitions are appended deterministically. Exact definitions for the same
/// target are accepted, making composition idempotent. Reserved-name
/// collisions and attempts to rebind an existing stream type are rejected.
pub fn compose(schema: &Schema, process_target: &Path) -> Result<Schema, ComposeError> {
    validate_process_target(schema, process_target)
        .map_err(|error| ComposeError::InvalidProcessTarget(Box::new(error)))?;

    let expected_entity = nvtx_event_entity(process_target);
    let expected_records = [
        color_record(),
        message_record(),
        payload_record(),
        attributes_record(),
    ];

    if schema.record(&nvtx_event_path()).is_some() {
        return Err(ComposeError::ReservedPathHasWrongKind {
            path: nvtx_event_path(),
            expected: "entity",
        });
    }
    if let Some(actual) = schema.entity(&nvtx_event_path())
        && actual != &expected_entity
    {
        if let Ok((_, _, existing_target)) = extract_process_binding(actual)
            && existing_target != *process_target
        {
            return Err(ComposeError::DifferentProcessTarget {
                existing: existing_target,
                requested: process_target.clone(),
            });
        }
        return Err(ComposeError::ConflictingEntity {
            entity: nvtx_event_path(),
        });
    }

    for expected in &expected_records {
        if schema.entity(expected.path()).is_some() {
            return Err(ComposeError::ReservedPathHasWrongKind {
                path: expected.path().clone(),
                expected: "record",
            });
        }
        if let Some(actual) = schema.record(expected.path())
            && actual != expected
        {
            return Err(ComposeError::ConflictingRecord {
                record: expected.path().clone(),
            });
        }
    }

    let mut builder = SchemaBuilder::new(schema.name().clone())
        .with_annotations(schema.annotations().clone())
        .with_entities(schema.entities().cloned())
        .with_records(schema.records().cloned());
    if schema.entity(&nvtx_event_path()).is_none() {
        builder = builder.with_entity(expected_entity);
    }
    for record in expected_records {
        if schema.record(record.path()).is_none() {
            builder = builder.with_record(record);
        }
    }

    let composed = builder.build()?;
    validated_bindings(&composed)
        .map_err(|error| ComposeError::InvalidComposedSchema(Box::new(error)))?;
    Ok(composed)
}

fn combine_errors(mut errors: Vec<NvtxError>) -> NvtxError {
    match errors.len() {
        1 => errors.pop().expect("length checked"),
        _ => NvtxError::Multiple(errors),
    }
}

/// A malformed `initialized.process` source binding.
#[derive(Debug, Error)]
pub enum ProcessBindingError {
    #[error("canonical NVTX entity is missing its `initialized` event")]
    MissingInitialized,
    #[error("canonical NVTX `initialized` event must have `Once` cardinality")]
    InitializedNotOnce,
    #[error("canonical NVTX `initialized` event is missing its `process` field")]
    MissingProcessField,
    #[error("canonical NVTX `initialized.process` field must be an entity reference")]
    ProcessFieldNotEntityRef,
    #[error("canonical NVTX `initialized.process` reference must not carry data")]
    ProcessReferenceCarriesData,
    #[error(
        "canonical NVTX `initialized.process` reference is missing `{}`",
        RefTargetConstraint::NAME
    )]
    MissingRefTarget,
    #[error("canonical NVTX `initialized.process` ref-target data is missing")]
    MissingRefTargetData,
    #[error("canonical NVTX `initialized.process` ref-target `{0}` is invalid")]
    InvalidRefTarget(String),
}

/// Error produced while validating the canonical NVTX extension.
#[derive(Debug, Error)]
pub enum NvtxError {
    #[error("canonical NVTX entity `{entity}` is missing")]
    MissingCanonicalEntity { entity: Path },
    #[error("canonical NVTX record `{record}` is missing")]
    MissingCanonicalRecord { record: Path },
    #[error(
        "canonical NVTX definition `{definition}` must carry constraint `{}`",
        NvtxConstraint::NAME
    )]
    MissingConstraintAnnotation { definition: Path },
    #[error("reserved NVTX path `{path}` must declare a {expected}")]
    ReservedPathHasWrongKind { path: Path, expected: &'static str },
    #[error("canonical NVTX entity `{entity}` does not match the required shape")]
    InvalidCanonicalEntity { entity: Path },
    #[error(transparent)]
    InvalidCanonicalRecord(#[from] RecordValidationError),
    #[error(transparent)]
    InvalidProcessBinding(#[from] ProcessBindingError),
    #[error("NVTX process binding points at unknown entity `{target}`")]
    UnknownProcessTarget { target: Path },
    #[error("NVTX process binding target `{target}` is not an OS-process entity")]
    TargetIsNotOsProcess { target: Path },
    #[error("NVTX process binding participates in an invalid OS schema: {0}")]
    InvalidOsSchema(#[source] Box<OsError>),
    #[error("multiple NVTX constraint violations:\n{}", bullet_list(.0))]
    Multiple(Vec<NvtxError>),
}

/// Error produced while composing the canonical NVTX extension.
#[derive(Debug, Error)]
pub enum ComposeError {
    #[error("invalid NVTX process target: {0}")]
    InvalidProcessTarget(#[source] Box<NvtxError>),
    #[error("reserved NVTX path `{path}` must declare a {expected}")]
    ReservedPathHasWrongKind { path: Path, expected: &'static str },
    #[error("canonical NVTX entity `{entity}` conflicts with the requested definition")]
    ConflictingEntity { entity: Path },
    #[error("canonical NVTX record `{record}` conflicts with the requested definition")]
    ConflictingRecord { record: Path },
    #[error("canonical NVTX stream is already bound to `{existing}`, not `{requested}`")]
    DifferentProcessTarget { existing: Path, requested: Path },
    #[error("composed NVTX schema failed validation: {0}")]
    InvalidComposedSchema(#[source] Box<NvtxError>),
    #[error(transparent)]
    Build(#[from] BuilderError),
}
