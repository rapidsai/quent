// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_fsm::{FsmConstraint, FsmEntityBuilder, FsmEntityBuilderError, FsmError, StateDecl};
use quent_schema::{
    Annotations, Cardinality, Entity, Event, Schema,
    builder::{AnnotationsBuilder, EntityBuilder},
    test_utils::{entity as bare_entity, event_with, ident, schema},
};

fn event(name: &str, cardinality: Cardinality) -> Event {
    event_with(name, cardinality, vec![])
}

fn state(name: &str, to: &[&str], initial: bool, exit: bool) -> StateDecl {
    StateDecl {
        name: ident(name),
        attributes: vec![],
        to: to.iter().map(|s| ident(s)).collect(),
        initial,
        exit,
    }
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

/// Annotations carrying the FSM constraint with `data`.
fn fsm_annotations(data: Option<String>) -> Annotations {
    AnnotationsBuilder::new()
        .try_with_constraint(FsmConstraint::NAME, data)
        .unwrap()
        .build()
}

fn entity_with(name: &str, events: Vec<Event>, data: &str) -> Entity {
    EntityBuilder::new(ident(name))
        .try_with_events(events)
        .unwrap()
        .with_annotations(fsm_annotations(Some(data.to_string())))
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
        .try_with_event(event("a", Cardinality::Once))
        .unwrap()
        .with_annotations(fsm_annotations(None))
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
        .try_with_event(event("a", Cardinality::Once))
        .unwrap()
        .with_annotations(fsm_annotations(Some("{ trash".to_string())))
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
fn builder_produces_entity_with_state_events() {
    let entity = FsmEntityBuilder::new(ident("E"))
        .with_states([
            state("a", &["b"], true, false),
            state("b", &[], false, true),
        ])
        .build()
        .unwrap();

    let names: Vec<_> = entity.events().map(|e| e.name().to_string()).collect();
    assert_eq!(names, vec!["a", "b"]);
    assert!(entity.annotations().has_constraint(FsmConstraint::NAME));
}

#[test]
fn cardinality_is_derived_from_cycles() {
    // a -> b, b -> b: a sits off any cycle (Once), b self-loops (Multi).
    let entity = FsmEntityBuilder::new(ident("E"))
        .with_states([
            state("a", &["b"], true, false),
            state("b", &["b"], false, true),
        ])
        .build()
        .unwrap();

    let cardinality = |name: &str| entity.event(&ident(name)).unwrap().cardinality();
    assert_eq!(cardinality("a"), Cardinality::Once);
    assert_eq!(cardinality("b"), Cardinality::Multi);
}

#[test]
fn builder_rejects_malformed_states() {
    let no_initial = FsmEntityBuilder::new(ident("E"))
        .with_states([state("a", &[], false, true)])
        .build()
        .unwrap_err();
    assert!(matches!(no_initial, FsmEntityBuilderError::NoInitialState));

    let many_initial = FsmEntityBuilder::new(ident("E"))
        .with_states([state("a", &[], true, false), state("b", &[], true, true)])
        .build()
        .unwrap_err();
    assert!(matches!(
        many_initial,
        FsmEntityBuilderError::MultipleInitialStates(_)
    ));

    let no_exit = FsmEntityBuilder::new(ident("E"))
        .with_states([state("a", &["a"], true, false)])
        .build()
        .unwrap_err();
    assert!(matches!(no_exit, FsmEntityBuilderError::NoExitState));

    // A duplicate that is also marked initial must report the duplicate, not
    // `MultipleInitialStates`.
    let duplicate = FsmEntityBuilder::new(ident("E"))
        .with_state(state("a", &[], true, true))
        .with_state(state("a", &[], true, false))
        .build()
        .unwrap_err();
    assert!(matches!(
        duplicate,
        FsmEntityBuilderError::DuplicateState(_)
    ));
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
