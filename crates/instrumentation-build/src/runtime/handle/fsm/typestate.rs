// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of consuming typestate FSM handles.

use std::collections::HashSet;

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_fsm::{Fsm, SEQUENCE_FIELD_NAME};
use quent_schema::{Entity, Event, Schema};
use quote::quote;

use super::super::{event_construct, event_fields, event_params};
use crate::{
    GenerateError,
    common::{raw_ident, relative_root_type, to_case},
    runtime::{event_ident, marker_ident, model_ident},
};

pub(super) fn handle_type(schema: &Schema) -> TokenStream {
    let model = model_ident(schema);
    let docs = format!(
        "Typestate handle to one FSM entity instance in the `{}` instrumentation model.",
        schema.name()
    );
    quote! {
        #[doc = #docs]
        pub struct FsmHandle<
            E: ::quent_instrumentation::InstrumentedEntity<Context = Context<#model>>,
            S = (),
        > {
            inner: ::quent_instrumentation::FsmHandleInner<E>,
            _state: ::core::marker::PhantomData<S>,
        }

        impl<E: ::quent_instrumentation::InstrumentedEntity<Context = Context<#model>>>
            ::core::convert::From<::quent_instrumentation::HandleInner<E>> for FsmHandle<E>
        {
            fn from(inner: ::quent_instrumentation::HandleInner<E>) -> Self {
                Self {
                    inner: inner.into(),
                    _state: ::core::marker::PhantomData,
                }
            }
        }

        impl<
            E: ::quent_instrumentation::InstrumentedEntity<Context = Context<#model>>,
            S,
        > FsmHandle<E, S>
        {
            /// Returns the entity instance ID.
            pub fn id(&self) -> ::quent_instrumentation::Uuid {
                self.inner.id()
            }

            /// Returns a typed reference to this instance carrying no data.
            pub fn as_entity_ref(&self) -> ::quent_instrumentation::EntityRef<E> {
                self.inner.as_entity_ref()
            }

            /// Returns a typed reference to this instance carrying `data`.
            pub fn as_entity_ref_with<T>(&self, data: T) -> ::quent_instrumentation::EntityRef<E, T> {
                self.inner.as_entity_ref_with(data)
            }

            /// Returns an untyped reference to this instance carrying no data.
            pub fn as_any_entity_ref(&self) -> ::quent_instrumentation::EntityRef<::quent_instrumentation::AnyEntity> {
                self.inner.as_any_entity_ref()
            }

            /// Returns an untyped reference to this instance carrying `data`.
            pub fn as_any_entity_ref_with<T>(
                &self,
                data: T,
            ) -> ::quent_instrumentation::EntityRef<::quent_instrumentation::AnyEntity, T> {
                self.inner.as_any_entity_ref_with(data)
            }
        }
    }
}

pub(super) fn handle_type_path(entity: &Entity) -> TokenStream {
    relative_root_type("FsmHandle", entity.path().namespace())
}

pub(super) fn entity_impl(entity: &Entity, fsm: &Fsm) -> Result<TokenStream, GenerateError> {
    let event_ty = event_ident(entity);
    let marker_ty = marker_ident(entity);
    let handle_ty = handle_type_path(entity);
    let state_module = raw_ident(format!(
        "{}_state",
        to_case(entity.path().name(), Case::Snake)
    ));
    let state_module_doc = format!("Typestate markers for the `{}` FSM.", entity.path());
    let markers = entity.events().map(|event| {
        let marker = raw_ident(to_case(event.name(), Case::Pascal));
        let marker_doc = format!(
            "Handle state after the FSM transitions into `{}`.",
            event.name()
        );
        quote! {
            #[doc = #marker_doc]
            pub enum #marker {}
        }
    });

    let initial_event = entity
        .events()
        .find(|event| event.name() == fsm.initial_state())
        .expect("validated FSM initial state has an event");
    let initial_method = transition_method(
        entity,
        initial_event,
        &event_ty,
        &marker_ty,
        &handle_ty,
        &state_module,
    )?;

    let mut emitted_edges = HashSet::new();
    let mut methods_by_source = Vec::new();
    for source_event in entity.events() {
        let methods = fsm
            .transitions()
            .iter()
            .filter(|transition| transition.source() == source_event.name())
            .filter(|transition| {
                emitted_edges.insert((transition.source().clone(), transition.target().clone()))
            })
            .map(|transition| {
                let target_event = entity
                    .events()
                    .find(|event| event.name() == transition.target())
                    .expect("validated FSM transition target has an event");
                transition_method(
                    entity,
                    target_event,
                    &event_ty,
                    &marker_ty,
                    &handle_ty,
                    &state_module,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        if methods.is_empty() {
            continue;
        }
        let source_marker = raw_ident(to_case(source_event.name(), Case::Pascal));
        methods_by_source.push(quote! {
            impl #handle_ty<#marker_ty, #state_module::#source_marker> {
                #(#methods)*
            }
        });
    }

    Ok(quote! {
        #[doc = #state_module_doc]
        pub mod #state_module {
            #(#markers)*
        }

        impl #handle_ty<#marker_ty> {
            #initial_method
        }

        #(#methods_by_source)*
    })
}

fn transition_method(
    entity: &Entity,
    event: &Event,
    event_ty: &syn::Ident,
    entity_ty: &syn::Ident,
    handle_ty: &TokenStream,
    state_module: &syn::Ident,
) -> Result<TokenStream, GenerateError> {
    let method = raw_ident(to_case(event.name(), Case::Snake));
    let method_at = raw_ident(format!("{}_at", to_case(event.name(), Case::Snake)));
    let variant = raw_ident(to_case(event.name(), Case::Pascal));
    let target_marker = raw_ident(to_case(event.name(), Case::Pascal));
    let (docs, at_docs) = super::transition_docs(event);
    let params = event_params(entity, event, Some(SEQUENCE_FIELD_NAME))?;
    let sequence_placeholder = quote! { 0 };
    let fields = event_fields(event, Some((SEQUENCE_FIELD_NAME, &sequence_placeholder)));
    let construct = event_construct(event_ty, &variant, &fields);

    Ok(quote! {
        #docs
        pub fn #method(self, #(#params),*) -> #handle_ty<#entity_ty, #state_module::#target_marker> {
            #handle_ty {
                inner: self.inner.transition(#construct),
                _state: ::core::marker::PhantomData,
            }
        }

        #at_docs
        pub fn #method_at(
            self,
            timestamp_ns: u64,
            #(#params),*
        ) -> #handle_ty<#entity_ty, #state_module::#target_marker> {
            #handle_ty {
                inner: self.inner.transition_at(timestamp_ns, #construct),
                _state: ::core::marker::PhantomData,
            }
        }
    })
}
