// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Assembly of generated schema namespaces.

use std::collections::BTreeMap;

use quent_instrumentation_build::generated_module_ident;
use quent_schema::{Entity, Identifier};
use quote::quote;

use crate::GenerateError;

/// An output module containing generated Rust items and nested modules.
#[derive(Default)]
pub(super) struct Module {
    /// Child modules keyed by Rust name, with their identifier and schema
    /// origin.
    children: BTreeMap<String, (syn::Ident, String, Module)>,
    /// Generated type names mapped to their origins for conflict checking.
    pub(super) type_names: BTreeMap<String, String>,
    /// Rust items emitted directly in this module.
    ///
    /// For example, `pub struct InitHandle<'a> { event: &'a Event<TaskEvent> }`.
    pub(super) items: Vec<syn::Item>,
}

impl Module {
    fn child(&mut self, ident: syn::Ident, origin: String) -> Result<&mut Self, GenerateError> {
        let name = ident.to_string();
        let (_, existing, child) = self
            .children
            .entry(name.clone())
            .or_insert_with(|| (ident, origin.clone(), Self::default()));
        if *existing != origin {
            return Err(GenerateError::EntityEventsNameConflict {
                name,
                first: existing.clone(),
                second: origin,
            });
        }
        Ok(child)
    }

    pub(super) fn into_item(self, ident: syn::Ident) -> Result<syn::ItemMod, GenerateError> {
        let mut items = self.items;
        for (ident, _, child) in self.children.into_values() {
            items.push(syn::Item::Mod(child.into_item(ident)?));
        }
        syn::parse2(quote! { pub mod #ident { #(#items)* } })
            .map_err(GenerateError::InvalidGeneratedCode)
    }
}

pub(super) fn for_entity<'a>(
    tree: &'a mut Module,
    entity: &Entity,
    root: &Identifier,
) -> Result<(&'a mut Module, Vec<Identifier>), GenerateError> {
    let mut namespace = vec![root.clone()];
    let mut module = tree;
    for segment in entity.path().namespace() {
        namespace.push(segment.clone());
        let path = namespace[1..]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("::");
        module = module.child(generated_module_ident(segment), format!("namespace {path}"))?;
    }
    namespace.push(entity.path().name().clone());
    module = module.child(
        generated_module_ident(entity.path().name()),
        format!("entity {}", entity.path()),
    )?;
    Ok((module, namespace))
}
