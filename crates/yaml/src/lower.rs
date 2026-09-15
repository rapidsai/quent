// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Lowering from the deserialized model to a schema.
//!
//! Lowering deliberately avoids the usual `Result`/`?` path, which would stop
//! at the first error. Instead each function reports any problems into one
//! shared sink and carries on, returning what it built or `None` to skip an
//! invalid element. A single run can therefore surface problems across
//! independent declarations. Whether it succeeded is decided by the caller
//! from the sink, not from a `Result`.
//!
//! Constraint and metadata payloads are opaque: attached as written, never
//! interpreted.

use indexmap::IndexMap;
use quent_schema::builder::{
    AnnotationsBuilder, BuilderError, EntityBuilder, EventBuilder, RecordBuilder, SchemaBuilder,
};
use quent_schema::{Annotations, Cardinality, DataType, Entity, Field, Identifier, Record, Schema};
use serde::Deserialize;

use crate::ast::{self, AnnotationMap, Model, TypeExpr};
use crate::diag::Diagnostics;
use crate::extensions::{
    Elaborator as ExtensionElaborator, EventContext, FieldContext, FieldElaboration,
};

/// Lower `model` to a schema, reporting problems into `sink`.
///
/// Lowering proceeds in the following phases:
///
/// - Validate the format and model name, then prepare extension elaboration.
/// - Lower declared records.
/// - Lower entities and collect extension-generated records.
/// - Elaborate extension-owned model declarations.
/// - Insert the records, attach the entities and model annotations, and build
///   the schema.
///
/// Recoverable errors do not stop later phases, allowing one run to report
/// multiple problems. A returned schema is meaningful only when `sink`
/// contains no errors. Returns `None` when no schema can be built.
pub(crate) fn lower(model: &Model, sink: &mut Diagnostics) -> Option<Schema> {
    // Validate model-level inputs.
    if model.quent != "alpha" {
        sink.error(
            "quent",
            format!("unsupported format version `{}`", model.quent),
            Some("supported versions: alpha".to_string()),
        );
    }

    let name = ident(&model.model, "model", sink);
    // Enter built-in extension elaboration. All extension-owned schema
    // modifications are dispatched through this boundary.
    let extensions = ExtensionElaborator::new(model, sink);

    // Lower declared records.
    let mut records: Vec<Record> = model
        .records
        .iter()
        .filter_map(|(name, record)| record_of(name, record, &extensions, sink))
        .collect();

    // Lower entities and extension-generated records.
    let mut entities: Vec<Entity> = Vec::new();
    for (name, entity) in &model.entities {
        if let Some((entity, generated)) = entity_of(name, entity, &extensions, sink) {
            entities.push(entity);
            records.extend(generated);
        }
    }

    // Elaborate extension-owned model declarations and merge their schema additions.
    let model_elaboration = extensions.elaborate_model(model, sink);
    entities.extend(model_elaboration.entities);
    records.extend(model_elaboration.records);

    let schema = SchemaBuilder::new(name?)
        .with_records(records)
        .with_entities(entities)
        .with_annotations(annotations(
            &model.doc,
            &model.constraints,
            &model.metadata,
            &extensions,
            "",
            sink,
        ))
        .build();
    match schema {
        Ok(schema) => Some(schema),
        Err(error) => {
            schema_builder_diagnostics(&error, sink);
            None
        }
    }
}

fn schema_builder_diagnostics(error: &BuilderError, sink: &mut Diagnostics) {
    match error {
        BuilderError::DuplicateName(name) => {
            sink.error("", format!("duplicate type path `{name}`"), None)
        }
        error => sink.error("", error.to_string(), None),
    }
}

/// Lower one record, or `None` (after reporting) if its name is rejected.
///
/// Fields are lowered for their diagnostics even when the name is bad, but a
/// bad name skips the record so no placeholder identifier reaches the builder.
fn record_of(
    name: &str,
    record: &ast::Record,
    extensions: &ExtensionElaborator,
    sink: &mut Diagnostics,
) -> Option<Record> {
    let path = format!("records.{name}");
    let id = type_decl_ident(name, "records", sink);
    let fields = fields_of(&record.fields, &path, extensions, sink);
    let record = RecordBuilder::new(id?)
        .with_fields(fields)
        .with_annotations(annotations(
            &record.doc,
            &record.constraints,
            &record.metadata,
            extensions,
            &path,
            sink,
        ))
        .build();
    build_or_diagnose(record, &path, sink)
}

