// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
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
}

/// One state of an FSM.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StateSpec {
    #[serde(default)]
    pub(crate) initial: bool,
    #[serde(default)]
    pub(crate) attributes: IndexMap<String, Field>,
    // States the FSM can transition to.
    //
    // "exit" is a reserved special name to mark a state as final before
    // dissapearing from existence through the exit transition.
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
}

/// An event: either the short form giving just a cardinality (`started: once`),
/// or a mapping that adds a payload and annotations.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Event {
    OneLiner(Cardinality),
    Body(Box<EventBody>),
}

/// Whether an event fires once per entity instance or repeatedly.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Cardinality {
    Once,
    Multi,
}

/// The mapping form of an event: annotations plus a once or multi payload.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EventBody {
    #[serde(default)]
    pub(crate) doc: Option<String>,
    #[serde(default)]
    pub(crate) constraints: AnnotationMap,
    #[serde(default)]
    pub(crate) metadata: AnnotationMap,
    #[serde(default)]
    pub(crate) once: Option<Payload>,
    #[serde(default)]
    pub(crate) multi: Option<Payload>,
}

/// The fields an event carries, or nothing when it has no payload.
pub(crate) type Payload = Option<IndexMap<String, Field>>;

/// A field: either just its type, or a mapping that adds annotations to it.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Field {
    Bare(TypeExpr),
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
///
/// A bare name is a built-in type (including `ref`, a plain entity reference)
/// or the name of a record. The list and option forms wrap another type. A
/// `ref` or `scope-ref` form names the entity a reference points at and may
/// carry a `data` type; a `scope-ref` additionally marks the reference as
/// tree-forming. Nested types are written as nested YAML, not packed into one
/// string.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum TypeExpr {
    Builtin(BuiltinType),
    Record(String),
    List(ListType),
    Option(OptionType),
    Ref(RefForm),
    Scope(ScopeForm),
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

impl From<Cardinality> for quent_schema::Cardinality {
    fn from(c: Cardinality) -> Self {
        match c {
            Cardinality::Once => quent_schema::Cardinality::Once,
            Cardinality::Multi => quent_schema::Cardinality::Multi,
        }
    }
}
