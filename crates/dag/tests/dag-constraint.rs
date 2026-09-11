// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_dag::{DagConstraint, DagError, DagRole, DagRoleParseError};
use quent_ref_target::RefTargetConstraint;
use quent_schema::{
    Annotations, Cardinality, DataType, Entity, Field, Schema,
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
            [role_field("dag", ref_to(Some(parent)), DagRole::Membership)],
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
            role_field("dag", ref_to(Some(parent)), DagRole::Membership),
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

#[test]
fn roles_round_trip_through_annotations() {
    for role in [
        DagRole::Dag,
        DagRole::Vertex,
        DagRole::Edge,
        DagRole::Membership,
        DagRole::Source,
        DagRole::Target,
    ] {
        assert_eq!(
            DagRole::from_annotations(&role.annotations()),
            Ok(Some(role))
        );
        assert_eq!(role.to_string().parse(), Ok(role));
    }
    assert_eq!(
        "node".parse::<DagRole>(),
        Err(DagRoleParseError::UnknownRole("node".to_string()))
    );

    let annotations = DagRole::Vertex
        .annotate(AnnotationsBuilder::new().with_docs("An operator."))
        .build()
        .unwrap();
    assert_eq!(annotations.docs(), Some("An operator."));
    assert_eq!(
        DagRole::from_annotations(&annotations),
        Ok(Some(DagRole::Vertex))
    );
}

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

#[test]
fn type_erased_endpoints_are_rejected() {
    let plan_edge = edge(
        "PlanEdge",
        "Plan",
        ref_to(None),
        ref_to(None),
        Cardinality::Once,
    );
    let errors = errors(&dag_schema([
        dag("Plan"),
        vertex("Scan", "Plan"),
        vertex("Filter", "Plan"),
        plan_edge,
    ]));
    assert_eq!(
        errors
            .iter()
            .filter(|error| matches!(error, DagError::UntargetedEndpoint { .. }))
            .count(),
        2
    );
}

#[test]
fn vertex_and_edge_require_membership() {
    let operator = entity_with_role(
        "Operator",
        DagRole::Vertex,
        [event("declared", [field("name", DataType::String)])],
    );
    let plan_edge = entity_with_role(
        "PlanEdge",
        DagRole::Edge,
        [event(
            "declared",
            [
                role_field("source", ref_to(None), DagRole::Source),
                role_field("target", ref_to(None), DagRole::Target),
            ],
        )],
    );
    let errors = errors(&dag_schema([dag("Plan"), operator, plan_edge]));

    assert!(errors.iter().any(
        |error| matches!(error, DagError::MissingMembership { entity, .. } if entity == "Operator")
    ));
    assert!(errors.iter().any(
        |error| matches!(error, DagError::MissingMembership { entity, .. } if entity == "PlanEdge")
    ));
}

#[test]
fn membership_target_must_be_a_dag() {
    let owner = EntityBuilder::new(ident("Owner"))
        .with_event(event("declared", []))
        .build()
        .unwrap();
    let errors = errors(&dag_schema([owner, vertex("Operator", "Owner")]));

    assert!(errors.iter().any(|error| matches!(
        error,
        DagError::MembershipTargetNotDag { target, .. } if target == "Owner"
    )));
}

#[test]
fn membership_must_be_a_direct_event_field() {
    let membership = RecordBuilder::new(ident("Membership"))
        .with_field(role_field("dag", ref_to(Some("Plan")), DagRole::Membership))
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
        |error| matches!(error, DagError::MissingMembership { entity, .. } if entity == "Operator")
    ));
}

#[test]
fn membership_requires_a_targeted_entity_reference() {
    let invalid_type = entity_with_role(
        "InvalidType",
        DagRole::Vertex,
        [event(
            "declared",
            [role_field("dag", DataType::Uuid, DagRole::Membership)],
        )],
    );
    let untargeted = entity_with_role(
        "Untargeted",
        DagRole::Vertex,
        [event(
            "declared",
            [role_field("dag", ref_to(None), DagRole::Membership)],
        )],
    );
    let errors = errors(&dag_schema([dag("Plan"), invalid_type, untargeted]));

    assert!(
        errors
            .iter()
            .any(|error| matches!(error, DagError::InvalidMembershipType { .. }))
    );
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, DagError::UntargetedMembership { .. }))
    );
}

