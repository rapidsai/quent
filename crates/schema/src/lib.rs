// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! # Schema of Application Event Models
//!
//! This module defines types of core concepts necessary to express the schema
//! of an application event model. The schema captures the information necessary
//! to write and read all events without further interpretation.
//!
//! ## Core Concepts
//!
//! The schema core concepts are:
//!
//! - [`Identifier`]: defines the name of things
//! - [`DataType`]: defines common data types (bool, integer, string, etc.)
//!   plus Quent-specific types, such as:
//!     - [`DataType::EntityRef`]
//!     - [`DataType::DynamicRecord`]
//! - [`Event`]: defines an event type that applications can emit.
//! - [`Entity`]: defines a uniquely identifiable type of thing that emits
//!   some set of related events.
//! - [`Schema`]: defines a type of uniquely identifiable collection of
//!   entities that are somehow related. Top-level of an application event
//!   model.
//! - [`Annotations`]: opaque metadata and doc strings for most other core
//!   concepts.
//!
//! ## Purpose
//!
//! The schema is leveraged for (cross-language) code generation, model
//! validation, and serialization.
//!
//! Code generation involves generating cross-language compatible bridge code,
//! e.g. for C++ through a CXX bridge, or for Python through PyO3.
//!
//! Model validation involves checking certain constraints placed on a model
//! captured in the schema. These might need to be met to even succeed in
//! constructing the schema in-memory using this crate, e.g. by ensuring that
//! [`Identifier`]s are accepted by the prescribed grammar. Some constraints may
//! be about event attributes or order, e.g. that FSM states are reachable from
//! the entry transition and an exit transition is reachable from all states.
//! The latter is an example of a constraint that is not expressed through a
//! core schema concept, because it does not contribute to the ability to write
//! or read events.
//!
//! Serialization involves the ability to store a model, which can be leveraged
//! for model re-use, sharing, and archival purposes.
//!
//! ## Annotations
//!
//! The schema is kept as minimal as possible in order to prevent contamination
//! and complexity from concerns imbued by application-specific semantics or
//! constraints, as well as concerns from modeling APIs or DSLs, and from code
//! generation flows for either instrumentation or analysis APIs.
//!
//! All of these concerns can instead be attached to most core schema types as
//! [`annotations::Annotations`]. Three types of annotations exist:
//!
//! - [`Constraint`]: rules applicable to the model that must hold for the
//!   schema to be logically sound. Constraints require validation against the
//!   entire schema if it cannot be guaranteed that a schema is logically sound
//!   (e.g. after deserialization or construction throug some DSL parser).
//! - [`Metadata`]: opaque data carried through the schema, e.g. to produce a
//!   more user-friendly instrumentation API or to feed code generation. It
//!   carries no validation requirement.
//! - [`Annotations::docs`]: can be used to add user-facing documentation e.g.
//!   in instrumentation API code generation.
//!
//! Quent provides various built-in constraints, also see the crates
//! `quent-fsm`, `quent-ref-target`, etc.
//!
//! Note that this approach promotes a stronger guarantee against breaking
//! changes. For example, even if a new constraint is added, but code generation
//! does not yet support it, it will still be able to produce an instrumentation
//! API that allows users to emit events that may have been defined as a result
//! of the new constraint. Users of the code generator may not yet get the
//! benefit of some potential elegant type-safe API better expressing these
//! constraints, preventing certain illogical behavior violating te constraint
//! at compile-time, but everything will "still work".
//!
//! In order to validate constraints against the schema, a lightweight canonical
//! mechanism exists in the `quent-constraints` crate. It is strongly
//! recommended to perform this validation after constructing the schema from
//! any source that isn't inherently guaranteed to validate.
//!
//! ## Binary Format
//!
//! There is no stable binary format for schemas yet. As a stop-gap solution for
//! serializing schemas, this crate has a `serde` feature.

pub use schema::{
    Schema, annotations::Annotations, constraint::Constraint, data_type::DataType, entity::Entity,
    event::Cardinality, event::Event, field::Field, identifier::Identifier, metadata::Metadata,
    record::Record,
};

pub mod builder;
pub mod schema;
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
#[cfg(feature = "visitor")]
pub mod visitor;
