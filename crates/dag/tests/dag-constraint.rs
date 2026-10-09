// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_dag::{DagConstraint, DagError, DagRole};
use quent_ref_target::RefTargetConstraint;
use quent_schema::{
    Cardinality, DataType, Entity, Field, Schema,
    builder::{AnnotationsBuilder, EntityBuilder, EventBuilder, RecordBuilder, SchemaBuilder},
    test_utils::{event, field, ident, schema},
};

fn ref_to(target: Option<&str>) -> DataType {
    let mut annotations = AnnotationsBuilder::new();
    if let Some(target) = target {
        annotations =
            annotations.with_constraint(RefTargetConstraint::NAME, Some(target.to_string()));
    }
    DataType::EntityRef {
        data: None,
        annotations: annotations.build().unwrap(),
    }
}

fn role_field(name: &str, ty: DataType, role: DagRole) -> Field {
    Field::new(ident(name), ty, role.annotations())
}

fn entity_with_role(
    name: &str,
    role: DagRole,
    events: impl IntoIterator<Item = quent_schema::Event>,
) -> Entity {
    EntityBuilder::new(ident(name))
        .with_events(events)
        .with_annotations(role.annotations())
        .build()
        .unwrap()
}

fn dag(name: &str) -> Entity {
    entity_with_role(name, DagRole::Dag, [event("declared", [])])
}

fn vertex(name: &str, parent: &str) -> Entity {
    entity_with_role(
        name,
        DagRole::Vertex,
        [event(
            "declared",
            [role_field("dag", ref_to(Some(parent)), DagRole::MemberOf)],
        )],
    )
}

fn edge(
    name: &str,
    parent: &str,
    source: DataType,
    target: DataType,
    cardinality: Cardinality,
) -> Entity {
    let declared = EventBuilder::new(ident("declared"), cardinality)
        .with_fields([
            role_field("dag", ref_to(Some(parent)), DagRole::MemberOf),
            role_field("source", source, DagRole::Source),
            role_field("target", target, DagRole::Target),
        ])
        .build()
        .unwrap();
    entity_with_role(name, DagRole::Edge, [declared])
}

fn dag_schema(entities: impl IntoIterator<Item = Entity>) -> Schema {
    schema("S", entities, [])
}

fn errors(schema: &Schema) -> Vec<DagError> {
    match quent_constraints::validate::<(DagConstraint,)>(schema)
        .results
        .0
    {
        Ok(()) => Vec::new(),
        Err(DagError::Multiple(errors)) => errors,
        Err(single) => vec![single],
    }
}

// Accepts a schema with a DAG, a vertex type, and a correctly declared edge.
#[test]
fn valid_dag_types_pass() {
    let plan = dag("Plan");
    let operator = vertex("Operator", "Plan");
    let plan_edge = edge(
        "PlanEdge",
        "Plan",
        ref_to(Some("Operator")),
        ref_to(Some("Operator")),
        Cardinality::Once,
    );

    assert!(errors(&dag_schema([plan, operator, plan_edge])).is_empty());
}

// Rejects membership inside a record without counting it as entity membership.
#[test]
fn membership_must_be_a_direct_event_field() {
    let membership = RecordBuilder::new(ident("Membership"))
        .with_field(role_field("dag", ref_to(Some("Plan")), DagRole::MemberOf))
        .build()
        .unwrap();
    let operator = entity_with_role(
        "Operator",
        DagRole::Vertex,
        [event(
            "declared",
            [field(
                "membership",
                DataType::Record(ident("Membership").into()),
            )],
        )],
    );
    let schema = SchemaBuilder::new(ident("S"))
        .with_entities([dag("Plan"), operator])
        .with_record(membership)
        .build()
        .unwrap();

    let errors = errors(&schema);
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, DagError::MisplacedRole { .. }))
    );
    assert!(errors.iter().any(
        |error| matches!(error, DagError::MissingMemberOf { entity, .. } if entity == "Operator")
    ));
}

// Rejects membership with a non-reference type, no target type, or a non-DAG
// target type.
#[test]
fn membership_requires_a_reference_to_a_dag() {
    let invalid_type = entity_with_role(
        "InvalidType",
        DagRole::Vertex,
        [event(
            "declared",
            [role_field("dag", DataType::Uuid, DagRole::MemberOf)],
        )],
    );
    let untargeted = entity_with_role(
        "Untargeted",
        DagRole::Vertex,
        [event(
            "declared",
            [role_field("dag", ref_to(None), DagRole::MemberOf)],
        )],
    );
    let owner = EntityBuilder::new(ident("Owner"))
        .with_event(event("declared", []))
        .build()
        .unwrap();
    let errors = errors(&dag_schema([
        dag("Plan"),
        invalid_type,
        untargeted,
        owner,
        vertex("WrongTarget", "Owner"),
    ]));

    assert!(
        errors
            .iter()
            .any(|error| matches!(error, DagError::InvalidMemberOfType { .. }))
    );
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, DagError::UntargetedMemberOf { .. }))
    );
    assert!(errors.iter().any(|error| matches!(
        error,
        DagError::MemberOfTargetNotDag { target, .. } if target == "Owner"
    )));
}

