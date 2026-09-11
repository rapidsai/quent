// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_dag::{
    DagConstraint, DagEntityBuilder, DagEventDecl, DagRole, EndpointDecl, MembershipDecl,
};
use quent_ref_target::{RefTarget, RefTargetConstraint};
use quent_schema::{
    Annotations, Cardinality, DataType, Field, Path,
    builder::{AnnotationsBuilder, EventBuilder, SchemaBuilder},
    test_utils::ident,
};

fn event(name: &str) -> quent_schema::Event {
    EventBuilder::new(ident(name), Cardinality::Once)
        .build()
        .unwrap()
}

#[test]
fn builds_a_valid_dag_schema_fragment() {
    let annotations = AnnotationsBuilder::new()
        .with_docs("Arbitrary entity annotations.")
        .build()
        .unwrap();

    let dag = DagEntityBuilder::dag(ident("Plan"))
        .with_annotations(annotations)
        .with_event(event("created"))
        .build()
        .unwrap();
    let vertex = DagEntityBuilder::vertex(
        ident("Operator"),
        DagEventDecl::new(ident("declared")).with_attribute(Field::new(
            ident("name"),
            DataType::String,
            Annotations::default(),
        )),
        MembershipDecl::new(ident("dag"), ident("Plan")),
    )
    .with_event(event("updated"))
    .build()
    .unwrap();
    let edge = DagEntityBuilder::edge(
        ident("PlanEdge"),
        DagEventDecl::new(ident("declared")),
        MembershipDecl::new(ident("dag"), ident("Plan")),
        EndpointDecl::new(ident("source"), ident("Operator")),
        EndpointDecl::new(ident("target"), ident("Operator")),
    )
    .build()
    .unwrap();
    let schema = SchemaBuilder::new(ident("QueryPlan"))
        .with_entities([dag, vertex, edge])
        .build()
        .unwrap();

    let validation =
        quent_constraints::validate::<(RefTargetConstraint, DagConstraint)>(&schema).results;
    assert!(validation.0.is_ok());
    assert!(validation.1.is_ok());
}

#[test]
fn preserves_custom_annotations_and_reference_data() {
    let field_annotations = AnnotationsBuilder::new()
        .with_docs("Membership field.")
        .build()
        .unwrap();
    let reference_annotations = AnnotationsBuilder::new()
        .with_docs("Reference type.")
        .build()
        .unwrap();

    let vertex = DagEntityBuilder::vertex(
        ident("Operator"),
        DagEventDecl::new(ident("declared")),
        MembershipDecl::new(ident("owner"), ident("Plan"))
            .with_data(DataType::Uuid)
            .with_field_annotations(field_annotations)
            .with_reference_annotations(reference_annotations),
    )
    .build()
    .unwrap();

    let field = vertex
        .event(&ident("declared"))
        .unwrap()
        .field(&ident("owner"))
        .unwrap();
    assert_eq!(field.annotations().docs(), Some("Membership field."));
    assert_eq!(
        DagRole::from_annotations(field.annotations()),
        Ok(Some(DagRole::Membership))
    );
    let DataType::EntityRef { data, annotations } = field.ty() else {
        panic!("membership must be an entity reference");
    };
    assert_eq!(data.as_deref(), Some(&DataType::Uuid));
    assert_eq!(annotations.docs(), Some("Reference type."));
    assert_eq!(
        RefTarget::from_annotations(annotations).map(Path::from),
        Some(Path::from(ident("Plan")))
    );
}

#[test]
fn rejects_declaration_field_name_collisions() {
    let result = DagEntityBuilder::vertex(
        ident("Operator"),
        DagEventDecl::new(ident("declared")).with_attribute(Field::new(
            ident("dag"),
            DataType::String,
            Annotations::default(),
        )),
        MembershipDecl::new(ident("dag"), ident("Plan")),
    )
    .build();

    assert!(result.is_err());
}
