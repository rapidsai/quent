// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_fsm::{Fsm, FsmConstraint, FsmError};
use quent_schema::{
    Cardinality, Entity, Event, Schema,
    builder::{AnnotationsBuilder, EntityBuilder},
    test_utils::{entity as bare_entity, event_with, ident, schema},
};

fn event(name: &str, cardinality: Cardinality) -> Event {
    event_with(name, cardinality, vec![])
}

// Build the constraint's JSON directly so we can build it in an invalid way.
fn fsm(initial: &str, transitions: &[(&str, &str)], exit: &[&str]) -> String {
    let transitions: Vec<serde_json::Value> = transitions
        .iter()
        .map(|&(source, target)| serde_json::json!({ "source": source, "target": target }))
        .collect();
    let (first_exit, other_exit) = exit.split_first().unwrap();
    serde_json::json!({
        "initial_state": initial,
        "transitions": transitions,
        "exit_from_states": { "state": first_exit, "others": other_exit },
    })
    .to_string()
}

fn entity_with(name: &str, events: Vec<Event>, data: &str) -> Entity {
    EntityBuilder::new(ident(name))
        .events(events)
        .unwrap()
        .annotations(
            AnnotationsBuilder::new()
                .constraint(FsmConstraint::NAME, Some(data.to_string()))
                .unwrap()
                .build(),
        )
        .build()
}

fn schema_with(entity: Entity) -> Schema {
    schema("S", vec![entity], vec![])
}

fn validate(schema: &Schema) -> Vec<FsmError> {
    let report = quent_constraints::validate::<(FsmConstraint,)>(schema);
    match report.results.0 {
        Ok(()) => Vec::new(),
        Err(FsmError::Multiple(errors)) => errors,
        Err(single) => vec![single],
    }
}

#[test]
fn well_formed_linear_fsm_passes() {
    let fsm = fsm("a", &[("a", "b")], &["b"]);
    let entity = entity_with(
        "E",
        vec![event("a", Cardinality::Once), event("b", Cardinality::Once)],
        &fsm,
    );
    assert!(validate(&schema_with(entity)).is_empty());
}

#[test]
fn well_formed_self_loop_fsm_passes() {
    let fsm = fsm("a", &[("a", "a")], &["a"]);
    let entity = entity_with("E", vec![event("a", Cardinality::Multi)], &fsm);
    assert!(validate(&schema_with(entity)).is_empty());
}

#[test]
fn single_state_fsm_passes() {
    let fsm = fsm("a", &[], &["a"]);
    let entity = entity_with("E", vec![event("a", Cardinality::Once)], &fsm);
    assert!(validate(&schema_with(entity)).is_empty());
}

#[test]
fn missing_data_is_rejected() {
    let entity = EntityBuilder::new(ident("E"))
        .event(event("a", Cardinality::Once))
        .unwrap()
        .annotations(
            AnnotationsBuilder::new()
                .constraint(FsmConstraint::NAME, None)
                .unwrap()
                .build(),
        )
        .build();
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::InvalidData { .. }))
    );
}

#[test]
fn invalid_json_is_rejected() {
    let entity = EntityBuilder::new(ident("E"))
        .event(event("a", Cardinality::Once))
        .unwrap()
        .annotations(
            AnnotationsBuilder::new()
                .constraint(FsmConstraint::NAME, Some("{ trash".to_string()))
                .unwrap()
                .build(),
        )
        .build();
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::InvalidData { .. })),
    );
}

#[test]
fn reserved_name_exit_is_rejected() {
    let fsm = fsm("a", &[], &["a"]);
    let entity = entity_with(
        "E",
        vec![
            event("EXIT", Cardinality::Once),
            event("a", Cardinality::Once),
        ],
        &fsm,
    );
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::ReservedStateName { name: "exit", .. })),
    );
}

#[test]
fn empty_exit_is_rejected() {
    let data = serde_json::json!({
        "initial_state": "a",
        "transitions": [],
        "exit_from_states": [],
    })
    .to_string();
    let entity = entity_with("E", vec![event("a", Cardinality::Once)], &data);
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::InvalidData { .. }))
    );
}

#[test]
fn state_unreachable_from_initial_is_rejected() {
    // b is listed as an exit state but nothing transitions into it
    let fsm = fsm("a", &[], &["a", "b"]);
    let entity = entity_with(
        "E",
        vec![event("a", Cardinality::Once), event("b", Cardinality::Once)],
        &fsm,
    );
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::UnreachableFromInit { state, .. } if state == "b")),
    );
}

#[test]
fn state_cannot_reach_exit_is_rejected() {
    // a may exit, but b has no path to an exit state
    let fsm = fsm("a", &[("a", "b")], &["a"]);
    let entity = entity_with(
        "E",
        vec![event("a", Cardinality::Once), event("b", Cardinality::Once)],
        &fsm,
    );
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::CannotReachExit { state, .. } if state == "b")),
    );
}

#[test]
fn fsm_state_not_in_events_is_rejected() {
    let fsm = fsm("phantom", &[], &["phantom"]);
    let entity = entity_with("E", vec![event("a", Cardinality::Once)], &fsm);
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::UnknownState { state, .. } if state == "phantom")),
    );
}

