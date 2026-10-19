// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Placement of generated types in schema namespaces.

use std::collections::BTreeMap;

use quent_events_build::common::module_ident;
use quent_schema::Entity;

use crate::GenerateError;

/// Finds the schema namespace in which an entity's types belong.
///
/// Rejects schema namespaces that normalize to the same Rust name.
pub(super) fn for_entity<'a>(
    events: &'a mut syn::File,
    entity: &Entity,
    names: &mut BTreeMap<String, String>,
) -> Result<&'a mut Vec<syn::Item>, GenerateError> {
    let mut items = &mut events.items;
    let mut rust_path = Vec::new();
    let mut schema_path = Vec::new();
    for segment in entity.path().namespace() {
        let ident = module_ident(segment);
        rust_path.push(ident.to_string());
        schema_path.push(segment.to_string());
        let name = rust_path.join("::");
        let origin = schema_path.join("::");
        if let Some(first) = names.insert(name.clone(), origin.clone())
            && first != origin
        {
            return Err(GenerateError::EntityEventsNameConflict {
                name,
                first,
                second: origin,
            });
        }
        items = items
            .iter_mut()
            .find_map(|item| match item {
                syn::Item::Mod(module) if module.ident == ident => {
                    module.content.as_mut().map(|(_, items)| items)
                }
                _ => None,
            })
            .expect("event generation emits schema namespaces");
    }
    Ok(items)
}

/// Adds entity types to a namespace, rejecting conflicts with existing names.
pub(super) fn extend(
    items: &mut Vec<syn::Item>,
    generated: syn::File,
    entity: &Entity,
) -> Result<(), GenerateError> {
    for item in generated.items {
        if let Some(name) = ident(&item)
            && items
                .iter()
                .filter_map(ident)
                .any(|existing| existing == name)
        {
            return Err(GenerateError::EntityEventsNameConflict {
                name: name.to_string(),
                first: "existing generated type or namespace".into(),
                second: format!("entity access for {}", entity.path()),
            });
        }
        items.push(item);
    }
    Ok(())
}

/// Returns the name of a struct, enum, type alias, trait, or module, or `None`
/// for other items.
pub(crate) fn ident(item: &syn::Item) -> Option<&syn::Ident> {
    match item {
        syn::Item::Struct(item) => Some(&item.ident),
        syn::Item::Enum(item) => Some(&item.ident),
        syn::Item::Type(item) => Some(&item.ident),
        syn::Item::Trait(item) => Some(&item.ident),
        syn::Item::Mod(item) => Some(&item.ident),
        _ => None,
    }
}
