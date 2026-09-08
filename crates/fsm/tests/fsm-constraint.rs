// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
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

fn state(name: &str, to: &[&str], initial: bool) -> StateDecl {
    StateDecl {
        name: ident(name),
        attributes: vec![],
        to: to.iter().map(|s| ident(s)).collect(),
        initial,
    }
}

// Build the constraint's JSON directly so we can build it in an invalid way.
fn fsm(initial: &str, transitions: &[(&str, &str)]) -> String {
    let transitions: Vec<serde_json::Value> = transitions
        .iter()
        .map(|&(source, target)| serde_json::json!({ "source": source, "target": target }))
        .collect();
    serde_json::json!({
        "initial_state": initial,
        "transitions": transitions,
    })
    .to_string()
}

/// Annotations carrying the FSM constraint with `data`.
fn fsm_annotations(data: Option<String>) -> Annotations {
    AnnotationsBuilder::new()
        .with_constraint(FsmConstraint::NAME, data)
        .build()
        .unwrap()
}

fn entity_with(name: &str, events: Vec<Event>, data: &str) -> Entity {
    EntityBuilder::new(name.parse::<quent_schema::Path>().unwrap())
        .with_events(events)
        .with_annotations(fsm_annotations(Some(data.to_string())))
        .build()
        .unwrap()
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
    let fsm = fsm("a", &[("a", "b")]);
    let entity = entity_with(
        "E",
        vec![event("a", Cardinality::Once), event("b", Cardinality::Once)],
        &fsm,
    );
    assert!(validate(&schema_with(entity)).is_empty());
}

#[test]
fn diagnostics_include_the_qualified_entity_path() {
    let fsm = fsm("a", &[("a", "missing")]);
    let entity = entity_with("Foo::E", vec![event("a", Cardinality::Once)], &fsm);

    assert!(
        validate(&schema_with(entity)).iter().any(
            |error| matches!(error, FsmError::UnknownState { entity, .. } if entity == "Foo::E")
        )
    );
}

#[test]
fn well_formed_self_loop_fsm_passes() {
    let fsm = fsm("a", &[("a", "a"), ("a", "b")]);
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Multi),
            event("b", Cardinality::Once),
        ],
        &fsm,
    );
    assert!(validate(&schema_with(entity)).is_empty());
}

#[test]
fn initial_state_cannot_also_be_final() {
    let fsm = fsm("a", &[]);
    let entity = entity_with("E", vec![event("a", Cardinality::Once)], &fsm);
    let errors = validate(&schema_with(entity));
    assert!(
        errors.iter().any(
            |error| matches!(error, FsmError::InitialStateIsFinal { state, .. } if state == "a")
        )
    );
}

#[test]
fn missing_data_is_rejected() {
    let entity = EntityBuilder::new(ident("E"))
        .with_event(event("a", Cardinality::Once))
        .with_annotations(fsm_annotations(None))
        .build()
        .unwrap();
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
        .with_event(event("a", Cardinality::Once))
        .with_annotations(fsm_annotations(Some("{ trash".to_string())))
        .build()
        .unwrap();
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::InvalidData { .. })),
    );
}

#[test]
fn exit_is_an_ordinary_state_name() {
    let fsm = fsm("a", &[("a", "EXIT")]);
    let entity = entity_with(
        "E",
        vec![
            event("EXIT", Cardinality::Once),
            event("a", Cardinality::Once),
        ],
        &fsm,
    );
    assert!(validate(&schema_with(entity)).is_empty());
}

#[test]
fn fsm_without_a_final_state_is_rejected() {
    let data = fsm("a", &[("a", "a")]);
    let entity = entity_with("E", vec![event("a", Cardinality::Multi)], &data);
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::CannotReachFinalState { state, .. } if state == "a"))
    );
}

#[test]
fn state_unreachable_from_initial_is_rejected() {
    // b and c form a disconnected component.
    let fsm = fsm("a", &[("a", "done"), ("b", "c")]);
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Once),
            event("done", Cardinality::Once),
            event("b", Cardinality::Once),
            event("c", Cardinality::Once),
        ],
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
fn state_cannot_reach_a_final_state_is_rejected() {
    // c is final, but b is trapped in a self-loop.
    let fsm = fsm("a", &[("a", "b"), ("a", "c"), ("b", "b")]);
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Once),
            event("b", Cardinality::Multi),
            event("c", Cardinality::Once),
        ],
        &fsm,
    );
    let errors = validate(&schema_with(entity));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, FsmError::CannotReachFinalState { state, .. } if state == "b"))
    );
}

// TODO(johanpel): consider allowing FSMs to have freestanding events
#[test]
fn event_not_covered_by_fsm_is_rejected() {
    // dead is declared but never appears as a state in the FSM.
    let fsm = fsm("a", &[("a", "done")]);
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Once),
            event("done", Cardinality::Once),
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
    let fsm = fsm("a", &[("a", "a"), ("a", "b")]);
    let entity = entity_with(
        "E",
        vec![event("a", Cardinality::Once), event("b", Cardinality::Once)],
        &fsm,
    );
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
    let fsm = fsm("a", &[("a", "done")]);
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Multi),
            event("done", Cardinality::Once),
        ],
        &fsm,
    );
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
    let fsm = fsm("a", &[("a", "b"), ("b", "a"), ("b", "final_state")]);

    // a and b should actually be multi, so this should not validate
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Once),
            event("b", Cardinality::Once),
            event("final_state", Cardinality::Once),
        ],
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
            event("final_state", Cardinality::Once),
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
        .with_states([state("a", &["b"], true), state("b", &[], false)])
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
            state("a", &["b"], true),
            state("b", &["b", "done"], false),
            state("done", &[], false),
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
        .with_states([state("a", &[], false)])
        .build()
        .unwrap_err();
    assert!(matches!(no_initial, FsmEntityBuilderError::NoInitialState));

    let many_initial = FsmEntityBuilder::new(ident("E"))
        .with_states([state("a", &[], true), state("b", &[], true)])
        .build()
        .unwrap_err();
    assert!(matches!(
        many_initial,
        FsmEntityBuilderError::MultipleInitialStates(_)
    ));

    let no_final = FsmEntityBuilder::new(ident("E"))
        .with_states([state("a", &["a"], true)])
        .build()
        .unwrap_err();
    assert!(matches!(
        no_final,
        FsmEntityBuilderError::Invalid(FsmError::CannotReachFinalState { .. })
    ));

    // A duplicate that is also marked initial must report the duplicate, not
    // `MultipleInitialStates`.
    let duplicate = FsmEntityBuilder::new(ident("E"))
        .with_state(state("a", &[], true))
        .with_state(state("a", &[], true))
        .build()
        .unwrap_err();
    assert!(matches!(
        duplicate,
        FsmEntityBuilderError::DuplicateState(_)
    ));
}

#[test]
fn multiple_final_states_pass() {
    // Both b and c have no outgoing transitions.
    let fsm = fsm("a", &[("a", "b"), ("a", "c")]);
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
fn multiple_violations_are_aggregated() {
    // b and c are unreachable from the initial state.
    // b is declared Multi though it is acyclic.
    let fsm = fsm("a", &[("a", "done"), ("b", "c")]);
    let entity = entity_with(
        "E",
        vec![
            event("a", Cardinality::Once),
            event("done", Cardinality::Once),
            event("b", Cardinality::Multi),
            event("c", Cardinality::Once),
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