// Rejects missing or duplicate membership and membership in repeatable events.
#[test]
fn membership_is_required_unique_and_declared_once() {
    let missing_vertex =
        entity_with_role("MissingVertex", DagRole::Vertex, [event("declared", [])]);
    let missing_edge = entity_with_role(
        "MissingEdge",
        DagRole::Edge,
        [event(
            "declared",
            [
                role_field("source", ref_to(Some("MissingVertex")), DagRole::Source),
                role_field("target", ref_to(Some("MissingVertex")), DagRole::Target),
            ],
        )],
    );
    let duplicate = entity_with_role(
        "Duplicate",
        DagRole::Vertex,
        [event(
            "declared",
            [
                role_field("dag_a", ref_to(Some("Plan")), DagRole::MemberOf),
                role_field("dag_b", ref_to(Some("Plan")), DagRole::MemberOf),
            ],
        )],
    );
    let repeated_event = EventBuilder::new(ident("declared"), Cardinality::Multi)
        .with_field(role_field("dag", ref_to(Some("Plan")), DagRole::MemberOf))
        .build()
        .unwrap();
    let repeated = entity_with_role("Repeated", DagRole::Vertex, [repeated_event]);
    let errors = errors(&dag_schema([
        dag("Plan"),
        missing_vertex,
        missing_edge,
        duplicate,
        repeated,
    ]));

    assert!(errors.iter().any(
        |error| matches!(error, DagError::MissingMemberOf { entity, .. } if entity == "MissingVertex")
    ));
    assert!(errors.iter().any(
        |error| matches!(error, DagError::MissingMemberOf { entity, .. } if entity == "MissingEdge")
    ));
    assert!(errors.iter().any(
        |error| matches!(error, DagError::MultipleMemberOf { entity, .. } if entity == "Duplicate")
    ));
    assert!(errors.iter().any(
        |error| matches!(error, DagError::MemberOfEventNotOnce { entity, .. } if entity == "Repeated")
    ));
}

// Rejects missing or duplicate source and target fields.
#[test]
fn edge_requires_one_source_and_target() {
    for (invalid_role, count) in [
        (DagRole::Source, 0),
        (DagRole::Source, 2),
        (DagRole::Target, 0),
        (DagRole::Target, 2),
    ] {
        let mut fields = vec![role_field("dag", ref_to(Some("Plan")), DagRole::MemberOf)];
        for role in [DagRole::Source, DagRole::Target] {
            let field_count = if role == invalid_role { count } else { 1 };
            for index in 0..field_count {
                fields.push(role_field(
                    &format!("{role}_{index}"),
                    ref_to(Some("Operator")),
                    role,
                ));
            }
        }
        let plan_edge = entity_with_role("PlanEdge", DagRole::Edge, [event("declared", fields)]);
        let errors = errors(&dag_schema([
            dag("Plan"),
            vertex("Operator", "Plan"),
            plan_edge,
        ]));

        assert!(
            match (count, errors.as_slice()) {
                (0, [DagError::MissingEndpoint { role, .. }])
                | (2, [DagError::MultipleEndpoints { role, .. }]) => *role == invalid_role,
                _ => false,
            },
            "unexpected errors for {invalid_role} count {count}: {errors:?}"
        );
    }
}