// TODO(johanpel): consider allowing FSMs to have freestanding events
#[test]
fn event_not_covered_by_fsm_is_rejected() {
    // dead is declared but never appears as a state in the FSM.
    let fsm = fsm("a", &[], &["a"]);
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Once),
            event("dead", Cardinality::Once),
        ],
        &fsm,
    );
    let errors = validate(&schema_with(entity));
    assert!(errors.iter().any(|e| matches!(e,
    FsmError::UncoveredEvent { event, .. } if event == "dead")),);
}

#[test]
fn cycle_requires_multi_cardinality() {
    let fsm = fsm("a", &[("a", "a")], &["a"]);
    let entity = entity_with("E", vec![event("a", Cardinality::Once)], &fsm);
    let errors = validate(&schema_with(entity));
    assert!(errors.iter().any(|e| matches!(
        e,
        FsmError::CardinalityMismatch {
            expected: Cardinality::Multi,
            found: Cardinality::Once,
            ..
        }
    )),);
}

#[test]
fn acyclic_requires_once_cardinality() {
    let fsm = fsm("a", &[], &["a"]);
    let entity = entity_with("E", vec![event("a", Cardinality::Multi)], &fsm);
    let errors = validate(&schema_with(entity));
    assert!(errors.iter().any(|e| matches!(
        e,
        FsmError::CardinalityMismatch {
            expected: Cardinality::Once,
            found: Cardinality::Multi,
            ..
        }
    )),);
}

#[test]
fn scc_of_size_two_requires_multi_for_both_states() {
    let fsm = fsm("a", &[("a", "b"), ("b", "a")], &["b"]);

    // a and b should actually be multi, so this should not validate
    let entity = entity_with(
        "E",
        vec![event("a", Cardinality::Once), event("b", Cardinality::Once)],
        &fsm,
    );
    let errors = validate(&schema_with(entity));
    assert!(errors.iter().any(|e| matches!(
        e,
        FsmError::CardinalityMismatch { state, expected: Cardinality::Multi, .. }
            if state == "a"
    )),);
    assert!(errors.iter().any(|e| matches!(
        e,
        FsmError::CardinalityMismatch { state, expected: Cardinality::Multi, .. }
            if state == "b"
    )),);

    // make them multi to make it pass
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Multi),
            event("b", Cardinality::Multi),
        ],
        &fsm,
    );
    assert!(validate(&schema_with(entity)).is_empty());
}

#[test]
fn entity_without_fsm_constraint_is_ignored() {
    let entity = bare_entity("E", vec![event("a", Cardinality::Once)]);
    assert!(validate(&schema_with(entity)).is_empty());
}

#[test]
fn builder_produces_valid_fsm_and_exposes_data() {
    let entity = bare_entity(
        "E",
        vec![event("a", Cardinality::Once), event("b", Cardinality::Once)],
    );
    let fsm = Fsm::builder(&entity, ident("a"), ident("b"))
        .transition(ident("a"), ident("b"))
        .build()
        .unwrap();

    assert_eq!(fsm.initial_state(), &ident("a"));
    assert_eq!(fsm.transitions().len(), 1);
    assert_eq!(fsm.transitions()[0].source(), &ident("a"));
    assert_eq!(fsm.transitions()[0].target(), &ident("b"));
    assert_eq!(
        fsm.exit_from_states().collect::<Vec<_>>(),
        vec![&ident("b")],
    );
}

#[test]
fn builder_rejects_unknown_event() {
    // b is not an event
    let entity = bare_entity("E", vec![event("a", Cardinality::Once)]);
    let err = Fsm::builder(&entity, ident("b"), ident("b"))
        .build()
        .err()
        .unwrap();
    let errors = match err {
        FsmError::Multiple(errors) => errors,
        single => vec![single],
    };
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::UnknownState { state, .. } if state == "b")),
    );
}

#[test]
fn multiple_exit_states_pass() {
    // Both b and c are valid exit states.
    let fsm = fsm("a", &[("a", "b"), ("a", "c")], &["b", "c"]);
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Once),
            event("b", Cardinality::Once),
            event("c", Cardinality::Once),
        ],
        &fsm,
    );
    assert!(validate(&schema_with(entity)).is_empty());
}

#[test]
fn exit_state_may_have_outgoing_transition() {
    // a is an exit state but may also continue to b.
    let fsm = fsm("a", &[("a", "b")], &["a", "b"]);
    let entity = entity_with(
        "E",
        vec![event("a", Cardinality::Once), event("b", Cardinality::Once)],
        &fsm,
    );
    assert!(validate(&schema_with(entity)).is_empty());
}

#[test]
fn multiple_violations_are_aggregated() {
    // b is unreachable from the initial state
    // b is declared Multi though it is acyclic
    let fsm = fsm("a", &[], &["a", "b"]);
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Once),
            event("b", Cardinality::Multi),
        ],
        &fsm,
    );
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::UnreachableFromInit { .. }))
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::CardinalityMismatch { .. }))
    );
}
