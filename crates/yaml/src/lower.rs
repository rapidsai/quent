// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Lowering from the deserialized model to a schema.
//!
//! Lowering deliberately avoids the usual `Result`/`?` path, which would stop
//! at the first error. Instead each function reports any problems into one
//! shared sink and carries on — returning what it built, or `None` (skip the
//! element) or a stand-in when a name is rejected — so a single run surfaces
//! every problem in the file, not just the first. Whether it succeeded is
//! decided by the caller from the sink, not from a `Result`.
//!
//! Constraint and metadata payloads are opaque: attached as written, never
//! interpreted.

use indexmap::IndexMap;
use quent_schema::builder::{
    AnnotationsBuilder, EntityBuilder, EventBuilder, RecordBuilder, SchemaBuilder,
};
use quent_schema::{Annotations, DataType, Entity, Field, Identifier, Record, Schema};
use serde::Deserialize;

use crate::ast::{self, AnnotationMap, Model, TypeExpr};
use crate::diag::Diagnostics;

/// Lower `model` to a schema, reporting problems into `sink`.
///
/// Always returns a schema, but it is only meaningful when `sink` reports no
/// errors; on any error the caller discards it.
pub(crate) fn lower(model: &Model, sink: &mut Diagnostics) -> Schema {
    if model.quent != "alpha" {
        sink.error(
            "quent",
            format!("unsupported format version `{}`", model.quent),
            Some("supported versions: alpha".to_string()),
        );
    }

    // Keep going after a bad name so one run reports every problem, not just the
    // first. The caller discards this schema whenever the sink holds errors, so
    // building it with a valid stand-in name here is harmless.
    let name = ident(&model.model, "model", sink)
        .unwrap_or_else(|| Identifier::try_new("invalid").expect("stand-in is valid"));

    let records: Vec<Record> = model
        .records
        .iter()
        .filter_map(|(name, record)| record_of(name, record, sink))
        .collect();
    let entities: Vec<Entity> = model
        .entities
        .iter()
        .filter_map(|(name, entity)| entity_of(name, entity, sink))
        .collect();

    SchemaBuilder::new(name)
        .try_with_records(records)
        .expect("record names are unique (deserialized from a map)")
        .try_with_entities(entities)
        .expect("entity names are unique (deserialized from a map)")
        .with_annotations(annotations(
            &model.doc,
            &model.constraints,
            &model.metadata,
            "",
            sink,
        ))
        .build()
}

/// Lower one record, or `None` (after reporting) if its name is rejected.
///
/// Fields are lowered for their diagnostics even when the name is bad, but a
/// bad name skips the record so no placeholder identifier reaches the builder.
fn record_of(name: &str, record: &ast::Record, sink: &mut Diagnostics) -> Option<Record> {
    let path = format!("records.{name}");
    let id = type_decl_ident(name, "records", sink);
    let fields = fields_of(&record.fields, &path, sink);
    Some(
        RecordBuilder::new(id?)
            .try_with_fields(fields)
            .expect("field names are unique")
            .with_annotations(annotations(
                &record.doc,
                &record.constraints,
                &record.metadata,
                &path,
                sink,
            ))
            .build(),
    )
}

/// Lower one entity, or `None` (after reporting) if its name is rejected.
fn entity_of(name: &str, entity: &ast::Entity, sink: &mut Diagnostics) -> Option<Entity> {
    let path = format!("entities.{name}");
    let id = type_decl_ident(name, "entities", sink);
    let events: Vec<_> = entity
        .events
        .iter()
        .filter_map(|(event_name, event)| event_of(event_name, event, &path, sink))
        .collect();
    Some(
        EntityBuilder::new(id?)
            .try_with_events(events)
            .expect("event names are unique")
            .with_annotations(annotations(
                &entity.doc,
                &entity.constraints,
                &entity.metadata,
                &path,
                sink,
            ))
            .build(),
    )
}

/// Lower one event, or `None` (after reporting) if its name is rejected.
fn event_of(
    name: &str,
    event: &ast::Event,
    entity_path: &str,
    sink: &mut Diagnostics,
) -> Option<quent_schema::Event> {
    let events_path = format!("{entity_path}.events");
    let path = format!("{events_path}.{name}");
    let id = ident(name, &events_path, sink);
    match event {
        ast::Event::OneLiner(card) => Some(EventBuilder::new(id?, (*card).into()).build()),
        ast::Event::Body(body) => {
            let (card, payload_key, payload) = match (&body.once, &body.multi) {
                (Some(_), Some(_)) => {
                    sink.error(
                        &path,
                        "event declares both `once` and `multi`",
                        Some("keep exactly one".to_string()),
                    );
                    (ast::Cardinality::Once, "once", &body.once)
                }
                (Some(_), None) => (ast::Cardinality::Once, "once", &body.once),
                (None, Some(_)) => (ast::Cardinality::Multi, "multi", &body.multi),
                (None, None) => {
                    sink.error(
                        &path,
                        "event must declare a cardinality",
                        Some("add `once:` or `multi:`, or write `name: once`".to_string()),
                    );
                    (ast::Cardinality::Once, "once", &body.once)
                }
            };
            let fields = match payload.as_ref().and_then(|p| p.as_ref()) {
                Some(map) => fields_of(map, &format!("{path}.{payload_key}"), sink),
                None => Vec::new(),
            };
            let anns = annotations(&body.doc, &body.constraints, &body.metadata, &path, sink);
            Some(
                EventBuilder::new(id?, card.into())
                    .try_with_fields(fields)
                    .expect("field names are unique")
                    .with_annotations(anns)
                    .build(),
            )
        }
    }
}