// Rejects edges whose membership, source, and target do not share one
// once-event.
#[test]
fn edge_topology_must_share_one_once_event() {
    let split_endpoints = entity_with_role(
        "SplitEndpoints",
        DagRole::Edge,
        [
            event(
                "source",
                [
                    role_field("dag", ref_to(Some("Plan")), DagRole::MemberOf),
                    role_field("vertex", ref_to(Some("Operator")), DagRole::Source),
                ],
            ),
            event(
                "target",
                [role_field(
                    "vertex",
                    ref_to(Some("Operator")),
                    DagRole::Target,
                )],
            ),
        ],
    );
    let repeated = edge(
        "RepeatedEdge",
        "Plan",
        ref_to(Some("Operator")),
        ref_to(Some("Operator")),
        Cardinality::Multi,
    );
    let split_membership = entity_with_role(
        "SplitMembership",
        DagRole::Edge,
        [
            event(
                "membership",
                [role_field("dag", ref_to(Some("Plan")), DagRole::MemberOf)],
            ),
            event(
                "endpoints",
                [
                    role_field("source", ref_to(Some("Operator")), DagRole::Source),
                    role_field("target", ref_to(Some("Operator")), DagRole::Target),
                ],
            ),
        ],
    );
    let errors = errors(&dag_schema([
        dag("Plan"),
        vertex("Operator", "Plan"),
        split_endpoints,
        repeated,
        split_membership,
    ]));

    assert!(errors.iter().any(
        |error| matches!(error, DagError::EndpointsInDifferentEvents { edge, .. } if edge == "SplitEndpoints")
    ));
    assert!(errors.iter().any(
        |error| matches!(error, DagError::EndpointEventNotOnce { edge, .. } if edge == "RepeatedEdge")
    ));
    assert!(errors.iter().any(
        |error| matches!(error, DagError::MemberOfInDifferentEvent { edge, .. } if edge == "SplitMembership")
    ));
}

// Rejects non-reference or untyped endpoints and endpoint roles on vertices.
// Only entity references without target types get missing-target errors.
#[test]
fn endpoints_require_typed_entity_references_on_edges() {
    let bad_edge = edge(
        "PlanEdge",
        "Plan",
        DataType::Uuid,
        DataType::String,
        Cardinality::Once,
    );
    let untargeted = edge(
        "UntargetedEdge",
        "Plan",
        ref_to(None),
        ref_to(None),
        Cardinality::Once,
    );
    let misplaced = entity_with_role(
        "Operator",
        DagRole::Vertex,
        [event(
            "declared",
            [
                role_field("dag", ref_to(Some("Plan")), DagRole::MemberOf),
                role_field("other", ref_to(None), DagRole::Source),
            ],
        )],
    );
    let errors = errors(&dag_schema([dag("Plan"), bad_edge, untargeted, misplaced]));

    assert_eq!(
        errors
            .iter()
            .filter(|error| matches!(error, DagError::InvalidEndpointType { .. }))
            .count(),
        2
    );
    assert_eq!(
        errors
            .iter()
            .filter(|error| matches!(error, DagError::UntargetedEndpoint { .. }))
            .count(),
        2
    );
    assert!(errors.iter().any(
        |error| matches!(error, DagError::EndpointOnNonEdge { entity, .. } if entity == "Operator")
    ));
}

// Rejects endpoints that reference non-vertex entities or another DAG type.
#[test]
fn targeted_endpoints_must_name_vertices_in_the_same_dag() {
    let wrong_role = edge(
        "WrongRoleEdge",
        "PlanA",
        ref_to(Some("Helper")),
        ref_to(Some("VertexA")),
        Cardinality::Once,
    );
    let cross_dag = edge(
        "CrossDagEdge",
        "PlanA",
        ref_to(Some("VertexA")),
        ref_to(Some("VertexB")),
        Cardinality::Once,
    );
    let helper = EntityBuilder::new(ident("Helper"))
        .with_event(event("declared", []))
        .build()
        .unwrap();
    let errors = errors(&dag_schema([
        dag("PlanA"),
        dag("PlanB"),
        vertex("VertexA", "PlanA"),
        vertex("VertexB", "PlanB"),
        helper,
        wrong_role,
        cross_dag,
    ]));

    assert!(errors.iter().any(|error| matches!(
        error,
        DagError::EndpointTargetNotVertex { target, .. } if target == "Helper"
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        DagError::EndpointOutsideDag { vertex, .. } if vertex == "VertexB"
    )));
}

// Rejects an unknown role name and a DAG role placed on a record.
#[test]
fn invalid_and_misplaced_roles_are_rejected() {
    let invalid = AnnotationsBuilder::new()
        .with_constraint(DagConstraint::NAME, Some("node".to_string()))
        .build()
        .unwrap();
    let bad_entity = EntityBuilder::new(ident("Bad"))
        .with_event(event("declared", []))
        .with_annotations(invalid)
        .build()
        .unwrap();
    let bad_record = RecordBuilder::new(ident("BadRecord"))
        .with_annotations(DagRole::Dag.annotations())
        .build()
        .unwrap();
    let schema = SchemaBuilder::new(ident("S"))
        .with_entity(bad_entity)
        .with_record(bad_record)
        .build()
        .unwrap();
    let errors = errors(&schema);

    assert!(
        errors
            .iter()
            .any(|error| matches!(error, DagError::InvalidData { .. }))
    );
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, DagError::MisplacedRole { .. }))
    );
}
