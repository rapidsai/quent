// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_dag::DagRole;
use quent_ref_target::RefTarget;
use quent_schema::{DataType, Path, test_utils::path};
use quent_yaml::{Error, parse_from_str};

const MODEL: &str = "\
quent: alpha
model: SimplePlan
entities:
  Plan:
    events:
      created: {}
  Operator:
    dag:
      vertex: Plan
  PlanEdge:
    dag:
      edge: Plan
    events:
      connected:
        attributes:
          input:
            dag:
              source: Operator
          output:
            dag:
              target: Operator
";

#[test]
fn dag_sugar_builds_and_validates_schema() {
    let parsed = parse_from_str(MODEL, None).expect("valid DAG model");
    assert!(parsed.warnings.is_empty());

    let plan = parsed.schema.entity(&path("Plan")).unwrap();
    assert_eq!(
        DagRole::from_annotations(plan.annotations()),
        Ok(Some(DagRole::Dag))
    );
    assert!(plan.event(&"created".try_into().unwrap()).is_some());

    let operator = parsed.schema.entity(&path("Operator")).unwrap();
    let membership = operator
        .event(&"declared".try_into().unwrap())
        .unwrap()
        .field(&"dag".try_into().unwrap())
        .unwrap();
    assert_eq!(
        DagRole::from_annotations(membership.annotations()),
        Ok(Some(DagRole::Membership))
    );
    assert_eq!(reference_target(membership.ty()), Some(path("Plan")));

    let edge = parsed.schema.entity(&path("PlanEdge")).unwrap();
    let connected = edge.event(&"connected".try_into().unwrap()).unwrap();
    assert_eq!(
        DagRole::from_annotations(
            connected
                .field(&"dag".try_into().unwrap())
                .unwrap()
                .annotations()
        ),
        Ok(Some(DagRole::Membership))
    );
    assert_eq!(
        DagRole::from_annotations(
            connected
                .field(&"input".try_into().unwrap())
                .unwrap()
                .annotations()
        ),
        Ok(Some(DagRole::Source))
    );
    assert_eq!(
        DagRole::from_annotations(
            connected
                .field(&"output".try_into().unwrap())
                .unwrap()
                .annotations()
        ),
        Ok(Some(DagRole::Target))
    );
}

#[test]
fn endpoint_target_must_be_a_vertex() {
    let invalid = MODEL.replace("source: Operator", "source: Plan");
    let Err(Error::Invalid(diagnostics)) = parse_from_str(invalid, None) else {
        panic!("expected invalid DAG model");
    };
    assert!(
        diagnostics
            .to_string()
            .contains("`source` targets \"Plan\", which is not a vertex entity")
    );
}

#[test]
fn dag_marker_must_be_true() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Plan:
    dag: false
    events:
      created: {}
",
    );
    assert!(errors.contains("`dag` must be `true`"));
}

#[test]
fn inferred_dag_must_declare_an_event() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Plan: {}
  Operator:
    dag:
      vertex: Plan
",
    );
    assert!(
        errors.contains("entity `Plan` declares no events"),
        "{errors}"
    );
}

#[test]
fn hand_written_dag_constraint_is_rejected_on_an_entity() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Plan:
    constraints:
      quent.dag.v0.1.0: dag
    events:
      declared: {}
",
    );
    assert!(errors.contains("DAG constraint is set from a `dag:` declaration"));
}

#[test]
fn hand_written_dag_constraint_is_rejected_on_a_field() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Plan:
    events:
      declared:
        attributes:
          endpoint:
            type:
              ref: Plan
            constraints:
              quent.dag.v0.1.0: source
",
    );
    assert!(errors.contains("DAG constraint is set from a `dag:` declaration"));
}

#[test]
fn generated_membership_field_name_is_reserved() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Plan:
    events:
      created: {}
  Operator:
    dag:
      vertex: Plan
    events:
      declared:
        attributes:
          dag: string
",
    );
    assert!(
        errors.contains("`dag` is reserved for generated DAG membership"),
        "{errors}"
    );
    assert!(!errors.contains("duplicate name"), "{errors}");
    assert!(!errors.contains("declares no events"), "{errors}");
}

#[test]
fn dag_membership_event_must_not_be_multi() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Plan:
    events:
      created: {}
  Operator:
    dag:
      vertex: Plan
    events:
      declared:
        multi: true
",
    );
    assert!(
        errors.contains("`multi` must be `false` for a DAG membership event"),
        "{errors}"
    );
}

#[test]
fn fsm_vertex_belongs_to_a_dag() {
    let parsed = parse_from_str(
        "\
quent: alpha
model: m
entities:
  Plan:
    events:
      created: {}
fsms:
  Operator:
    dag:
      vertex: Plan
    states:
      created:
        initial: true
        to: [done]
      done: {}
",
        None,
    )
    .expect("valid FSM vertex");

    let operator = parsed.schema.entity(&path("Operator")).unwrap();
    let membership = operator
        .event(&"created".try_into().unwrap())
        .unwrap()
        .field(&"dag".try_into().unwrap())
        .unwrap();
    assert_eq!(
        DagRole::from_annotations(membership.annotations()),
        Ok(Some(DagRole::Membership))
    );
    assert_eq!(reference_target(membership.ty()), Some(path("Plan")));
}

#[test]
fn fsm_edge_belongs_to_a_dag() {
    let parsed = parse_from_str(
        "\
quent: alpha
model: m
entities:
  Plan:
    events:
      created: {}
  Operator:
    dag:
      vertex: Plan
fsms:
  PlanEdge:
    dag:
      edge: Plan
    states:
      connected:
        initial: true
        attributes:
          input:
            dag:
              source: Operator
          output:
            dag:
              target: Operator
        to: [done]
      done: {}
",
        None,
    )
    .expect("valid FSM edge");

    let edge = parsed.schema.entity(&path("PlanEdge")).unwrap();
    let connected = edge.event(&"connected".try_into().unwrap()).unwrap();
    assert_eq!(
        DagRole::from_annotations(
            connected
                .field(&"dag".try_into().unwrap())
                .unwrap()
                .annotations()
        ),
        Ok(Some(DagRole::Membership))
    );
}

#[test]
fn fsm_edge_endpoints_must_share_a_state() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Plan:
    events:
      created: {}
  Operator:
    dag:
      vertex: Plan
fsms:
  PlanEdge:
    dag:
      edge: Plan
    states:
      source_state:
        initial: true
        attributes:
          input:
            dag:
              source: Operator
        to: [target_state]
      target_state:
        attributes:
          output:
            dag:
              target: Operator
        to: [done]
      done: {}
",
    );
    assert!(
        errors.contains("DAG edge endpoints must be declared in one state"),
        "{errors}"
    );
}

#[test]
fn fsm_dag_membership_field_name_is_reserved() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Plan:
    events:
      created: {}
fsms:
  Operator:
    dag:
      vertex: Plan
    states:
      created:
        initial: true
        attributes:
          dag: string
        to: [done]
      done: {}
",
    );
    assert!(
        errors.contains("`dag` is reserved for generated DAG membership"),
        "{errors}"
    );
}

fn errors_of(model: &str) -> String {
    let Err(Error::Invalid(diagnostics)) = parse_from_str(model, None) else {
        panic!("expected invalid model");
    };
    diagnostics.to_string()
}

fn reference_target(ty: &DataType) -> Option<Path> {
    let DataType::EntityRef { annotations, .. } = ty else {
        return None;
    };
    RefTarget::from_annotations(annotations).map(Path::from)
}
