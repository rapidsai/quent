// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Log tests: a `log:` block declares a log-sink entity with one repeatable
//! event per ordered level.

use quent_instrumentation_build::{Options, generate_str};
use quent_log::{LogDefinition, SourceFields};
use quent_schema::test_utils::{ident, path};
use quent_schema::{Cardinality, DataType, Schema};
use quent_yaml::{Error, parse_from_str};

fn schema_of(src: &str) -> Schema {
    parse_from_str(src, None).expect("parses").schema
}

fn errors_of(src: &str) -> String {
    match parse_from_str(src, None) {
        Err(Error::Invalid(diagnostics)) => diagnostics.to_string(),
        other => panic!("expected diagnostics, got {other:?}"),
    }
}

const MINIMAL: &str = "\
quent: alpha
model: Application
entities:
  AppLog:
    log:
      levels:
        - name: trace
        - name: debug
        - name: info
        - name: warning
        - name: error
";

#[test]
fn minimal_log_generates_ranked_repeatable_message_events() {
    let schema = schema_of(MINIMAL);
    let entity = schema.entity(&path("AppLog")).unwrap();
    let definition = LogDefinition::from_entity(entity).unwrap().unwrap();
    assert_eq!(
        definition
            .levels()
            .map(|level| (level.name().to_string(), level.rank()))
            .collect::<Vec<_>>(),
        [
            ("trace".into(), 0),
            ("debug".into(), 1),
            ("info".into(), 2),
            ("warning".into(), 3),
            ("error".into(), 4),
        ]
    );
    for event in entity.events() {
        assert_eq!(event.cardinality(), Cardinality::Multi);
        assert_eq!(
            event.field(&ident("message")).unwrap().ty(),
            &DataType::String
        );
        assert_eq!(event.fields().count(), 1);
        assert!(event.field(&ident("log_level")).is_none());
        assert!(event.field(&ident("scope")).is_none());
    }
}

#[test]
fn complete_log_preserves_annotations_and_attribute_scopes() {
    let schema = schema_of(
        "\
quent: alpha
model: Application
entities:
  AppLog:
    doc: Application logging sink.
    constraints:
      application.constraint.v0.1.0:
    metadata:
      owner: platform
    log:
      target: true
      source:
        file: true
        line: true
        module: true
      attributes:
        thread_name: { option: string }
      levels:
        - name: trace
        - name: debug
        - name: info
          doc: Informational messages.
        - name: warning
          attributes:
            category: { option: string }
        - name: error
          attributes:
            error_code: { option: u32 }
",
    );
    let entity = schema.entity(&path("AppLog")).unwrap();
    assert_eq!(
        entity.annotations().docs(),
        Some("Application logging sink.")
    );
    assert!(
        entity
            .annotations()
            .has_constraint("application.constraint.v0.1.0")
    );
    assert_eq!(
        entity.annotations().metadata("owner").unwrap().data(),
        Some("platform")
    );

    let definition = LogDefinition::from_entity(entity).unwrap().unwrap();
    assert!(definition.target_enabled());
    assert_eq!(definition.source(), SourceFields::all());
    for event in entity.events() {
        for name in ["message", "target", "file", "line", "module", "thread_name"] {
            assert!(
                event.field(&ident(name)).is_some(),
                "{name} on {}",
                event.name()
            );
        }
    }
    let info = entity.event(&ident("info")).unwrap();
    assert_eq!(info.annotations().docs(), Some("Informational messages."));
    assert!(
        entity
            .event(&ident("warning"))
            .unwrap()
            .field(&ident("category"))
            .is_some()
    );
    assert!(
        entity
            .event(&ident("error"))
            .unwrap()
            .field(&ident("error_code"))
            .is_some()
    );
    assert!(info.field(&ident("category")).is_none());
    assert!(info.field(&ident("error_code")).is_none());
}

#[test]
fn source_true_enables_all_source_fields() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  Log:
    log:
      source: true
      levels: [{ name: info }]
",
    );
    let entity = schema.entity(&path("Log")).unwrap();
    let event = entity.event(&ident("info")).unwrap();
    assert_eq!(
        event.field(&ident("file")).unwrap().ty(),
        &DataType::Option(Box::new(DataType::String))
    );
    assert_eq!(
        event.field(&ident("line")).unwrap().ty(),
        &DataType::Option(Box::new(DataType::U32))
    );
    assert_eq!(
        event.field(&ident("module")).unwrap().ty(),
        &DataType::Option(Box::new(DataType::String))
    );
}

#[test]
fn detailed_source_selects_fields_independently() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  Log:
    log:
      source: { file: true, module: true }
      levels: [{ name: info }]
