// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of per-entity handles — the per-instance emit surface.

use convert_case::Case;
use proc_macro2::{Literal, TokenStream};
use quent_schema::{Cardinality, Entity};
use quote::quote;

use super::{event_ident, handle_ident, marker_ident};
use crate::GenerateError;
use crate::common::{doc_attr_or, raw_ident, to_case};
use crate::data_type::map_data_type;

/// The maximum once-events an entity may declare: one bit per event in the
/// handle's `u64` once-flag word.
pub(crate) const MAX_ONCE_EVENTS: usize = u64::BITS as usize;

/// Generate the declaration of an {Entity}Handle and its impls.
///
/// # Errors
///
/// Returns [`GenerateError::TooManyOnceEvents`] if the entity declares more
/// once-cardinality events than fit the once-flag word.
pub(super) fn entity_handle(entity: &Entity) -> Result<TokenStream, GenerateError> {
    let entity_pascal = to_case(entity.path().name(), Case::Pascal);
    let event_ty = event_ident(entity);
    let handle_ty = handle_ident(entity);
    let marker_ty = marker_ident(entity);

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
    let methods: Vec<TokenStream> = entity
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

            let params: Vec<TokenStream> = event
                .fields()
                .map(|f| {
                    let name = raw_ident(to_case(f.name(), Case::Snake));
                    let ty = map_data_type(f.ty(), 0);
                    quote! { #name: #ty }
                })
                .collect();
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

            match event.cardinality() {
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
            }
        })
        .collect();

    let handle_doc = format!("Handle to one `{entity_pascal}` entity instance.");
    Ok(quote! {
        #[doc = #handle_doc]
        pub struct #handle_ty {
            inner: ::quent_instrumentation::Handle<#event_ty>,
        }

        impl #handle_ty {
            /// Id of the entity instance this handle emits for.
            pub fn uuid(&self) -> ::quent_instrumentation::Uuid {
                self.inner.id()
            }

            /// A typed reference to this instance, carrying no data.
            pub fn as_entity_ref(&self) -> ::quent_instrumentation::EntityRef<#marker_ty> {
                ::quent_instrumentation::EntityRef::new(self.uuid(), ())
            }

            /// A typed reference to this instance, carrying `data`.
            pub fn as_entity_ref_with<T>(&self, data: T) -> ::quent_instrumentation::EntityRef<#marker_ty, T> {
                ::quent_instrumentation::EntityRef::new(self.uuid(), data)
            }

            /// A reference to this instance for a field not restricted to a
            /// target entity type, carrying no data.
            pub fn as_any_entity_ref(&self) -> ::quent_instrumentation::EntityRef<::quent_instrumentation::AnyEntity> {
                ::quent_instrumentation::EntityRef::new(self.uuid(), ())
            }

            /// A reference to this instance for a field not restricted to a
            /// target entity type, carrying `data`.
            pub fn as_any_entity_ref_with<T>(&self, data: T) -> ::quent_instrumentation::EntityRef<::quent_instrumentation::AnyEntity, T> {
                ::quent_instrumentation::EntityRef::new(self.uuid(), data)
            }

            #(#methods)*
        }
    })
}
