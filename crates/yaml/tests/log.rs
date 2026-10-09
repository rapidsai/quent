// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Log tests: a `logs:` entry declares a log-sink entity with one repeatable
//! event per ordered level.

use quent_instrumentation_build::{Options, generate_str};
use quent_log::LogDefinition;
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
quent: '0.1.0'
model: Application
logs:
  AppLog:
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
quent: '0.1.0'
model: Application
logs:
  AppLog:
    doc: Application logging sink.
    constraints:
      application.constraint.v0.1.0:
    metadata:
      owner: platform
    attributes:
      target: string
      file: string
      line: u32
      module: string
      thread_name: string
    levels:
      - name: trace
      - name: debug
      - name: info
        doc: Informational messages.
      - name: warning
        attributes:
          category: string
      - name: error
        attributes:
          error_code: u32
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
fn logging_context_uses_ordinary_attributes() {
    let schema = schema_of(
        "\
quent: '0.1.0'
model: m
logs:
  Log:
    attributes:
      target: u64
      file: bool
      line: string
      module: dynamic
    levels: [{ name: info }]
",
    );
    let entity = schema.entity(&path("Log")).unwrap();
    let event = entity.event(&ident("info")).unwrap();
    for (name, ty) in [
        ("target", DataType::U64),
        ("file", DataType::Bool),
        ("line", DataType::String),
        ("module", DataType::DynamicRecord),
    ] {
        assert_eq!(event.field(&ident(name)).unwrap().ty(), &ty);
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
            "quent: '0.1.0'\nmodel: m\nlogs:\n  Log:\n    levels: {levels}\n"
        ));
        assert!(errors.contains(expected), "{errors}");
    }

    let levels = (0..257)
        .map(|rank| format!("        - name: level{rank}\n"))
        .collect::<String>();
    let errors = errors_of(&format!(
        "quent: '0.1.0'\nmodel: m\nlogs:\n  Log:\n    levels:\n{levels}"
    ));
    assert!(
        errors.contains("257 levels") && errors.contains("256"),
        "{errors}"
    );
}

#[test]
fn user_attributes_cannot_collide_with_generated_fields() {
    let errors = errors_of(
        "quent: '0.1.0'\nmodel: m\nlogs:\n  Log:\n    attributes: { message: string }\n    levels: [{ name: info }]\n",
    );
    assert!(errors.contains("conflicts"), "{errors}");
}

#[test]
fn level_attributes_cannot_repeat_common_attributes() {
    let errors = errors_of(
        "\
quent: '0.1.0'
model: m
logs:
  Log:
    attributes: { thread: string }
    levels:
      - name: info
        attributes: { thread: string }
",
    );
    assert!(errors.contains("`thread` conflicts"), "{errors}");
}

#[test]
fn initialization_event_can_attach_log_instance_to_os_process() {
    let schema = schema_of(
        "\
quent: '0.1.0'
model: m
entities:
  Process:
    events:
      started:
        attributes:
          process: { os: process }
logs:
  Log:
    events:
      initialized:
        attributes:
          process: { scope-ref: Process }
    levels: [{ name: info }]
",
    );
    let entity = schema.entity(&path("Log")).unwrap();
    let initialized = entity.event(&ident("initialized")).unwrap();
    assert_eq!(initialized.cardinality(), Cardinality::Once);
    assert!(initialized.field(&ident("message")).is_none());
    assert!(
        LogDefinition::from_entity(entity)
            .unwrap()
            .unwrap()
            .level(&ident("initialized"))
            .is_none()
    );
}

#[test]
fn non_level_event_cannot_reuse_a_level_name() {
    let errors = errors_of(
        "quent: '0.1.0'\nmodel: m\nlogs:\n  Log:\n    events:\n      info: {}\n    levels: [{ name: info }]\n",
    );
    assert!(errors.contains("duplicate name \"info\""), "{errors}");
}

#[test]
fn duplicate_entity_and_log_path_is_rejected() {
    let errors = errors_of(
        "quent: '0.1.0'\nmodel: m\nentities:\n  Log:\n    events:\n      emitted: {}\nlogs:\n  Log:\n    levels: [{ name: info }]\n",
    );
    assert!(errors.contains("duplicate type path `Log`"), "{errors}");
}

#[test]
fn logs_must_use_supported_form() {
    let _ = errors_of("quent: '0.1.0'\nmodel: m\nlogs:\n  Log:\n");
}

#[test]
fn removed_context_shortcuts_are_rejected() {
    for field in ["target: true", "source: true"] {
        let errors = errors_of(&format!(
            "quent: '0.1.0'\nmodel: m\nlogs:\n  Log:\n    {field}\n    levels: [{{ name: info }}]\n"
        ));
        assert!(errors.contains("unknown field"), "{errors}");
    }
}

#[test]
fn hand_written_log_constraint_is_rejected() {
    let errors = errors_of(
        "\
quent: '0.1.0'
model: m
entities:
  Log:
    constraints:
      quent.log.v0.1.0: '{}'
    events:
      info: { multi: true, attributes: { message: string } }
",
    );
    assert!(errors.contains("`logs:` declaration"), "{errors}");
}

#[test]
fn ordinary_entities_do_not_opt_in() {
    let schema = schema_of(
        "\
quent: '0.1.0'
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
