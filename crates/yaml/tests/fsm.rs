// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! FSM tests: an `fsms:` block declares an entity whose events are its states,
//! deriving their cardinality from the topology and validating it.

use quent_schema::test_utils::{ident, path};
use quent_schema::{Cardinality, Schema};
use quent_yaml::{Error, parse_from_str};

const FSM: &str = "quent.fsm.v0.1.0";

fn schema_of(src: &str) -> Schema {
    parse_from_str(src, None).expect("parses").schema
}

fn errors_of(src: &str) -> String {
    match parse_from_str(src, None) {
        Err(Error::Invalid(diagnostics)) => diagnostics.to_string(),
        other => panic!("expected diagnostics, got {other:?}"),
    }
}

const QUERY: &str = "\
quent: alpha
model: m
fsms:
  Query:
    doc: A query.
    states:
      submitted:
        initial: true
        attributes: { text: string }
        to: [progress]
      progress:
        attributes: { pct: u8 }
        to: [progress, finished]
      finished:
        attributes: { ok: bool }
";

#[test]
fn fsm_builds_events_and_derives_cardinality() {
    let schema = schema_of(QUERY);
    let query = schema.entity(&path("Query")).unwrap();
    assert!(query.annotations().has_constraint(FSM));
    assert_eq!(query.events().count(), 3);
    // `progress` self-loops, so it is Multi; the others are Once.
    let card = |e: &str| query.event(&ident(e)).unwrap().cardinality();
    assert!(matches!(card("progress"), Cardinality::Multi));
    assert!(matches!(card("submitted"), Cardinality::Once));
    assert!(matches!(card("finished"), Cardinality::Once));
}

#[test]
fn entity_declared_as_both_entity_and_fsm_is_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  E:
    events:
      a: {}
fsms:
  E:
    states:
      a: { initial: true }
",
    );
    assert!(
        errors.contains("both an entity and an FSM") && errors.contains("E"),
        "{errors}"
    );
}

#[test]
fn fsm_needs_one_initial_state() {
    let errors = errors_of(
        "\
quent: alpha
model: m
fsms:
  E:
    states:
      a: {}
      b: {}
",
    );
    assert!(
        errors.contains("no state marked `initial: true`"),
        "{errors}"
    );
}

#[test]
fn fsm_needs_a_final_state() {
    let errors = errors_of(
        "\
quent: alpha
model: m
fsms:
  E:
    states:
      a: { initial: true, to: [a] }
",
    );
    assert!(errors.contains("cannot reach any final state"), "{errors}");
}

#[test]
fn initial_state_cannot_also_be_final() {
    let errors = errors_of(
        "\
quent: alpha
model: m
fsms:
  E:
    states:
      only: { initial: true }
",
    );
    assert!(
        errors.contains("initial state \"only\" cannot also be a final state"),
        "{errors}"
    );
}

#[test]
fn unreachable_state_is_rejected() {
    // `b` is a state but nothing reaches it from the initial state.
    let errors = errors_of(
        "\
quent: alpha
model: m
fsms:
  E:
    states:
      a: { initial: true, to: [c] }
      b: { to: [a] }
      c: {}
",
    );
    assert!(errors.contains("unreachable"), "{errors}");
}

#[test]
fn exit_is_an_ordinary_state_with_attributes() {
    let schema = schema_of(
        "\
quent: alpha
model: m
fsms:
  E:
    states:
      running: { initial: true, to: [exit] }
      exit:
        attributes: { code: i32 }
",
    );
    let exit = schema
        .entity(&path("E"))
        .unwrap()
        .event(&ident("exit"))
        .unwrap();
    assert!(exit.field(&ident("code")).is_some());
}

#[test]
fn undeclared_exit_target_is_rejected() {
    let errors = errors_of(
        "\
quent: alpha
model: m
fsms:
  E:
    states:
      running: { initial: true, to: [exit] }
",
    );
    assert!(
        errors.contains("state \"exit\" does not match any event"),
        "{errors}"
    );
}