fn fields_of(
    fields: &IndexMap<String, ast::Field>,
    path: &str,
    sink: &mut Diagnostics,
) -> Vec<Field> {
    fields
        .iter()
        .filter_map(|(name, field)| {
            let field_path = format!("{path}.{name}");
            let id = ident(name, &field_path, sink);
            field_of(id, field, &field_path, sink)
        })
        .collect()
}

fn field_of(
    id: Option<Identifier>,
    field: &ast::Field,
    path: &str,
    sink: &mut Diagnostics,
) -> Option<Field> {
    let (ty, ann) = match field {
        ast::Field::Bare(expr) => (type_of(expr, path, sink)?, Annotations::default()),
        ast::Field::Full(body) => {
            let ty = type_of(&body.r#type, path, sink)?;
            let ann = annotations(&body.doc, &body.constraints, &body.metadata, path, sink);
            (ty, ann)
        }
    };
    Some(Field::new(id?, ty, ann))
}

fn type_of(expr: &TypeExpr, path: &str, sink: &mut Diagnostics) -> Option<DataType> {
    match expr {
        TypeExpr::Builtin(b) => Some(builtin_data_type(b)),
        TypeExpr::Record(name) => record_ref(name, path, sink),
        TypeExpr::List(t) => Some(DataType::List(Box::new(type_of(&t.list, path, sink)?))),
        TypeExpr::Option(t) => Some(DataType::Option(Box::new(type_of(&t.option, path, sink)?))),
    }
}

/// A bare name that is not a [`BuiltinType`], lowered as a record reference.
fn record_ref(name: &str, path: &str, sink: &mut Diagnostics) -> Option<DataType> {
    match Identifier::try_new(name) {
        Ok(id) => Some(DataType::Record(id)),
        Err(e) => {
            sink.error(path, format!("invalid type `{name}`: {e}"), None);
            None
        }
    }
}

/// The `DataType` a built-in name denotes.
fn builtin_data_type(builtin: &ast::BuiltinType) -> DataType {
    use ast::BuiltinType::*;
    match builtin {
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

/// Whether `name` is a [`BuiltinType`] spelling, decided by serde so the enum
/// stays the single source of truth for the reserved names.
fn is_builtin_type(name: &str) -> bool {
    use serde::de::IntoDeserializer;
    let de: serde::de::value::StrDeserializer<serde::de::value::Error> = name.into_deserializer();
    ast::BuiltinType::deserialize(de).is_ok()
}

fn annotations(
    doc: &Option<String>,
    constraints: &AnnotationMap,
    metadata: &AnnotationMap,
    path: &str,
    sink: &mut Diagnostics,
) -> Annotations {
    let mut builder = AnnotationsBuilder::new();
    if let Some(doc) = doc {
        builder.set_docs(doc.clone());
    }
    add_annotations(&mut builder, constraints, metadata, path, sink);
    builder.build()
}

fn add_annotations(
    builder: &mut AnnotationsBuilder,
    constraints: &AnnotationMap,
    metadata: &AnnotationMap,
    path: &str,
    sink: &mut Diagnostics,
) {
    for (name, value) in constraints {
        if name.is_empty() {
            sink.error(path, "constraint name must not be empty", None);
            continue;
        }
        if let Err(e) = builder.try_insert_constraint(name, value.clone()) {
            sink.error(path, e.to_string(), None);
        }
    }
    for (name, value) in metadata {
        if name.is_empty() {
            sink.error(path, "metadata name must not be empty", None);
            continue;
        }
        if let Err(e) = builder.try_insert_metadata(name, value.clone()) {
            sink.error(path, e.to_string(), None);
        }
    }
}

/// Validate a record/entity name, or `None` (after reporting) if it is invalid
/// or a reserved type name.
///
/// Records and entities are referenced by a bare name, so their names may not
/// shadow a built-in type ([`builtin_type`]). Field and event names are never
/// in type position and use [`ident`] directly.
fn type_decl_ident(name: &str, path: &str, sink: &mut Diagnostics) -> Option<Identifier> {
    if is_builtin_type(name) {
        sink.error(
            path,
            format!("`{name}` is a reserved type name"),
            Some("pick a different name".to_string()),
        );
        return None;
    }
    ident(name, path, sink)
}

fn ident(name: &str, path: &str, sink: &mut Diagnostics) -> Option<Identifier> {
    match Identifier::try_new(name) {
        Ok(id) => Some(id),
        Err(e) => {
            sink.error(path, format!("invalid name `{name}`: {e}"), None);
            None
        }
    }
}
