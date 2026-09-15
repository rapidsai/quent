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
    dag: true
    events:
      created: {}
  Operator:
    dag: vertex
    events:
      declared:
        attributes:
          plan:
            dag: { member-of: Plan }
  PlanEdge:
    dag: edge
    events:
      connected:
        attributes:
          plan:
            dag: { member-of: Plan }
          input:
            dag: { source: Operator }
          output:
            dag: { target: Operator }
";

#[test]
fn dag_syntax_builds_and_validates_schema() {
    let parsed = parse_from_str(MODEL, None).expect("valid DAG model");
    assert!(parsed.warnings.is_empty());

    let plan = parsed.schema.entity(&path("Plan")).unwrap();
    assert_eq!(
        DagRole::from_annotations(plan.annotations()),
        Ok(Some(DagRole::Dag))
    );

    let operator = parsed.schema.entity(&path("Operator")).unwrap();
    assert_eq!(
        DagRole::from_annotations(operator.annotations()),
        Ok(Some(DagRole::Vertex))
    );
    let member_of = operator
        .event(&"declared".try_into().unwrap())
        .unwrap()
        .field(&"plan".try_into().unwrap())
        .unwrap();
    assert_eq!(
        DagRole::from_annotations(member_of.annotations()),
        Ok(Some(DagRole::MemberOf))
    );
    assert_eq!(reference_target(member_of.ty()), Some(path("Plan")));

    let edge = parsed.schema.entity(&path("PlanEdge")).unwrap();
    assert_eq!(
        DagRole::from_annotations(edge.annotations()),
        Ok(Some(DagRole::Edge))
    );
    let connected = edge.event(&"connected".try_into().unwrap()).unwrap();
    for (field, role, target) in [
        ("plan", DagRole::MemberOf, "Plan"),
        ("input", DagRole::Source, "Operator"),
        ("output", DagRole::Target, "Operator"),
    ] {
        let field = connected.field(&field.try_into().unwrap()).unwrap();
        assert_eq!(
            DagRole::from_annotations(field.annotations()),
            Ok(Some(role))
        );
        assert_eq!(reference_target(field.ty()), Some(path(target)));
    }
}

#[test]
fn false_dag_marker_adds_no_role() {
    let parsed = parse_from_str(
        "\
quent: alpha
model: m
entities:
  Plain:
    dag: false
    events:
      declared: {}
",
        None,
    )
    .expect("valid non-DAG entity");
    let plain = parsed.schema.entity(&path("Plain")).unwrap();
    assert_eq!(DagRole::from_annotations(plain.annotations()), Ok(None));
}

#[test]
fn dag_roles_do_not_generate_topology() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Plan:
    dag: true
    events:
      created: {}
  Operator:
    dag: vertex
    events:
      declared: {}
",
    );
    assert!(
        errors.contains("vertex entity \"Operator\" has no `member-of` reference"),
        "{errors}"
    );
}

#[test]
fn member_of_target_must_be_a_dag() {
    let invalid = MODEL.replace("    dag: true\n", "    dag: false\n");
    let errors = errors_of(&invalid);
    assert!(
        errors.contains("`member-of` targets \"Plan\", which is not a DAG entity"),
        "{errors}"
    );
}

#[test]
fn endpoint_target_must_be_a_vertex() {
    let errors = errors_of(&MODEL.replace("source: Operator", "source: Plan"));
    assert!(
        errors.contains("`source` targets \"Plan\", which is not a vertex entity"),
        "{errors}"
    );
}

#[test]
fn topology_event_must_not_be_multi() {
    let invalid = MODEL.replace(
        "      connected:\n",
        "      connected:\n        multi: true\n",
    );
    let errors = errors_of(&invalid);
    assert!(
        errors.contains("edge entity \"PlanEdge\" declares endpoints in multi-event \"connected\""),
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
fn fsm_vertex_can_declare_membership() {
    let parsed = parse_from_str(
        "\
quent: alpha
model: m
entities:
  Plan:
    dag: true
    events:
      created: {}
fsms:
  Operator:
    dag: vertex
    states:
      created:
        initial: true
        attributes:
          plan:
            dag: { member-of: Plan }
        to: [done]
      done: {}
",
        None,
    )
    .expect("valid FSM vertex");

    let operator = parsed.schema.entity(&path("Operator")).unwrap();
    let member_of = operator
        .event(&"created".try_into().unwrap())
        .unwrap()
        .field(&"plan".try_into().unwrap())
        .unwrap();
    assert_eq!(
        DagRole::from_annotations(member_of.annotations()),
        Ok(Some(DagRole::MemberOf))
    );
}

#[test]
fn fsm_edge_can_declare_topology() {
    let parsed = parse_from_str(
        "\
quent: alpha
model: m
entities:
  Plan:
    dag: true
    events:
      created: {}
  Operator:
    dag: vertex
    events:
      declared:
        attributes:
          plan:
            dag: { member-of: Plan }
fsms:
  PlanEdge:
    dag: edge
    states:
      connected:
        initial: true
        attributes:
          plan:
            dag: { member-of: Plan }
          input:
            dag: { source: Operator }
          output:
            dag: { target: Operator }
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
                .field(&"plan".try_into().unwrap())
                .unwrap()
                .annotations()
        ),
        Ok(Some(DagRole::MemberOf))
    );
}

#[test]
fn fsm_edge_topology_must_share_a_state() {
    let errors = errors_of(
        "\
quent: alpha
model: m
entities:
  Plan:
    dag: true
    events:
      created: {}
  Operator:
    dag: vertex
    events:
      declared:
        attributes:
          plan:
            dag: { member-of: Plan }
fsms:
  PlanEdge:
    dag: edge
    states:
      connected:
        initial: true
        attributes:
          plan:
            dag: { member-of: Plan }
          input:
            dag: { source: Operator }
        to: [completed]
      completed:
        attributes:
          output:
            dag: { target: Operator }
",
    );
    assert!(
        errors.contains(
            "declares its source in event \"connected\" and target in event \"completed\""
        ),
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
