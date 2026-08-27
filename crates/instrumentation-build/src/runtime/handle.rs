// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of per-entity handles — the per-instance emit surface.

use convert_case::Case;
use proc_macro2::{Literal, TokenStream};
use quent_schema::{Cardinality, Entity};
use quote::quote;

use super::{event_ident, marker_ident};
use crate::common::{doc_attr_or, raw_ident, relative_root_type, to_case};
use crate::data_type::map_data_type;
use crate::{GenerateError, Options};

/// The maximum once-events an entity may declare: one bit per event in the
/// handle's `u64` once-flag word.
pub(crate) const MAX_ONCE_EVENTS: usize = u64::BITS as usize;

/// Generate entity-specific methods on the generic handle.
///
/// # Errors
///
/// Returns [`GenerateError::TooManyOnceEvents`] if the entity declares more
/// once-cardinality events than fit the once-flag word.
pub(super) fn entity_handle(entity: &Entity, opts: &Options) -> Result<TokenStream, GenerateError> {
    let event_ty = event_ident(entity);
    let marker_ty = marker_ident(entity);
    let handle_ty = relative_root_type("Handle", entity.path().namespace());

    let once_count = entity
        .events()
        .filter(|e| e.cardinality() == Cardinality::Once)
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

            let params = event
                .fields()
                .map(|f| {
                    let name = raw_ident(to_case(f.name(), Case::Snake));
                    let ty = map_data_type(f.ty(), 0, entity.path().namespace(), opts)?;
                    Ok::<_, GenerateError>(quote! { #name: #ty })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let field_names: Vec<TokenStream> = event
                .fields()
                .map(|f| {
                    let name = raw_ident(to_case(f.name(), Case::Snake));
                    quote! { #name }
                })
                .collect();
            let construct = if field_names.is_empty() {
                quote! { #event_ty::#variant }
            } else {
                quote! { #event_ty::#variant { #(#field_names),* } }
            };

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