",
    );
    let entity = schema.entity(&path("Log")).unwrap();
    let definition = LogDefinition::from_entity(entity).unwrap().unwrap();
    assert_eq!(definition.source(), SourceFields::new(true, false, true));
    let event = entity.event(&ident("info")).unwrap();
    assert!(event.field(&ident("file")).is_some());
    assert!(event.field(&ident("line")).is_none());
    assert!(event.field(&ident("module")).is_some());
}

#[test]
fn absent_and_false_source_generate_no_source_fields() {
    for source in ["", "      source: false\n"] {
        let schema = schema_of(&format!(
            "quent: alpha\nmodel: m\nentities:\n  Log:\n    log:\n{source}      levels: [{{ name: info }}]\n"
        ));
        let entity = schema.entity(&path("Log")).unwrap();
        let event = entity.event(&ident("info")).unwrap();
        for name in ["file", "line", "module"] {
            assert!(event.field(&ident(name)).is_none());
        }
    }
}

#[test]
fn levels_must_be_nonempty_unique_valid_and_at_most_256() {
    for (levels, expected) in [
        ("[]", "must not be empty"),
        ("[{ name: info }, { name: info }]", "more than once"),
        ("[{ name: not-valid }]", "invalid name"),
    ] {
        let errors = errors_of(&format!(
            "quent: alpha\nmodel: m\nentities:\n  Log:\n    log:\n      levels: {levels}\n"
        ));
        assert!(errors.contains(expected), "{errors}");
    }

    let levels = (0..257)
        .map(|rank| format!("        - name: level{rank}\n"))
        .collect::<String>();
    let errors = errors_of(&format!(
        "quent: alpha\nmodel: m\nentities:\n  Log:\n    log:\n      levels:\n{levels}"
    ));
    assert!(
        errors.contains("257 levels") && errors.contains("256"),
        "{errors}"
    );
}

#[test]
fn user_attributes_cannot_collide_with_generated_fields() {
    for body in [
        "attributes: { message: string }",
        "target: true\n      attributes: { target: string }",
        "source: { file: true }\n      attributes: { file: string }",
        "source: { line: true }\n      attributes: { line: u32 }",
        "source: { module: true }\n      attributes: { module: string }",
    ] {
        let errors = errors_of(&format!(
            "quent: alpha\nmodel: m\nentities:\n  Log:\n    log:\n      {body}\n      levels: [{{ name: info }}]\n"
        ));
        assert!(errors.contains("conflicts"), "{errors}");
    }
}

#[test]
fn level_attributes_cannot_repeat_common_attributes() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Log:
    log:
      attributes: { thread: string }
      levels:
        - name: info
          attributes: { thread: string }
",
    );
    assert!(errors.contains("`thread` conflicts"), "{errors}");
}

#[test]
fn events_and_log_are_mutually_exclusive_even_when_events_are_empty() {
    for events in ["{}", "{ emitted: { multi: true } }"] {
        let errors = errors_of(&format!(
            "quent: alpha\nmodel: m\nentities:\n  Log:\n    events: {events}\n    log:\n      levels: [{{ name: info }}]\n"
        ));
        assert!(errors.contains("mutually exclusive"), "{errors}");
    }
}

#[test]
fn log_and_source_must_use_supported_forms() {
    for body in ["log:", "log: { source: null, levels: [{ name: info }] }"] {
        let _ = errors_of(&format!(
            "quent: alpha\nmodel: m\nentities:\n  Log:\n    {body}\n"
        ));
    }
}

#[test]
fn hand_written_log_constraint_is_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Log:
    constraints:
      quent.log.v0.1.0: '{}'
    events:
      info: { multi: true, attributes: { message: string } }
",
    );
    assert!(errors.contains("`log:` block"), "{errors}");
}

#[test]
fn ordinary_entities_do_not_opt_in() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  Worker:
    events:
      worked: {}
",
    );
    let entity = schema.entity(&path("Worker")).unwrap();
    assert!(LogDefinition::from_entity(entity).is_none());
    assert_eq!(entity.events().count(), 1);
}

#[test]
fn instrumentation_generates_one_repeatable_method_per_level() {
    let source = generate_str(&schema_of(MINIMAL), &Options::default()).unwrap();
    assert!(source.contains("pub enum AppLogEvent"), "{source}");
    for level in ["trace", "debug", "info", "warning", "error"] {
        let start = source
            .find(&format!("pub fn {level}("))
            .unwrap_or_else(|| panic!("missing generated method for {level}:\n{source}"));
        let signature = &source[start..source[start..].find(") ->").unwrap() + start];
        assert!(signature.contains("&self"), "{signature}");
        assert!(signature.contains("message: String"), "{signature}");
    }
}
