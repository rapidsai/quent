// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of native in-memory event storage.

use proc_macro2::TokenStream;
use quent_schema::{Cardinality, Entity, Event};
use quote::quote;

use super::super::EventCode;

/// Native storage, insertion, and accessor implementation fragments.
pub(super) struct NativeEventStorageCode {
    /// A field holding an optional original event or a vector of original events.
    ///
    /// For example, `event_init: Option<Event<TaskEvent>>`.
    pub(super) storage_field_def: TokenStream,
    /// A match arm inserting the original event and checking cardinality.
    ///
    /// For example, `TaskEvent::Tick => self.event_tick.push(event)`.
    pub(super) conversion_match_arm: TokenStream,
    /// The native implementation returning borrowing handles.
    ///
    /// For example:
    /// ```text
    /// fn init(&self) -> Option<InitHandle<'_>> {
    ///     self.events().event_init.as_ref().map(|event| InitHandle { event })
    /// }
    /// ```
    pub(super) accessor_method_impl: TokenStream,
}

pub(super) fn generate_event(
    entity: &Entity,
    event: &Event,
    original: &TokenStream,
    handle: &syn::Ident,
    method: &syn::Ident,
    variant_pattern: &TokenStream,
    accessor_method_signature: &TokenStream,
) -> NativeEventStorageCode {
    let slot = quote::format_ident!("event_{}", method.to_string().trim_start_matches("r#"));
    let (storage_field_def, conversion_match_arm, accessor_method_impl) = if event.cardinality()
        == Cardinality::Once
    {
        let event_name = event.name().to_string();
        let accessor_expr = if entity.events().count() == 1 {
            quote! { #handle {
                event: self.events().#slot.as_ref().expect("non-empty entity must contain its sole event"),
            } }
        } else {
            quote! { self.events().#slot.as_ref().map(|event| #handle { event }) }
        };
        (
            quote! { #slot: ::core::option::Option<::quent_events::Event<#original>> },
            quote! { #variant_pattern => {
                ::quent_store::entity::insert_once(&mut self.#slot, event)
                    .map_err(|event| ::quent_store::Error::DuplicateOnceEvent {
                        entity_id: event.id,
                        event_name: #event_name,
                    })?;
            } },
            quote! { #accessor_method_signature { #accessor_expr } },
        )
    } else {
        (
            quote! { #slot: ::std::vec::Vec<::quent_events::Event<#original>> },
            quote! { #variant_pattern => { self.#slot.push(event); } },
            quote! { #accessor_method_signature {
                self.events().#slot.iter().map(|event| #handle { event })
            } },
        )
    };
    NativeEventStorageCode {
        storage_field_def,
        conversion_match_arm,
        accessor_method_impl,
    }
}

pub(super) fn generate_entity(
    entity: &Entity,
    native: &syn::Ident,
    event_storage_ident: &syn::Ident,
    access: &syn::Ident,
    marker: &TokenStream,
    events: &[EventCode],
) -> TokenStream {
    let storage = events
        .iter()
        .map(|event| &event.storage.native.storage_field_def);
    let arms = events
        .iter()
        .map(|event| &event.storage.native.conversion_match_arm);
    let implementations = events
        .iter()
        .map(|event| &event.storage.native.accessor_method_impl);
    let docs = format!("Owns events grouped by event type for `{}`.", entity.path());
    let storage_docs = format!("Event-type-scoped storage for `{}`.", entity.path());
    quote! {
        #[doc = #storage_docs]
        #[doc(hidden)]
        #[derive(Default)]
        pub struct #event_storage_ident {
            #(#storage,)*
        }
        impl ::quent_store::entity::EventStorage<#marker> for #event_storage_ident {
            fn push(&mut self, event: ::quent_events::Event<<#marker as ::quent_events::EntityMarker>::Payload>) -> ::core::result::Result<(), ::quent_store::Error> {
                match &event.data { #(#arms,)* }
                Ok(())
            }
        }
        #[doc = #docs]
        ///
        /// Conversion rejects duplicate once-events.
        pub type #native = ::quent_store::entity::native::NativeEntity<#marker, #event_storage_ident>;
        impl #access for #native { #(#implementations)* }
    }
}