#[test]
fn membership_is_unique_and_declared_once() {
    let duplicate = entity_with_role(
        "Duplicate",
        DagRole::Vertex,
        [event(
            "declared",
            [
                role_field("dag_a", ref_to(Some("Plan")), DagRole::Membership),
                role_field("dag_b", ref_to(Some("Plan")), DagRole::Membership),
            ],
        )],
    );
    let repeated_event = EventBuilder::new(ident("declared"), Cardinality::Multi)
        .with_field(role_field("dag", ref_to(Some("Plan")), DagRole::Membership))
        .build()
        .unwrap();
    let repeated = entity_with_role("Repeated", DagRole::Vertex, [repeated_event]);
    let errors = errors(&dag_schema([dag("Plan"), duplicate, repeated]));

    assert!(errors.iter().any(
        |error| matches!(error, DagError::MultipleMemberships { entity, .. } if entity == "Duplicate")
    ));
    assert!(errors.iter().any(
        |error| matches!(error, DagError::MembershipEventNotOnce { entity, .. } if entity == "Repeated")
    ));
}

#[test]
fn edge_requires_one_source_and_target() {
    let missing_target = entity_with_role(
        "MissingTarget",
        DagRole::Edge,
        [event(
            "declared",
            [
                role_field("dag", ref_to(Some("Plan")), DagRole::Membership),
                role_field("source", ref_to(None), DagRole::Source),
            ],
        )],
    );
    let duplicate_source = entity_with_role(
        "DuplicateSource",
        DagRole::Edge,
        [event(
            "declared",
            [
                role_field("dag", ref_to(Some("Plan")), DagRole::Membership),
                role_field("source_a", ref_to(None), DagRole::Source),
                role_field("source_b", ref_to(None), DagRole::Source),
                role_field("target", ref_to(None), DagRole::Target),
            ],
        )],
    );
    let errors = errors(&dag_schema([dag("Plan"), missing_target, duplicate_source]));

    assert!(errors.iter().any(|error| matches!(
        error,
        DagError::MissingEndpoint { edge, role: DagRole::Target } if edge == "MissingTarget"
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        DagError::MultipleEndpoints { edge, role: DagRole::Source, .. }
            if edge == "DuplicateSource"
    )));
}

#[test]
fn endpoints_must_share_one_once_event() {
    let split = entity_with_role(
        "SplitEdge",
        DagRole::Edge,
        [
            event(
                "source",
                [
                    role_field("dag", ref_to(Some("Plan")), DagRole::Membership),
                    role_field("vertex", ref_to(None), DagRole::Source),
                ],
            ),
            event(
                "target",
                [role_field("vertex", ref_to(None), DagRole::Target)],
            ),
        ],
    );
    let repeated = edge(
        "RepeatedEdge",
        "Plan",
        ref_to(None),
        ref_to(None),
        Cardinality::Multi,
    );
    let errors = errors(&dag_schema([dag("Plan"), split, repeated]));

    assert!(errors.iter().any(
        |error| matches!(error, DagError::EndpointsInDifferentEvents { edge, .. } if edge == "SplitEdge")
    ));
    assert!(errors.iter().any(
        |error| matches!(error, DagError::EndpointEventNotOnce { edge, .. } if edge == "RepeatedEdge")
    ));
}

#[test]
fn edge_membership_and_endpoints_share_an_event() {
    let split = entity_with_role(
        "SplitEdge",
        DagRole::Edge,
        [
            event(
                "membership",
                [role_field("dag", ref_to(Some("Plan")), DagRole::Membership)],
            ),
            event(
                "endpoints",
                [
                    role_field("source", ref_to(None), DagRole::Source),
                    role_field("target", ref_to(None), DagRole::Target),
                ],
            ),
        ],
    );
    let errors = errors(&dag_schema([dag("Plan"), split]));

    assert!(errors.iter().any(
        |error| matches!(error, DagError::MembershipInDifferentEvent { edge, .. } if edge == "SplitEdge")
    ));
}

#[test]
fn endpoint_roles_require_entity_references_on_edges() {
    let bad_edge = edge(
        "PlanEdge",
        "Plan",
        DataType::Uuid,
        DataType::String,
        Cardinality::Once,
    );
    let misplaced = entity_with_role(
        "Operator",
        DagRole::Vertex,
        [event(
            "declared",
            [
                role_field("dag", ref_to(Some("Plan")), DagRole::Membership),
                role_field("other", ref_to(None), DagRole::Source),
            ],
        )],
    );
    let errors = errors(&dag_schema([dag("Plan"), bad_edge, misplaced]));

    assert_eq!(
        errors
            .iter()
            .filter(|error| matches!(error, DagError::InvalidEndpointType { .. }))
            .count(),
        2
    );
    assert!(
        errors
            .iter()
            .all(|error| !matches!(error, DagError::UntargetedEndpoint { .. }))
    );
    assert!(errors.iter().any(
        |error| matches!(error, DagError::EndpointOnNonEdge { entity, .. } if entity == "Operator")
    ));
}

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

#[test]
fn absent_dag_annotations_are_ignored() {
    let plain = EntityBuilder::new(ident("Plain"))
        .with_event(event("declared", []))
        .with_annotations(Annotations::default())
        .build()
        .unwrap();
    assert!(errors(&dag_schema([plain])).is_empty());
}
