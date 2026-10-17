// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of per-entity instrumentation handles.

use convert_case::Case;
use proc_macro2::{Literal, TokenStream};
use quent_schema::{Cardinality, Entity, Event, Schema};
use quote::quote;

use super::{event_ident, marker_ident};
use crate::{
    GenerateError,
    common::{doc_attr_or, raw_ident, relative_root_type, to_case},
};
use quent_events_build::data_type::map_data_type;

mod fsm;

pub(crate) use fsm::MAX_FSM_STATES;

/// The maximum once-events an ordinary entity may declare: one bit per event
/// in the handle's `u64` once-flag word.
pub(crate) const MAX_ONCE_EVENTS: usize = u64::BITS as usize;

pub(super) struct GeneratedHandle {
    pub(super) tokens: TokenStream,
    pub(super) associated_type: TokenStream,
}

pub(super) fn fsm_handle_type(schema: &Schema) -> TokenStream {
    fsm::handle_type(schema)
}

/// Generates entity-specific methods on the generic handle.
pub(super) fn entity_handle(entity: &Entity) -> Result<GeneratedHandle, GenerateError> {
    if let Some(handle) = fsm::entity_handle(entity)? {
        return Ok(handle);
    }
    let handle_ty = relative_root_type("Handle", entity.path().namespace());
    Ok(GeneratedHandle {
        tokens: ordinary_handle(entity)?,
        associated_type: quote! { #handle_ty<Self> },
    })
}

fn ordinary_handle(entity: &Entity) -> Result<TokenStream, GenerateError> {
    let event_ty = event_ident(entity);
    let marker_ty = marker_ident(entity);
    let handle_ty = relative_root_type("Handle", entity.path().namespace());

    let once_count = entity
        .events()
        .filter(|event| event.cardinality() == Cardinality::Once)
        .count();
    if once_count > MAX_ONCE_EVENTS {
        return Err(GenerateError::TooManyOnceEvents {
            entity: entity.path().clone(),
            count: once_count,
        });
    }

    // Once-events claim successive bits of the handle's flag word, in
    // declaration order; multi-events route straight through `emit`.
    let mut once_bit = 0u32;
    let methods = entity
        .events()
        .map(|event| {
            let method = raw_ident(to_case(event.name(), Case::Snake));
            let variant = raw_ident(to_case(event.name(), Case::Pascal));
            let fallback = match event.cardinality() {
                Cardinality::Once => format!(
                    "Emit the once-cardinality `{}` event for this instance.",
                    event.name()
                ),
                Cardinality::Multi => {
                    format!("Emit a `{}` event for this instance.", event.name())
                }
            };
            let docs = doc_attr_or(event.annotations().docs(), &fallback);

            let params = event_params(entity, event, None)?;
            let fields = event_fields(event, None);
            let construct = event_construct(&event_ty, &variant, &fields);

            Ok(match event.cardinality() {
                Cardinality::Once => {
                    let bit = Literal::u32_unsuffixed(once_bit);
                    once_bit += 1;
                    let event_name = event.name().to_string();
                    let emitted_method =
                        raw_ident(format!("{}_emitted", to_case(event.name(), Case::Snake)));
                    let emitted_doc = format!(
                        "Whether the once-cardinality `{}` event has already been emitted \
                         for this instance.",
                        event.name()
                    );
                    quote! {
                        #docs
                        pub fn #method(
                            &mut self,
                            #(#params),*
                        ) -> ::core::result::Result<(), ::quent_instrumentation::HandleError> {
                            self.inner.emit_once::<#bit>(#event_name, #construct)
                        }

                        #[doc = #emitted_doc]
                        pub fn #emitted_method(&self) -> bool {
                            self.inner.is_emitted::<#bit>()
                        }
                    }
                }
                Cardinality::Multi => quote! {
                    #docs
                    pub fn #method(
                        &self,
                        #(#params),*
                    ) -> ::core::result::Result<(), ::quent_instrumentation::HandleError> {
                        self.inner.emit(#construct);
                        ::core::result::Result::Ok(())
                    }
                },
            })
        })
        .collect::<Result<Vec<_>, GenerateError>>()?;

    Ok(quote! {
        impl #handle_ty<#marker_ty> {
            #(#methods)*
        }
    })
}

fn event_params(
    entity: &Entity,
    event: &Event,
    omitted_field: Option<&str>,
) -> Result<Vec<TokenStream>, GenerateError> {
    event
        .fields()
        .filter(|field| omitted_field.is_none_or(|omitted| field.name() != omitted))
        .map(|field| {
            let name = raw_ident(to_case(field.name(), Case::Snake));
            let ty = map_data_type(
                field.ty(),
                0,
                entity.path().namespace(),
                &crate::event_runtime(),
            )?;
            Ok(quote! { #name: #ty })
        })
        .collect()
}

fn event_fields(event: &Event, replaced_field: Option<(&str, &TokenStream)>) -> Vec<TokenStream> {
    event
        .fields()
        .map(|field| {
            let name = raw_ident(to_case(field.name(), Case::Snake));
            match replaced_field.filter(|(replaced, _)| field.name() == *replaced) {
                Some((_, replacement)) => quote! { #name: #replacement },
                None => quote! { #name },
            }
        })
        .collect()
}

fn event_construct(
    event_ty: &syn::Ident,
    variant: &syn::Ident,
    fields: &[TokenStream],
) -> TokenStream {
    if fields.is_empty() {
        quote! { #event_ty::#variant }
    } else {
        quote! { #event_ty::#variant { #(#fields),* } }
    }
}
