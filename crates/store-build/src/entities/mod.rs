// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of entity-specific event access.

use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quent_instrumentation_build::{
    Options, generated_entity_event_path, generated_entity_path, generated_module_ident,
    generated_type_ident,
};
use quent_schema::{Entity, Event, Identifier, Schema};
use quote::quote;

use crate::{GenerateError, StorageSet};

mod accessors;
mod handles;
mod namespaces;
mod storage;

#[cfg(test)]
mod tests;

use handles::EventHandleCode;
use namespaces::Module;
use storage::EventStorageCode;

/// Backend-independent handle and accessor declarations for one schema event.
struct EventCode {
    /// The borrowing handle type and its field accessors.
    ///
    /// For example, `pub struct InitHandle<'a> { event: &'a Event<TaskEvent> }`.
    handle_type_def: TokenStream,
    /// An accessor declaration for the entity-specific trait.
    ///
    /// For example, `fn init(&self) -> Option<InitHandle<'_>>;`.
    accessor_method_decl: TokenStream,
    /// Storage fragments grouped by backend.
    storage: EventStorageCode,
}

pub(super) fn generate(
    schema: &Schema,
    opts: &Options,
    events: &syn::File,
    storage: &StorageSet,
) -> Result<syn::ItemMod, GenerateError> {
    if events
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Mod(module) if module.ident == "entity_events"))
    {
        return Err(GenerateError::EntityEventsNameConflict {
            name: "entity_events".into(),
            first: "schema namespace".into(),
            second: "entity event access".into(),
        });
    }
    let root = Identifier::try_new("entity_events").expect("valid identifier");
    let mut tree = Module::default();
    for entity in schema.entities() {
        let (module, namespace) = namespaces::for_entity(&mut tree, entity, &root)?;
        let items = generate_entity(entity, &namespace, opts, storage, &mut module.type_names)?;
        module.items.extend(items.items);
    }
    tree.into_item(syn::parse_quote!(entity_events))
}

fn generate_entity(
    entity: &Entity,
    namespace: &[Identifier],
    opts: &Options,
    storage: &StorageSet,
    type_names: &mut BTreeMap<String, String>,
) -> Result<syn::File, GenerateError> {
    let parents = vec![quote!(super); namespace.len()];
    let marker = generated_entity_path(entity);
    let original = generated_entity_event_path(entity);
    let marker = quote! { #(#parents::)* #marker };
    let original = quote! { #(#parents::)* #original };
    let entity_name = generated_type_ident(entity.path().name())
        .to_string()
        .trim_start_matches("r#")
        .to_owned();
    let access = quote::format_ident!("{}Events", entity_name);
    let native = quote::format_ident!("Native{}Events", entity_name);
    let event_storage_ident = quote::format_ident!("{}EventStorage", entity_name);
    type_names.insert(access.to_string(), "entity access trait".into());
    if storage.native {
        type_names.insert(native.to_string(), "native entity storage".into());
        type_names.insert(
            event_storage_ident.to_string(),
            "native event storage".into(),
        );
    }
    let mut method_names = BTreeMap::from([
        ("id".to_owned(), "entity identity".to_owned()),
        ("type_name".to_owned(), "entity type name".to_owned()),
        ("properties".to_owned(), "entity properties".to_owned()),
        (
            "events".to_owned(),
            "native event storage accessor".to_owned(),
        ),
    ]);
    let mut code = Vec::new();
    for event in entity.events() {
        let handle = handles::ident(event);
        let method = generated_module_ident(event.name());
        let origin = format!("event {}::{}", entity.path(), event.name());
        for (names, name) in [
            (&mut *type_names, handle.to_string()),
            (&mut method_names, method.to_string()),
        ] {
            if let Some(first) = names.insert(name.clone(), origin.clone()) {
                return Err(GenerateError::EntityEventsNameConflict {
                    name,
                    first,
                    second: origin.clone(),
                });
            }
        }
        code.push(generate_event(entity, event, namespace, opts, &original)?);
    }
    let handles = code.iter().map(|event| &event.handle_type_def);
    let methods = code.iter().map(|event| &event.accessor_method_decl);
    let native_storage = storage.native.then(|| {
        storage::generate_entity(
            entity,
            &native,
            &event_storage_ident,
            &access,
            &marker,
            &code,
        )
    });
    let docs = format!("Provides event-type-scoped access for `{}`.", entity.path());
    syn::parse2(quote! {
        #(#handles)*
        #[doc = #docs]
        pub trait #access: ::quent_store::entity::EntityHandle<Entity = #marker> {
            #(#methods)*
        }
        #native_storage
    })
    .map_err(GenerateError::InvalidGeneratedCode)
}

fn generate_event(
    entity: &Entity,
    event: &Event,
    namespace: &[Identifier],
    opts: &Options,
    original: &TokenStream,
) -> Result<EventCode, GenerateError> {
    let handle = handles::ident(event);
    let EventHandleCode {
        handle_type_def,
        variant_pattern,
    } = handles::generate(entity, event, namespace, opts, original)?;
    let accessor = accessors::generate(entity, event, &handle);
    let storage = storage::generate_event(
        entity,
        event,
        original,
        &handle,
        &accessor.method_ident,
        &variant_pattern,
        &accessor.method_signature,
    );
    Ok(EventCode {
        handle_type_def,
        accessor_method_decl: accessor.method_decl,
        storage,
    })
}
