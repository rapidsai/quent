// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of native in-memory entity stores.

use proc_macro2::TokenStream;
use quent_schema::{Cardinality, Entity, Event};
use quote::quote;

use crate::entities::EventCode;

/// Native index, insertion, and accessor implementation fragments.
pub(super) struct NativeEventStoreCode {
    /// A field holding an optional position or a vector of positions.
    pub(super) index_field_def: TokenStream,
    /// A match arm recording the position and checking cardinality.
    pub(super) conversion_match_arm: TokenStream,
    /// The native implementation returning borrowing handles.
    pub(super) accessor_method_impl: TokenStream,
}

/// Generates code to index and borrow one event type's occurrences in native
/// storage.
pub(super) fn generate_event(
    entity: &Entity,
    event: &Event,
    handle: &TokenStream,
    method: &syn::Ident,
    variant_pattern: &TokenStream,
    accessor_method_signature: &TokenStream,
) -> NativeEventStoreCode {
    let slot = quote::format_ident!("event_{}", method.to_string().trim_start_matches("r#"));
    // TODO(johanpel): Consider u32 positions with u32::MAX for absent Once
    // events, and block-based delta encoding for large Multi groups.
    if event.cardinality() == Cardinality::Once {
        let event_name = event.name().to_string();
        let accessor_expr = if entity.events().count() == 1 {
            quote! { #handle {
                event: &self.event_sequence().events()[self.event_index().#slot.expect("non-empty entity must contain its sole event")],
            } }
        } else {
            quote! { self.event_index().#slot.map(|position| #handle {
                event: &self.event_sequence().events()[position],
            }) }
        };
        NativeEventStoreCode {
            index_field_def: quote! { #slot: ::core::option::Option<usize> },
            conversion_match_arm: quote! { #variant_pattern => {
                ::quent_store::entity::native::insert_once(&mut self.#slot, position)
                    .map_err(|_| ::quent_store::Error::DuplicateOnceEvent {
                        event_name: #event_name,
                    })?;
            } },
            accessor_method_impl: quote! { #accessor_method_signature { #accessor_expr } },
        }
    } else {
        NativeEventStoreCode {
            index_field_def: quote! { #slot: ::std::vec::Vec<usize> },
            conversion_match_arm: quote! { #variant_pattern => { self.#slot.push(position); } },
            accessor_method_impl: quote! { #accessor_method_signature {
                self.event_index().#slot.iter().map(|&position| #handle {
                    event: &self.event_sequence().events()[position],
                })
            } },
        }
    }
}

/// Generates a native entity index and event access implementations.
pub(super) fn generate_entity(
    entity: &Entity,
    event_index_ident: &syn::Ident,
    access: &syn::Ident,
    marker: &TokenStream,
    events: &[EventCode],
) -> TokenStream {
    let index_fields = events
        .iter()
        .map(|event| &event.store.native.index_field_def);
    let arms = events
        .iter()
        .map(|event| &event.store.native.conversion_match_arm);
    let implementations = events
        .iter()
        .map(|event| &event.store.native.accessor_method_impl)
        .collect::<Vec<_>>();
    let index_docs = format!("Event-type indices for `{}`.", entity.path());
    quote! {
        #[doc = #index_docs]
        #[doc(hidden)]
        #[derive(Default)]
        pub struct #event_index_ident {
            #(#index_fields,)*
        }
        impl ::quent_store::entity::native::EventIndex<#marker> for #event_index_ident {
            fn record(&mut self, payload: &<#marker as ::quent_store::events::EntityMarker>::Payload, position: usize) -> ::core::result::Result<(), ::quent_store::Error> {
                match payload { #(#arms,)* }
                Ok(())
            }
        }
        impl ::quent_store::entity::native::NativeEntityMarker for #marker {
            type Index = #event_index_ident;
        }
        impl #access for ::quent_store::entity::native::NativeEntity<#marker> {
            #(#implementations)*
        }
        impl #access for &::quent_store::entity::native::NativeEntity<#marker> {
            #(#implementations)*
        }

    }
}
