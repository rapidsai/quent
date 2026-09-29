// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX tests: `nvtx: true` composes the canonical stream for one OS process.

use quent_ref_target::RefTarget;
use quent_schema::test_utils::{ident, path};
use quent_schema::{Cardinality, DataType, Schema};
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

const PROCESS: &str = "\
quent: alpha
model: m
entities:
  Process:
    events:
      started:
        attributes:
          process: { os: process }
";

#[test]
fn yaml_and_typed_composition_are_identical() {
    let base = schema_of(PROCESS);
    let expected = nvtx_schema::compose(&base, &path("Process")).unwrap();
    let actual = schema_of(
        "\
quent: alpha
model: m
entities:
  Process:
    nvtx: true
    events:
      started:
        attributes:
          process: { os: process }
",
    );

    assert_eq!(actual, expected);
}

#[test]
fn expansion_binds_the_stream_to_the_selected_process_type() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  Machine:
    events:
      discovered: {}
  OtherProcess:
    events:
      started:
        attributes:
          process: { os: process }
          machine: { scope-ref: Machine }
  CapturedProcess:
    nvtx: true
    events:
      started:
        attributes:
          process: { os: process }
          machine: { scope-ref: Machine }
",
    );

    let stream = schema.entity(&path("NvtxEvent")).unwrap();
    assert_eq!(stream.events().len(), 13);
    let initialized = stream.event(&ident("initialized")).unwrap();
    assert_eq!(initialized.cardinality(), Cardinality::Once);
    let DataType::EntityRef { annotations, .. } =
        initialized.field(&ident("process")).unwrap().ty()
    else {
        panic!("binding must be an entity reference");
    };
    assert_eq!(
        RefTarget::from_annotations(annotations).unwrap().as_ref(),
        &path("CapturedProcess")
    );
    assert_eq!(
        stream
            .events()
            .filter(|event| event.cardinality() == Cardinality::Multi)
            .count(),
        12
    );
}

#[test]
fn false_and_absent_leave_the_schema_unchanged() {
    let absent = schema_of(PROCESS);
    let explicitly_disabled = schema_of(
        "\
quent: alpha
model: m
entities:
  Process:
    nvtx: false
    events:
      started:
        attributes:
          process: { os: process }
",
    );

    assert_eq!(explicitly_disabled, absent);
    assert!(absent.entity(&path("NvtxEvent")).is_none());
}

#[test]
fn non_process_entity_cannot_enable_nvtx() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Application:
    nvtx: true
    events:
      started: {}
",
    );

    assert!(errors.contains("Application"), "{errors}");
    assert!(errors.contains("process"), "{errors}");
}

#[test]
fn two_selected_process_types_are_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  First:
    nvtx: true
    events:
      started:
        attributes:
          process: { os: process }
  Second:
    nvtx: true
    events:
      started:
        attributes:
          process: { os: process }
",
    );

    assert!(
        errors.contains("NVTX may be enabled on only one process entity type"),
        "{errors}"
    );
}

#[test]
fn combined_process_and_main_thread_may_enable_nvtx() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  Main:
    nvtx: true
    events:
      started:
        attributes:
          process: { os: process }
          thread: { os: thread }
",
    );

    assert!(schema.entity(&path("NvtxEvent")).is_some());
}

#[test]
fn authored_reserved_stream_name_is_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Process:
    nvtx: true
    events:
      started:
        attributes:
          process: { os: process }
  NvtxEvent:
    events:
      custom: {}
",
    );

    assert!(errors.contains("NvtxEvent"), "{errors}");
}

#[test]
fn nvtx_constraint_cannot_be_authored_directly() {
    let errors = errors_of(
        "\
quent: alpha
model: m
constraints:
  quent.nvtx.v0.1.0:
entities:
  Process:
    events:
      started:
        attributes:
          process: { os: process }
",
    );

    assert!(
        errors.contains("NVTX constraint is set from `nvtx: true`"),
        "{errors}"
    );
}
