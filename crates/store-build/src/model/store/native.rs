// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of model-wide native entity stores.

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quent_events_build::common::{module_ident, relative_type_path, type_ident};
use quent_schema::Schema;
use quote::quote;

use crate::GenerateError;

/// Native model-store declaration, construction, and entity access fragments.
pub(super) struct NativeModelStoreCode {
    pub(super) store_type_def: TokenStream,
    pub(super) store_impl: TokenStream,
    pub(super) entity_store_impls: TokenStream,
}

/// Generates a native model store, rejecting conflicts with existing type names.
pub(super) fn generate(
    schema: &Schema,
    events: &syn::File,
) -> Result<NativeModelStoreCode, GenerateError> {
    let model = type_ident(schema.name());
    let name = quote::format_ident!("{}Store", model.to_string().trim_start_matches("r#"));
    if events
        .items
        .iter()
        .filter_map(crate::entities::namespaces::ident)
        .any(|ident| ident == &name)
    {
        return Err(GenerateError::EntityEventsNameConflict {
            name: name.to_string(),
            first: "existing generated type or namespace".into(),
            second: "model-wide entity store".into(),
        });
    }
    let markers = schema
        .entities()
        .map(|entity| relative_type_path(entity.path(), &[], ""))
        .collect::<Vec<_>>();
    let mut field_names = BTreeSet::new();
    let fields = schema
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
        .collect::<Vec<_>>();
    let implementations = markers.iter().zip(&fields).map(|(marker, field)| quote! {
        impl ::quent_store::entity::EntityStore<#marker> for #name {
            type Error = ::quent_store::Error;
            type Handle<'a> = &'a ::quent_store::entity::native::NativeEntity<#marker>;

            fn entities(&self) -> ::quent_store::Result<impl Iterator<Item = ::quent_store::Id<#marker>>> {
                ::quent_store::entity::EntityStore::entities(&self.#field)
            }

            fn entity(&self, id: impl Into<::quent_store::Id<#marker>>) -> ::quent_store::Result<Option<Self::Handle<'_>>> {
                ::quent_store::entity::EntityStore::entity(&self.#field, id)
            }
        }

        impl ::quent_store::entity::BorrowedEventSequenceStore<#marker> for #name {
            fn event_sequence(&self, id: ::quent_store::Id<#marker>) -> ::quent_store::Result<&::quent_store::entity::sequence::EventSequence<#marker>> {
                ::quent_store::entity::BorrowedEventSequenceStore::event_sequence(&self.#field, id)
            }
        }
    });
    let docs = format!("Stores every entity type in `{model}`.");
    Ok(NativeModelStoreCode {
        store_type_def: quote! {
            #[doc = #docs]
            pub struct #name {
                #(#fields: ::quent_store::entity::native::NativeStore<#markers>,)*
            }
        },
        store_impl: quote! {
            impl #name {
                /// Loads every entity type from the loader.
                ///
                /// # Errors
                ///
                /// Returns an error if loading or event indexing fails.
                pub fn try_from_loader<L>(_loader: &L) -> ::quent_store::Result<Self>
                where
                    L: ?Sized,
                    #(L: ::quent_store::event::EventLoader<#markers, Error = ::quent_store::Error>,)*
                {
                    #(let #fields = ::quent_store::entity::native::NativeStore::<#markers>::try_from_loader(_loader)?;)*
                    Ok(Self { #(#fields,)* })
                }

                /// Lists the IDs of one entity type.
                pub fn entities<E: ::quent_store::events::EntityMarker>(&self) -> ::quent_store::Result<impl Iterator<Item = ::quent_store::Id<E>>>
                where
                    Self: ::quent_store::entity::EntityStore<E, Error = ::quent_store::Error>,
                {
                    <Self as ::quent_store::entity::EntityStore<E>>::entities(self)
                }

                /// Returns a borrowed typed handle, or `None` if the entity is absent.
                pub fn entity<E: ::quent_store::events::EntityMarker>(&self, id: impl Into<::quent_store::Id<E>>) -> ::quent_store::Result<Option<<Self as ::quent_store::entity::EntityStore<E>>::Handle<'_>>>
                where
                    Self: ::quent_store::entity::EntityStore<E, Error = ::quent_store::Error>,
                {
                    <Self as ::quent_store::entity::EntityStore<E>>::entity(self, id)
                }
            }
        },
        entity_store_impls: quote! {
            #(#implementations)*

            impl<E: ::quent_store::events::EntityMarker> ::quent_store::entity::OwnedEventSequenceStore<E> for #name
            where
                Self: ::quent_store::entity::BorrowedEventSequenceStore<E, Error = ::quent_store::Error>,
                E::Payload: Clone,
            {
                fn event_sequence_owned(&self, id: ::quent_store::Id<E>) -> ::quent_store::Result<::quent_store::entity::sequence::EventSequence<E>> {
                    ::quent_store::entity::BorrowedEventSequenceStore::event_sequence(self, id).cloned()
                }
            }
        },
    })
}

/// Assembles a native model store from its declarations and implementations.
pub(super) fn generate_model(code: &NativeModelStoreCode) -> TokenStream {
    let NativeModelStoreCode {
        store_type_def,
        store_impl,
        entity_store_impls,
    } = code;
    quote! {
        #store_type_def
        #store_impl
        #entity_store_impls
    }
}
