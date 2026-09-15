// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_log::{
    LevelDecl, LogConstraint, LogDefinition, LogDefinitionError, LogEntityBuilder,
    LogEntityBuilderError, LogError, MAX_LEVELS, SourceFields,
};
use quent_schema::{
    Annotations, Cardinality, DataType, Entity, Event, Field, Schema,
    builder::{AnnotationsBuilder, EntityBuilder, EventBuilder, SchemaBuilder},
    test_utils::{field, ident, path},
};

fn annotations(data: Option<String>) -> Annotations {
    AnnotationsBuilder::new()
        .with_constraint(LogConstraint::NAME, data)
        .build()
        .unwrap()
}

fn raw_definition(levels: &[&str], target: bool, source: SourceFields) -> String {
    serde_json::json!({
        "levels": levels,
        "target": target,
        "source": {
            "file": source.file(),
            "line": source.line(),
            "module": source.module(),
        },
    })
    .to_string()
}

fn event(name: &str, cardinality: Cardinality, fields: Vec<Field>) -> Event {
    EventBuilder::new(ident(name), cardinality)
        .with_fields(fields)
        .build()
        .unwrap()
}

fn required_fields(target: bool, source: SourceFields) -> Vec<Field> {
    let optional = |ty| DataType::Option(Box::new(ty));
    let mut fields = vec![field("message", DataType::String)];
    if target {
        fields.push(field("target", optional(DataType::String)));
    }
    if source.file() {
        fields.push(field("file", optional(DataType::String)));
    }
    if source.line() {
        fields.push(field("line", optional(DataType::U32)));
    }
    if source.module() {
        fields.push(field("module", optional(DataType::String)));
    }
    fields
}

fn schema(events: Vec<Event>, data: Option<String>) -> Schema {
    let entity = EntityBuilder::new(path("AppLog"))
        .with_events(events)
        .with_annotations(annotations(data))
        .build()
        .unwrap();
    SchemaBuilder::new(ident("Application"))
        .with_entity(entity)
        .build()
        .unwrap()
}

fn validate(schema: &Schema) -> Vec<LogError> {
    match quent_constraints::validate::<(LogConstraint,)>(schema)
        .results
        .0
    {
        Ok(()) => Vec::new(),
        Err(LogError::Multiple(errors)) => errors,
        Err(error) => vec![error],
    }
}

#[test]
fn builder_creates_ranked_repeatable_events() {
    let entity = LogEntityBuilder::new(path("AppLog"))
        .with_target(true)
        .with_source(SourceFields::all())
        .with_attributes([field(
            "thread_name",
            DataType::Option(Box::new(DataType::String)),
        )])
        .with_levels(["trace", "debug", "info"].map(|name| LevelDecl {
            name: ident(name),
            annotations: Annotations::default(),
            attributes: Vec::new(),
        }))
        .build()
        .unwrap();

    let definition = LogDefinition::from_entity(&entity).unwrap().unwrap();
    let levels: Vec<_> = definition
        .levels()
        .map(|level| (level.name().to_string(), level.rank()))
        .collect();
    assert_eq!(
        levels,
        [("trace".into(), 0), ("debug".into(), 1), ("info".into(), 2)]
    );
    assert!(definition.target_enabled());
    assert_eq!(definition.source(), SourceFields::all());
    for event in entity.events() {
        assert_eq!(event.cardinality(), Cardinality::Multi);
        assert_eq!(
            event.field(&ident("message")).unwrap().ty(),
            &DataType::String
        );
        assert!(event.field(&ident("thread_name")).is_some());
    }
}

#[test]
fn one_level_and_additional_fields_are_valid() {
    let mut fields = required_fields(false, SourceFields::default());
    fields.push(field("request_id", DataType::Uuid));
    let schema = schema(
        vec![event("info", Cardinality::Multi, fields)],
        Some(raw_definition(&["info"], false, SourceFields::default())),
    );
    assert!(validate(&schema).is_empty());
}

#[test]
fn all_standard_fields_are_valid() {
    let source = SourceFields::all();
    let schema = schema(
        vec![event(
            "warning",
            Cardinality::Multi,
            required_fields(true, source),
        )],
        Some(raw_definition(&["warning"], true, source)),
    );
    assert!(validate(&schema).is_empty());
}