/// Lower one entity and any extension-generated records.
fn entity_of(
    name: &str,
    entity: &ast::Entity,
    extensions: &ExtensionElaborator,
    sink: &mut Diagnostics,
) -> Option<(Entity, Vec<Record>)> {
    let path = format!("entities.{name}");
    let id = type_decl_ident(name, "entities", sink);

    let mut anns = annotations_builder(
        &entity.doc,
        &entity.constraints,
        &entity.metadata,
        extensions,
        &path,
        sink,
    );

    let entity_elaboration =
        extensions.elaborate_entity(name, id.clone(), entity, anns, &path, sink);
    anns = entity_elaboration.annotations;
    let records = entity_elaboration.records;
    let event_context = entity_elaboration.event_context;

    let events: Vec<_> = entity
        .events
        .iter()
        .filter_map(|(event_name, event)| {
            event_of(event_name, event, &path, &event_context, extensions, sink)
        })
        .collect();
    match EntityBuilder::new(id?)
        .with_events(events)
        .with_annotations(build_or_diagnose(anns.build(), &path, sink).unwrap_or_default())
        .build()
    {
        Ok(entity) => Some((entity, records)),
        Err(BuilderError::NoEvents) => {
            sink.error(
                &path,
                format!("entity `{name}` declares no events"),
                Some("entities must declare at least one event".to_string()),
            );
            None
        }
        Err(error) => {
            sink.error(&path, error.to_string(), None);
            None
        }
    }
}

/// Lower one event, including extension-owned fields.
fn event_of(
    name: &str,
    event: &ast::Event,
    entity_path: &str,
    event_context: &EventContext,
    extensions: &ExtensionElaborator,
    sink: &mut Diagnostics,
) -> Option<quent_schema::Event> {
    let events_path = format!("{entity_path}.events");
    let path = format!("{events_path}.{name}");
    let id = ident(name, &events_path, sink);
    let fields = event_fields(
        &event.attributes,
        &format!("{path}.attributes"),
        event_context,
        extensions,
        sink,
    );
    let anns = annotations(
        &event.doc,
        &event.constraints,
        &event.metadata,
        extensions,
        &path,
        sink,
    );
    let cardinality = if event.multi {
        Cardinality::Multi
    } else {
        Cardinality::Once
    };
    let event = EventBuilder::new(id?, cardinality)
        .with_fields(fields)
        .with_annotations(anns)
        .build();
    build_or_diagnose(event, &path, sink)
}

/// Lower event attributes, including extension-owned fields.
pub(crate) fn event_fields(
    fields: &IndexMap<String, ast::Field>,
    path: &str,
    event_context: &EventContext,
    extensions: &ExtensionElaborator,
    sink: &mut Diagnostics,
) -> Vec<Field> {
    let mut lowered = Vec::new();
    for (name, field) in fields {
        let field_path = format!("{path}.{name}");
        let id = ident(name, &field_path, sink);
        match extensions.elaborate_field(
            &id,
            field,
            FieldContext::Event(event_context),
            &field_path,
            sink,
        ) {
            FieldElaboration::Handled(field) => lowered.push(*field),
            FieldElaboration::Rejected => {}
            FieldElaboration::NotHandled => {
                lowered.extend(field_of(id, field, &field_path, extensions, sink));
            }
        }
    }
    lowered
}

fn fields_of(
    fields: &IndexMap<String, ast::Field>,
    path: &str,
    extensions: &ExtensionElaborator,
    sink: &mut Diagnostics,
) -> Vec<Field> {
    fields
        .iter()
        .filter_map(|(name, field)| {
            let field_path = format!("{path}.{name}");
            let id = ident(name, &field_path, sink);
            field_of(id, field, &field_path, extensions, sink)
        })
        .collect()
}

