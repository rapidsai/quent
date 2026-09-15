// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! DAG forms and their schema elaboration.

use quent_dag::DagRole;
use quent_schema::builder::AnnotationsBuilder;
use quent_schema::{Field, Identifier};
use serde::Deserialize;

use crate::diag::Diagnostics;
use crate::extensions::reference::entity_ref_type;

/// A DAG entity role.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum DagEntityDecl {
    Marker(bool),
    Role(DagEntityRole),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DagEntityRole {
    Vertex,
    Edge,
}

/// A field forming a `member-of` or edge endpoint relation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DagRelationField {
    pub(crate) dag: DagFieldDecl,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum DagFieldDecl {
    MemberOf(DagMemberOfDecl),
    Source(DagSourceDecl),
    Target(DagTargetDecl),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DagMemberOfDecl {
    #[serde(rename = "member-of")]
    pub(crate) member_of: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DagSourceDecl {
    pub(crate) source: String,
}

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
        DagFieldDecl::MemberOf(declaration) => (&declaration.member_of, DagRole::MemberOf),
        DagFieldDecl::Source(declaration) => (&declaration.source, DagRole::Source),
        DagFieldDecl::Target(declaration) => (&declaration.target, DagRole::Target),
    };
    Some(Field::new(
        name?,
        entity_ref_type(target, None, false, path, sink)?,
        role.annotations(),
    ))
}