#[test]
fn malformed_level_definitions_are_rejected() {
    for (data, expected) in [
        (None, "missing"),
        (Some("not json".to_string()), "decode"),
        (
            Some(r#"{"levels":["info"],"unknown":true}"#.to_string()),
            "unknown field",
        ),
        (
            Some(raw_definition(&[], false, SourceFields::default())),
            "must not be empty",
        ),
        (
            Some(raw_definition(
                &["info", "info"],
                false,
                SourceFields::default(),
            )),
            "more than once",
        ),
        (
            Some(raw_definition(
                &["not-valid"],
                false,
                SourceFields::default(),
            )),
            "invalid name",
        ),
    ] {
        let schema = schema(
            vec![event(
                "info",
                Cardinality::Multi,
                required_fields(false, SourceFields::default()),
            )],
            data,
        );
        assert!(validate(&schema)[0].to_string().contains(expected));
    }
}

#[test]
fn level_limit_accepts_256_and_rejects_257() {
    let levels: Vec<_> = (0..MAX_LEVELS)
        .map(|rank| ident(&format!("level{rank}")))
        .collect();
    assert!(LogDefinition::new(levels.clone(), false, SourceFields::default()).is_ok());
    let mut too_many = levels;
    too_many.push(ident("overflow"));
    assert!(matches!(
        LogDefinition::new(too_many, false, SourceFields::default()),
        Err(LogDefinitionError::TooManyLevels { count: 257, .. })
    ));
}

#[test]
fn event_set_must_equal_level_set() {
    let schema = schema(
        vec![
            event(
                "info",
                Cardinality::Multi,
                required_fields(false, SourceFields::default()),
            ),
            event(
                "extra",
                Cardinality::Multi,
                required_fields(false, SourceFields::default()),
            ),
        ],
        Some(raw_definition(
            &["info", "error"],
            false,
            SourceFields::default(),
        )),
    );
    let errors = validate(&schema);
    assert!(errors.iter().any(
        |error| matches!(error, LogError::MissingLevelEvent { level, .. } if level == "error")
    ));
    assert!(
        errors.iter().any(
            |error| matches!(error, LogError::UnexpectedEvent { event, .. } if event == "extra")
        )
    );
}

#[test]
fn level_events_must_be_repeatable() {
    let schema = schema(
        vec![event(
            "info",
            Cardinality::Once,
            required_fields(false, SourceFields::default()),
        )],
        Some(raw_definition(&["info"], false, SourceFields::default())),
    );
    assert!(matches!(
        validate(&schema).as_slice(),
        [LogError::CardinalityMismatch { .. }]
    ));
}

#[test]
fn required_standard_fields_must_exist_with_exact_types() {
    let source = SourceFields::all();
    let schema = schema(
        vec![event(
            "info",
            Cardinality::Multi,
            vec![
                field("target", DataType::String),
                field("file", DataType::Option(Box::new(DataType::String))),
                field("line", DataType::Option(Box::new(DataType::U64))),
            ],
        )],
        Some(raw_definition(&["info"], true, source)),
    );
    let errors = validate(&schema);
    assert!(
        errors.iter().any(
            |error| matches!(error, LogError::MissingField { field, .. } if field == "message")
        )
    );
    assert!(errors.iter().any(
        |error| matches!(error, LogError::IncorrectFieldType { field, .. } if field == "target")
    ));
    assert!(errors.iter().any(
        |error| matches!(error, LogError::IncorrectFieldType { field, .. } if field == "line")
    ));
    assert!(
        errors.iter().any(
            |error| matches!(error, LogError::MissingField { field, .. } if field == "module")
        )
    );
}

#[test]
fn builder_rejects_implicit_and_common_attribute_collisions() {
    let level = || LevelDecl {
        name: ident("info"),
        annotations: Annotations::default(),
        attributes: vec![field("thread", DataType::String)],
    };
    let error = LogEntityBuilder::new(path("Log"))
        .with_attributes([field("message", DataType::String)])
        .with_level(level())
        .build()
        .unwrap_err();
    assert!(matches!(
        error,
        LogEntityBuilderError::CommonAttributeCollision { name } if name == "message"
    ));

    let error = LogEntityBuilder::new(path("Log"))
        .with_attributes([field("thread", DataType::String)])
        .with_level(level())
        .build()
        .unwrap_err();
    assert!(matches!(
        error,
        LogEntityBuilderError::LevelAttributeCollision { name, .. } if name == "thread"
    ));
}

#[test]
fn misplaced_constraint_is_rejected() {
    let event = EventBuilder::new(ident("info"), Cardinality::Multi)
        .with_field(field("message", DataType::String))
        .with_annotations(annotations(Some(raw_definition(
            &["info"],
            false,
            SourceFields::default(),
        ))))
        .build()
        .unwrap();
    let entity = EntityBuilder::new(path("AppLog"))
        .with_event(event)
        .build()
        .unwrap();
    let schema = SchemaBuilder::new(ident("Application"))
        .with_entity(entity)
        .build()
        .unwrap();
    assert!(matches!(
        validate(&schema).as_slice(),
        [LogError::Misplaced { .. }]
    ));
}

#[test]
fn unannotated_entities_are_ignored() {
    let entity: Entity = EntityBuilder::new(path("Worker"))
        .with_event(event("worked", Cardinality::Once, Vec::new()))
        .build()
        .unwrap();
    let schema = SchemaBuilder::new(ident("Application"))
        .with_entity(entity)
        .build()
        .unwrap();
    assert!(validate(&schema).is_empty());
}
