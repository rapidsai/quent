// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The shape a model file deserializes into, format `alpha`.
//!
//! These types mirror the YAML one to one. Names stay as plain strings and are
//! checked when the schema is built; the ordered maps keep declaration order.
//!
//! Each element carries its own doc, constraints, and metadata instead of
//! sharing them through one common struct. Folding those in would need serde's
//! `flatten`, which cannot report unknown keys — and a clear error for a
//! mistyped key is worth more than saving the repetition.

use indexmap::IndexMap;
use serde::Deserialize;

use crate::extensions::{
    fsm::FsmSpec,
    reference::{RefForm, ScopeForm},
    resource::{ResourceBoundsField, ResourceDecl, UsesForm},
};

/// A set of constraints or of metadata: each name maps to a payload string, or
/// to nothing when the name is written with no value.
///
/// A payload is a plain scalar. Anything structured, like a list or a nested
/// mapping, is rejected while reading.
pub(crate) type AnnotationMap = IndexMap<String, Option<String>>;

/// A whole model file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Model {
    /// The format version. Only `alpha` is supported.
    pub(crate) quent: String,
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) doc: Option<String>,
    #[serde(default)]
    pub(crate) constraints: AnnotationMap,
    #[serde(default)]
    pub(crate) metadata: AnnotationMap,
    #[serde(default)]
    pub(crate) records: IndexMap<String, Record>,
    #[serde(default)]
    pub(crate) entities: IndexMap<String, Entity>,

    // Built-in extension AST nodes.
    #[serde(default)]
    pub(crate) fsms: IndexMap<String, FsmSpec>,
}

/// A record: named fields plus annotations.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    #[serde(default)]
    pub(crate) doc: Option<String>,
    #[serde(default)]
    pub(crate) constraints: AnnotationMap,
    #[serde(default)]
    pub(crate) metadata: AnnotationMap,
    #[serde(default)]
    pub(crate) fields: IndexMap<String, Field>,
}

/// An entity: named events plus annotations.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entity {
    #[serde(default)]
    pub(crate) doc: Option<String>,
    #[serde(default)]
    pub(crate) constraints: AnnotationMap,
    #[serde(default)]
    pub(crate) metadata: AnnotationMap,
    #[serde(default)]
    pub(crate) events: IndexMap<String, Event>,

    // Built-in extension AST nodes.
    #[serde(default)]
    pub(crate) resource: Option<ResourceDecl>,
}

/// An entity event.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Event {
    #[serde(default)]
    pub(crate) doc: Option<String>,
    #[serde(default)]
    pub(crate) constraints: AnnotationMap,
    #[serde(default)]
    pub(crate) metadata: AnnotationMap,
    #[serde(default)]
    pub(crate) multi: bool,
    #[serde(default)]
    pub(crate) attributes: IndexMap<String, Field>,
}

/// A field declaration.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Field {
    /// A type without annotations.
    Bare(TypeExpr),
    /// A type with annotations.
    Full(Box<FieldBody>),

    // Built-in extension AST nodes.
    /// An attribute carrying a resource's bounds.
    ResourceBounds(ResourceBoundsField),
}

/// The mapping form of a field: a type plus annotations.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldBody {
    pub(crate) r#type: TypeExpr,
    #[serde(default)]
    pub(crate) doc: Option<String>,
    #[serde(default)]
    pub(crate) constraints: AnnotationMap,
    #[serde(default)]
    pub(crate) metadata: AnnotationMap,
}

/// A field's type.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum TypeExpr {
    // Core type expressions.
    /// A reserved bare type keyword.
    Keyword(TypeKeyword),
    /// The name of a record.
    Record(String),
    /// A list of another type.
    List(ListType),
    /// An optional value of another type.
    Option(OptionType),

    // Built-in extension AST nodes.
    /// A targeted entity reference with optional data.
    Ref(RefForm),
    /// A tree-forming entity reference with optional data.
    Scope(ScopeForm),
    /// A reference claiming capacity from a resource.
    Uses(UsesForm),
}

/// Reserved bare type keywords.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TypeKeyword {
    Bool,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    String,
    Uuid,
    Dynamic,
    Ref,
}

/// The list form: a sequence of another type.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListType {
    pub(crate) list: Box<TypeExpr>,
}

/// The option form: a value that may be absent.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OptionType {
    pub(crate) option: Box<TypeExpr>,
}
