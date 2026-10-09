// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! DAG forms and their schema elaboration.

use quent_dag::DagRole;
use quent_schema::builder::AnnotationsBuilder;
use quent_schema::{Field, Identifier};
use serde::Deserialize;

use crate::diag::Diagnostics;
use crate::extensions::reference::entity_ref_type;

/// An entity's `dag:` value: `true` for a DAG, `false` for no role, or a
/// constituent role.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum DagEntityDecl {
    Marker(bool),
    Role(DagEntityRole),
}

/// Named `dag:` values for vertex and directed edge entities.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DagEntityRole {
    Vertex,
    Edge,
}

/// An event field declared with a `dag:` topology relation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DagRelationField {
    pub(crate) dag: DagFieldDecl,
}

/// A `dag:` mapping with exactly one `in`, `source`, or `target` entity type.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum DagFieldDecl {
    MemberOf(DagMemberOfDecl),
    Source(DagSourceDecl),
    Target(DagTargetDecl),
}

/// The `{ in: DagType }` mapping naming a vertex or edge's containing DAG type.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DagMemberOfDecl {
    #[serde(rename = "in")]
    pub(crate) target: String,
}

/// The `{ source: VertexType }` mapping naming a directed edge's source vertex
/// type.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DagSourceDecl {
    pub(crate) source: String,
}

/// The `{ target: VertexType }` mapping naming a directed edge's target vertex
/// type.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DagTargetDecl {
    pub(crate) target: String,
}

pub(crate) fn attach_entity_role(
    annotations: AnnotationsBuilder,
    declaration: Option<&DagEntityDecl>,
) -> AnnotationsBuilder {
    let role = match declaration {
        None | Some(DagEntityDecl::Marker(false)) => return annotations,
        Some(DagEntityDecl::Marker(true)) => DagRole::Dag,
        Some(DagEntityDecl::Role(DagEntityRole::Vertex)) => DagRole::Vertex,
        Some(DagEntityDecl::Role(DagEntityRole::Edge)) => DagRole::Edge,
    };
    role.annotate(annotations)
}

pub(crate) fn elaborate_field(
    name: Option<Identifier>,
    relation: &DagRelationField,
    path: &str,
    sink: &mut Diagnostics,
) -> Option<Field> {
    let (target, role) = match &relation.dag {
        DagFieldDecl::MemberOf(declaration) => (&declaration.target, DagRole::MemberOf),
        DagFieldDecl::Source(declaration) => (&declaration.source, DagRole::Source),
        DagFieldDecl::Target(declaration) => (&declaration.target, DagRole::Target),
    };
    Some(Field::new(
        name?,
        entity_ref_type(target, None, false, path, sink)?,
        role.annotations(),
    ))
}
