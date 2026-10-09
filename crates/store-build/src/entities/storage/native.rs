// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of native in-memory event storage.

use proc_macro2::TokenStream;
use quent_schema::{Cardinality, Entity, Event};
use quote::quote;

use super::super::EventCode;

/// Native in-memory storage, conversion, and accessor implementation fragments.
pub(super) struct NativeEventStorageCode {
    /// A native storage field holding an optional event or a vector of events.
    ///
    /// For example, `event_init: Option<Event<InitPayload>>`.
    pub(super) storage_field_def: TokenStream,
    /// A match arm moving an enum variant into native storage and checking
    /// cardinality.
    ///
    /// For example, a multi-event arm:
    /// ```text
    /// TaskEvent::Tick => {
    ///     self.event_tick.push(Event::new(id, ts, TickPayload {}));
    /// }
    /// ```
    pub(super) conversion_match_arm: TokenStream,
    /// The native storage implementation of the trait accessor.
    ///
    /// For example:
    /// ```text
    /// fn init(&self) -> Option<&Event<InitPayload>> {
    ///     self.event_storage().event_init.as_ref()
    /// }
    /// ```
    pub(super) accessor_method_impl: TokenStream,
}

pub(super) fn generate_event(
    entity: &Entity,
    event: &Event,
    payload: &syn::Ident,
    method: &syn::Ident,
    variant_pattern: &TokenStream,
    event_constructor_expr: &TokenStream,
    accessor_method_signature: &TokenStream,
) -> NativeEventStorageCode {
    let slot = quote::format_ident!("event_{}", method.to_string().trim_start_matches("r#"));
    let (storage_field_def, conversion_match_arm, accessor_method_impl) = if event.cardinality()
        == Cardinality::Once
    {
        let event_name = event.name().to_string();
        let accessor_expr = if entity.events().count() == 1 {
            quote! { self.event_storage().#slot.as_ref().expect("non-empty entity must contain its sole event") }
        } else {
            quote! { self.event_storage().#slot.as_ref() }
        };
        (
            quote! { #slot: ::core::option::Option<::quent_events::Event<#payload>> },
            quote! { #variant_pattern => {
                ::quent_store::entity::insert_once(&mut self.#slot, #event_constructor_expr)
                    .map_err(|event| ::quent_store::entity::DuplicateOnceEvent {
                        entity_id: event.id,
                        event_name: #event_name,
                    })?;
            } },
            quote! { #accessor_method_signature { #accessor_expr } },
        )
    } else {
        (
            quote! { #slot: ::std::vec::Vec<::quent_events::Event<#payload>> },
            quote! { #variant_pattern => { self.#slot.push(#event_constructor_expr); } },
            quote! { #accessor_method_signature { self.event_storage().#slot.iter() } },
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
        #[derive(Default)]
        pub struct #event_storage_ident {
            #(#storage,)*
        }
        impl ::quent_store::entity::grouped::EventStorage<#marker> for #event_storage_ident {
            fn push(&mut self, event: ::quent_events::Event<<#marker as ::quent_events::EntityMarker>::Payload>) -> ::core::result::Result<(), ::quent_store::entity::DuplicateOnceEvent> {
                let ::quent_events::Event { id: event_id, timestamp: event_timestamp, data } = event;
                match data { #(#arms,)* }
                Ok(())
            }
        }
        #[doc = #docs]
        ///
        /// Conversion consumes the sequence without cloning payloads and rejects duplicate once-events.
        pub type #native = ::quent_store::entity::grouped::NativeEntity<#marker, #event_storage_ident>;
        impl #access for #native { #(#implementations)* }
    }
}
