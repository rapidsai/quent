// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates event payloads, records, entity markers, and model types from Quent schemas.

pub mod common;
pub mod data_type;
mod events;
mod model;
pub mod namespace;
mod records;
#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use quent_constraints::{BaseConstraintsError, Report};
use quent_ref_target::{RefTargetConstraint, RefTargetError};
use quent_schema::Schema;
use quote::quote;

/// Validates base and reference-target constraints and returns unregistered constraint names.
///
/// # Errors
///
/// Returns an error when base schema or reference-target validation fails.
pub fn validate_schema(schema: &Schema) -> Result<Vec<String>, Error> {
    let Report {
        base_constraints,
        unregistered_constraints,
        results,
    } = quent_constraints::validate::<(RefTargetConstraint,)>(schema);
    base_constraints?;
    let (ref_target,) = results;
    ref_target?;
    Ok(unregistered_constraints)
}

/// Returns event types using runtime paths supplied by the calling generator.
///
/// # Errors
///
/// Returns validation or type-generation errors.
pub fn generate_str(schema: &Schema, opts: &Options, runtime: &Runtime) -> Result<String, Error> {
    validate_schema(schema)?;
    generate_str_unvalidated(schema, opts, runtime)
}

/// Generates declarations for one schema namespace using the supplied runtime paths.
///
/// # Errors
///
/// Returns an error for invalid derives or excessive nesting in a schema event
/// or record field's type.
pub fn generate_namespace(
    schema: &Schema,
    opts: &Options,
    namespace: &namespace::Namespace<'_>,
    runtime: &Runtime,
) -> Result<NamespaceCode, Error> {
    let records = namespace
        .records()
        .iter()
        .map(|record| records::record_struct(record, opts, runtime))
        .collect::<Result<Vec<_>, _>>()?;
    let events = namespace
        .entities()
        .iter()
        .map(|entity| events::entity_event_payload_enum(entity, opts, runtime))
        .collect::<Result<Vec<_>, _>>()?;
    let markers = namespace
        .entities()
        .iter()
        .map(|entity| events::entity_types(entity, runtime));
    let model = model::generate(schema, namespace, opts, runtime)?;
    Ok(NamespaceCode {
        declarations: quote! { #(#records)* #(#events)* #(#markers)* },
        model,
    })
}

/// Options controlling event type generation.
pub struct Options {
    /// Derive [`Debug`](std::fmt::Debug) on generated event and record types.
    pub debug: bool,

    /// Derive `serde::Serialize` and `serde::Deserialize` on generated event
    /// and record types.
    ///
    pub serde: bool,

    /// Derives applied to every generated event payload enum.
    ///
    /// Use [`Self::debug`] and [`Self::serde`] for the built-in derives.
    pub event_derives: &'static [&'static str],

    /// Derives applied to every generated record struct.
    ///
    /// Use [`Self::debug`] and [`Self::serde`] for the built-in derives.
    pub record_derives: &'static [&'static str],

    /// Emit model-wide combined event enums and implement the combined
    /// capability for the generated model.
    ///
    /// No namespace enum is emitted without entity events, except at the root.
    pub combined_event: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            debug: true,
            serde: false,
            event_derives: Default::default(),
            record_derives: Default::default(),
            combined_event: false,
        }
    }
}

/// An error from validating or generating event types.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("base schema validation failed: {0}")]
    InvalidSchema(#[from] BaseConstraintsError),
    #[error("reference-target validation failed: {0}")]
    InvalidRefTarget(#[from] RefTargetError),
    #[error("invalid derive path {derive:?}")]
    InvalidDerive { derive: String, source: syn::Error },
    #[error("generated code did not form a valid Rust file")]
    InvalidGeneratedCode(#[source] syn::Error),
    #[error("field type nesting exceeds the maximum depth of {max}")]
    TypeNestingTooDeep { max: usize },
}

/// Generated declarations for one schema namespace.
pub struct NamespaceCode {
    /// Records, per-entity event enums, and entity markers.
    pub declarations: TokenStream,
    /// Combined events and model metadata.
    pub model: TokenStream,
}

/// Runtime paths supplied by a calling code generator.
pub struct Runtime {
    /// Module exporting event types, model traits, references, and build
    /// metadata.
    pub events: syn::Path,
    /// Trait implemented by generated entity markers.
    pub entity_marker: syn::Path,
    /// Optional Serde re-export used by derives and their crate attribute.
    pub serde: Option<syn::Path>,
}

/// Generates formatted Rust source for the entire schema.
///
/// The caller must validate the schema with [`validate_schema`] first.
fn generate_str_unvalidated(
    schema: &Schema,
    opts: &Options,
    runtime: &Runtime,
) -> Result<String, Error> {
    let namespaces = namespace::Namespace::root(schema);
    let reexports = events::reexports(runtime);
    let types = generate_tree(schema, opts, &namespaces, runtime)?;
    let file = syn::parse2::<syn::File>(quote! { #reexports #types })
        .map_err(Error::InvalidGeneratedCode)?;
    Ok(prettyplease::unparse(&file))
}

/// Generates Rust types for a schema namespace and all its nested namespaces.
fn generate_tree(
    schema: &Schema,
    opts: &Options,
    namespace: &namespace::Namespace<'_>,
    runtime: &Runtime,
) -> Result<TokenStream, Error> {
    let NamespaceCode {
        declarations,
        model,
    } = generate_namespace(schema, opts, namespace, runtime)?;
    let children = namespace
        .children()
        .iter()
        .map(|child| {
            let segment = child
                .path()
                .last()
                .expect("child namespaces extend their parent");
            let module = common::module_ident(segment);
            let contents = generate_tree(schema, opts, child, runtime)?;
            Ok::<_, Error>(quote! { pub mod #module { #contents } })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! { #declarations #(#children)* #model })
}
