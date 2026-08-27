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
    #[serde(default)]
    pub(crate) fsms: IndexMap<String, FsmSpec>,
}

/// An FSM entity: annotations plus its states.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FsmSpec {
    #[serde(default)]
    pub(crate) doc: Option<String>,
    #[serde(default)]
    pub(crate) constraints: AnnotationMap,
    #[serde(default)]
    pub(crate) metadata: AnnotationMap,
    pub(crate) states: IndexMap<String, StateSpec>,
    #[serde(default)]
    pub(crate) resource: Option<ResourceDecl>,
}

/// One state of an FSM.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StateSpec {
    #[serde(default)]
    pub(crate) initial: bool,
    #[serde(default)]
    pub(crate) attributes: IndexMap<String, Field>,
    /// States the FSM can transition to. An empty list makes this a final state.
    #[serde(default)]
    pub(crate) to: Vec<String>,
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
    /// An attribute carrying a resource's bounds.
    ResourceBounds(ResourceBoundsField),
    /// A type without annotations.
    Bare(TypeExpr),
    /// A type with annotations.
    Full(Box<FieldBody>),
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
    /// A built-in type name, including `ref` for an untargeted entity reference.
    Builtin(BuiltinType),
    /// The name of a record.
    Record(String),
    /// A list of another type.
    List(ListType),
    /// An optional value of another type.
    Option(OptionType),
    /// A targeted entity reference with optional data.
    Ref(RefForm),
    /// A tree-forming entity reference with optional data.
    Scope(ScopeForm),
    /// A reference claiming capacity from a resource.
    Uses(UsesForm),
}

/// The bare names that stand for a built-in type. Each is written lowercase in
/// the YAML (`u8`, `string`, `ref`, and so on).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BuiltinType {
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

/// A targeted entity reference: `ref` names the entity it points at, with an
/// optional `data` type the reference carries.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RefForm {
    pub(crate) r#ref: String,
    #[serde(default)]
    pub(crate) data: Option<Box<TypeExpr>>,
}

/// A tree-forming targeted reference: `scope-ref` names the entity it points
/// at and marks the reference as part of the scoping tree, with an optional
/// `data` type the reference carries.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScopeForm {
    #[serde(rename = "scope-ref")]
    pub(crate) scope_ref: String,
    #[serde(default)]
    pub(crate) data: Option<Box<TypeExpr>>,
}

/// A resource declaration.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ResourceDecl {
    Unit(bool),
    Detailed(ResourceSpec),
    Capacities(IndexMap<String, CapacitySpec>),
}

/// A resource declaration with generated record name overrides.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResourceSpec {
    #[serde(default)]
    pub(crate) capacities: IndexMap<String, CapacitySpec>,
    #[serde(default, rename = "usage-record")]
    pub(crate) usage_record: Option<String>,
    #[serde(default, rename = "bounds-record")]
    pub(crate) bounds_record: Option<String>,
}

impl ResourceDecl {
    pub(crate) fn usage_record(&self) -> Option<&str> {
        match self {
            Self::Detailed(spec) => spec.usage_record.as_deref(),
            _ => None,
        }
    }

    pub(crate) fn bounds_record(&self) -> Option<&str> {
        match self {
            Self::Detailed(spec) => spec.bounds_record.as_deref(),
            _ => None,
        }
    }
}

/// One capacity of a resource.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacitySpec {
    pub(crate) kind: quent_resource::CapacityKind,
    #[serde(default, rename = "known-bounds")]
    pub(crate) known_bounds: bool,
}

/// Marks an event attribute as carrying the resource's bounds.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct ResourceBoundsField {
    pub(crate) sets_resource_bounds: bool,
}

/// A resource usage reference.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UsesForm {
    pub(crate) uses: String,
}
