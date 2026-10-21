// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of model-wide types.

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quent_events_build::common::module_ident;
use quent_schema::Schema;

use crate::GenerateError;

mod ref_tree;
mod store;

#[cfg(test)]
mod tests;

/// Generates model-wide stores and typed tree traversal.
pub(super) fn generate(schema: &Schema, events: &syn::File) -> Result<TokenStream, GenerateError> {
    let code = store::generate(schema, events)?;
    let store = store::generate_model(&code);
    let tree = ref_tree::generate(schema, events)?;
    Ok(quote::quote! { #store #tree })
}

/// Returns distinct Rust names for schema entities.
fn entity_names(schema: &Schema) -> Vec<syn::Ident> {
    let mut field_names = BTreeSet::new();
    schema
        .entities()
        .map(|entity| {
            let segments = entity
                .path()
                .namespace()
                .iter()
                .chain(std::iter::once(entity.path().name()));
            let name = segments
                .map(|segment| {
                    module_ident(segment)
                        .to_string()
                        .trim_start_matches("r#")
                        .to_owned()
                })
                .collect::<Vec<_>>()
                .join("_");
            let mut candidate = name.clone();
            let mut suffix = 2;
            while !field_names.insert(candidate.clone()) {
                candidate = format!("{name}_{suffix}");
                suffix += 1;
            }
            if candidate == name && entity.path().namespace().is_empty() {
                module_ident(entity.path().name())
            } else {
                quote::format_ident!("{}", candidate)
            }
        })
        .collect()
}
