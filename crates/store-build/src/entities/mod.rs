// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of entity-specific event access.

use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quent_events_build::{
    Options,
    common::{module_ident, relative_type_path, type_ident},
};
use quent_schema::{Entity, Event, Identifier, Schema};
use quote::quote;

use crate::{EntityStoreImplementations, GenerateError, entities::store::EventStoreCode};

mod accessors;
mod handles;
mod namespaces;
mod store;

#[cfg(test)]
mod tests;

/// Backend-independent handle and accessor declarations for one schema event.
struct EventCode {
    /// The borrowing handle type and its field accessors.
    ///
    /// For example, `pub struct InitHandle<'a> { event: &'a Event<TaskEvent>
    /// }`.
    handle_type_def: TokenStream,
    /// An accessor declaration for the entity-specific trait.
    ///
    /// For example, `fn init(&self) -> Option<InitHandle<'_>>;`.
    accessor_method_decl: TokenStream,
    /// Store fragments grouped by backend.
    store: EventStoreCode,
}

/// Generates event handles, access traits, and indices in their schema
/// namespaces.
pub(super) fn generate(
    schema: &Schema,
    opts: &Options,
    events: &mut syn::File,
    stores: &EntityStoreImplementations,
) -> Result<(), GenerateError> {
    let mut names = BTreeMap::new();
    for entity in schema.entities() {
        let items = namespaces::for_entity(events, entity, &mut names)?;
        let generated = generate_entity(entity, opts, stores)?;
        namespaces::extend(items, generated, entity)?;
    }
    Ok(())
}

/// Generates event handles, an access trait, and indices for one entity.
fn generate_entity(
    entity: &Entity,
    opts: &Options,
    stores: &EntityStoreImplementations,
) -> Result<syn::File, GenerateError> {
    let mut namespace = entity.path().namespace().to_vec();
    let module = module_ident(entity.path().name());
    namespace.push(entity.path().name().clone());
    let parents = vec![quote!(super); namespace.len()];
    let marker = relative_type_path(entity.path(), &[], "");
    let original = relative_type_path(entity.path(), &[], "Event");
    let marker = quote! { #(#parents::)* #marker };
    let original = quote! { #(#parents::)* #original };
    let entity_name = type_ident(entity.path().name())
        .to_string()
        .trim_start_matches("r#")
        .to_owned();
    let access = quote::format_ident!("{}Events", entity_name);
    let event_index_ident = quote::format_ident!("{}EventIndex", entity_name);
    let mut type_names = BTreeMap::new();
    let mut method_names = BTreeMap::from([
        ("id".to_owned(), "entity identity".to_owned()),
        (
            "earliest_timestamp".to_owned(),
            "entity earliest timestamp".to_owned(),
        ),
        (
            "latest_timestamp".to_owned(),
            "entity latest timestamp".to_owned(),
        ),
        ("num_events".to_owned(), "entity event count".to_owned()),
        ("type_name".to_owned(), "entity type name".to_owned()),
        (
            "event_sequence".to_owned(),
            "raw event sequence accessor".to_owned(),
        ),
        (
            "event_index".to_owned(),
            "native event index accessor".to_owned(),
        ),
    ]);
    let mut code = Vec::new();
    for event in entity.events() {
        let handle = handles::ident(event);
        let method = module_ident(event.name());
        let origin = format!("event {}::{}", entity.path(), event.name());
        for (names, name) in [
            (&mut type_names, handle.to_string()),
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
        code.push(generate_event(entity, event, &namespace, opts, &original)?);
    }
    let handles = code.iter().map(|event| &event.handle_type_def);
    let methods = code.iter().map(|event| &event.accessor_method_decl);
    let native_store = stores
        .native
        .then(|| store::generate_entity(entity, &event_index_ident, &access, &marker, &code));
    let docs = format!("Provides event-type-scoped access for `{}`.", entity.path());
    syn::parse2(quote! {
        pub mod #module {
            #(#handles)*
            #[doc = #docs]
            pub trait #access: ::quent_store::entity::EntityHandle<Entity = #marker> {
                #(#methods)*
            }
            #native_store
        }
    })
    .map_err(GenerateError::InvalidGeneratedCode)
}

/// Generates a borrowing handle, an accessor declaration, and store code for
/// one event.
fn generate_event(
    entity: &Entity,
    event: &Event,
    namespace: &[Identifier],
    opts: &Options,
    original: &TokenStream,
) -> Result<EventCode, GenerateError> {
    let handle = handles::ident(event);
    let handle_type_def = handles::generate(entity, event, namespace, opts, original)?;
    let variant_pattern = handles::variant_pattern(event, original);
    let handle = quote! { #handle };
    let accessor = accessors::generate(entity, event, &handle);
    let store = store::generate_event(
        entity,
        event,
        &handle,
        &accessor.method_ident,
        &variant_pattern,
        &accessor.method_signature,
    );
    Ok(EventCode {
        handle_type_def,
        accessor_method_decl: accessor.method_decl,
        store,
    })
}