fn field_of(
    id: Option<Identifier>,
    field: &ast::Field,
    path: &str,
    extensions: &ExtensionElaborator,
    sink: &mut Diagnostics,
) -> Option<Field> {
    match extensions.elaborate_field(&id, field, FieldContext::Other, path, sink) {
        FieldElaboration::Handled(field) => return Some(*field),
        FieldElaboration::Rejected => return None,
        FieldElaboration::NotHandled => {}
    }
    let (ty, ann) = match field {
        ast::Field::Bare(expr) => (
            type_of(expr, path, extensions, sink)?,
            Annotations::default(),
        ),
        ast::Field::Full(body) => {
            let ty = type_of(&body.r#type, path, extensions, sink)?;
            let ann = annotations(
                &body.doc,
                &body.constraints,
                &body.metadata,
                extensions,
                path,
                sink,
            );
            (ty, ann)
        }
        _ => unreachable!("extension-owned field was handled above"),
    };
    Some(Field::new(id?, ty, ann))
}

pub(crate) fn type_of(
    expr: &TypeExpr,
    path: &str,
    extensions: &ExtensionElaborator,
    sink: &mut Diagnostics,
) -> Option<DataType> {
    match expr {
        TypeExpr::Keyword(keyword) => Some(keyword_data_type(keyword)),
        TypeExpr::Record(name) => record_ref(name, path, sink),
        TypeExpr::List(t) => Some(DataType::List(Box::new(type_of(
            &t.list, path, extensions, sink,
        )?))),
        TypeExpr::Option(t) => Some(DataType::Option(Box::new(type_of(
            &t.option, path, extensions, sink,
        )?))),

        // Built-in extension type expressions.
        TypeExpr::Ref(_) | TypeExpr::Scope(_) | TypeExpr::Uses(_) => {
            extensions.elaborate_type(expr, path, sink)
        }
    }
}

/// A bare name that is not a core type keyword, lowered as a record reference.
fn record_ref(name: &str, path: &str, sink: &mut Diagnostics) -> Option<DataType> {
    match Identifier::try_new(name) {
        Ok(id) => Some(DataType::Record(id.into())),
        Err(e) => {
            sink.error(path, format!("invalid type `{name}`: {e}"), None);
            None
        }
    }
}

/// The `DataType` a reserved bare keyword denotes.
fn keyword_data_type(keyword: &ast::TypeKeyword) -> DataType {
    use ast::TypeKeyword::*;
    match keyword {
        Bool => DataType::Bool,
        U8 => DataType::U8,
        U16 => DataType::U16,
        U32 => DataType::U32,
        U64 => DataType::U64,
        I8 => DataType::I8,
        I16 => DataType::I16,
        I32 => DataType::I32,
        I64 => DataType::I64,
        F32 => DataType::F32,
        F64 => DataType::F64,
        String => DataType::String,
        Uuid => DataType::Uuid,
        Dynamic => DataType::DynamicRecord,
        Ref => DataType::EntityRef {
            data: None,
            annotations: Annotations::default(),
        },
    }
}

/// Whether `name` is a core type keyword.
fn is_core_type_name(name: &str) -> bool {
    use serde::de::IntoDeserializer;
    let de: serde::de::value::StrDeserializer<serde::de::value::Error> = name.into_deserializer();
    ast::TypeKeyword::deserialize(de).is_ok()
}

fn annotations(
    doc: &Option<String>,
    constraints: &AnnotationMap,
    metadata: &AnnotationMap,
    extensions: &ExtensionElaborator,
    path: &str,
    sink: &mut Diagnostics,
) -> Annotations {
    build_or_diagnose(
        annotations_builder(doc, constraints, metadata, extensions, path, sink).build(),
        path,
        sink,
    )
    .unwrap_or_default()
}

pub(crate) fn build_or_diagnose<T>(
    result: Result<T, BuilderError>,
    path: &str,
    sink: &mut Diagnostics,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            sink.error(path, error.to_string(), None);
            None
        }
    }
}

pub(crate) fn annotations_builder(
    doc: &Option<String>,
    constraints: &AnnotationMap,
    metadata: &AnnotationMap,
    extensions: &ExtensionElaborator,
    path: &str,
    sink: &mut Diagnostics,
) -> AnnotationsBuilder {
    let mut builder = match doc {
        Some(doc) => AnnotationsBuilder::new().with_docs(doc),
        None => AnnotationsBuilder::new(),
    };
    builder = extensions.elaborate_annotations(builder, constraints, path, sink);
    for (name, value) in metadata {
        builder = builder.with_metadata(name, value.clone());
    }
    builder
}

/// Validate a record/entity name, or `None` (after reporting) if it is invalid
/// or a reserved type name.
///
/// Records and entities are referenced by a bare name, so their names may not
/// shadow a core type keyword. Field and event names are never
/// in type position and use [`ident`] directly.
pub(crate) fn type_decl_ident(
    name: &str,
    path: &str,
    sink: &mut Diagnostics,
) -> Option<Identifier> {
    if is_core_type_name(name) {
        sink.error(
            path,
            format!("`{name}` is a reserved type name"),
            Some("pick a different name".to_string()),
        );
        return None;
    }
    ident(name, path, sink)
}

pub(crate) fn ident(name: &str, path: &str, sink: &mut Diagnostics) -> Option<Identifier> {
    match Identifier::try_new(name) {
        Ok(id) => Some(id),
        Err(e) => {
            sink.error(path, format!("invalid name `{name}`: {e}"), None);
            None
        }
    }
}
