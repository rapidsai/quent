// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! OS tests: OS field types add and use canonical process and thread records.

use quent_constraints::Constraint as _;
use quent_os::{OsConstraint, process_path, thread_path};
use quent_schema::test_utils::{ident, path};
use quent_schema::{DataType, Schema};
use quent_yaml::parse_from_str;

fn schema_of(src: &str) -> Schema {
    parse_from_str(src, None).expect("parses").schema
}

fn errors_of(src: &str) -> String {
    match parse_from_str(src, None) {
        Ok(_) => panic!("expected errors, but parsing succeeded"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn process_type_adds_canonical_record() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  MyProcess:
    events:
      init:
        attributes:
          process: { os: process }
",
    );

    let field = schema
        .entity(&path("MyProcess"))
        .unwrap()
        .event(&ident("init"))
        .unwrap()
        .field(&ident("process"))
        .unwrap();
    assert_eq!(field.ty(), &DataType::Record(process_path()));
    assert!(
        schema
            .record(&process_path())
            .unwrap()
            .annotations()
            .has_constraint(OsConstraint::NAME)
    );
    assert!(schema.record(&thread_path()).is_none());
}

#[test]
fn os_type_may_be_used_in_an_annotated_field() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  MyProcess:
    events:
      init:
        attributes:
          process:
            type: { os: process }
            doc: Platform process identity.
",
    );

    let field = schema
        .entity(&path("MyProcess"))
        .unwrap()
        .event(&ident("init"))
        .unwrap()
        .field(&ident("process"))
        .unwrap();
    assert_eq!(field.ty(), &DataType::Record(process_path()));
    assert_eq!(
        field.annotations().docs(),
        Some("Platform process identity.")
    );
}

#[test]
fn thread_type_adds_canonical_record_when_scoped_under_process() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  MyProcess:
    events:
      init:
        attributes:
          process: { os: process }
  MyThread:
    events:
      init:
        attributes:
          thread: { os: thread }
          process: { scope-ref: MyProcess }
",
    );

    assert!(schema.record(&thread_path()).is_some());
    assert!(schema.record(&process_path()).is_some());
}

#[test]
fn process_and_main_thread_may_share_an_entity_without_a_scope_reference() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  Main:
    events:
      started:
        attributes:
          process: { os: process }
          thread: { os: thread }
",
    );

    let event = schema
        .entity(&path("Main"))
        .unwrap()
        .event(&ident("started"))
        .unwrap();
    assert_eq!(
        event.field(&ident("process")).unwrap().ty(),
        &DataType::Record(process_path())
    );
    assert_eq!(
        event.field(&ident("thread")).unwrap().ty(),
        &DataType::Record(thread_path())
    );
}

#[test]
fn process_record_on_multi_event_is_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  MyProcess:
    events:
      observed:
        multi: true
        attributes:
          process: { os: process }
",
    );

    assert!(errors.contains("must carry OS record `quent::os::Process` in a `Once` event"));
}

#[test]
fn thread_without_process_ancestor_is_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  MyThread:
    events:
      started:
        attributes:
          thread: { os: thread }
",
    );

    assert!(errors.contains("OS thread entity must be scoped under an OS process entity"));
}

#[test]
fn nested_os_records_are_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Target:
    events:
      init: {}
  Candidate:
    events:
      init:
        attributes:
          optional: { option: { os: process } }
          processes: { list: { os: process } }
          target: { ref: Target, data: { os: process } }
",
    );

    assert_eq!(
        errors
            .matches("must be used directly as an entity event field")
            .count(),
        3,
        "{errors}"
    );
}

#[test]
fn duplicate_os_records_in_one_event_are_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  MyProcess:
    events:
      init:
        attributes:
          first: { os: process }
          second: { os: process }
",
    );

    assert!(
        errors.contains("OS record `quent::os::Process` may be carried by only one event field")
    );
}
