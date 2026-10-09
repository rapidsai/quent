// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of dynamic-state FSM handles.

use convert_case::Case;
use proc_macro2::{Literal, TokenStream};
use quent_fsm::{Fsm, SEQUENCE_FIELD_NAME};
use quent_schema::{Entity, Event, Schema};
use quote::quote;

use super::super::{event_construct, event_fields, event_params};
use crate::common::{doc_attr_or, raw_ident, relative_root_type, to_case};
use crate::runtime::{event_ident, marker_ident, model_ident};
use crate::{GenerateError, Options};

pub(super) fn handle_type(schema: &Schema) -> TokenStream {
    let model = model_ident(schema);
    let docs = format!(
        "Dynamic-state handle to one FSM entity instance in the `{}` instrumentation model.",
        schema.name()
    );
    quote! {
        #[doc = #docs]
        pub struct DynamicFsmHandle<
            E: ::quent_instrumentation::InstrumentedEntity<Context = Context<#model>>,
        > {
            inner: ::quent_instrumentation::FsmHandleInner<E>,
            state: u8,
        }

        impl<E: ::quent_instrumentation::InstrumentedEntity<Context = Context<#model>>>
            DynamicFsmHandle<E>
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

pub(super) fn entity_impl(
    entity: &Entity,
    fsm: &Fsm,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let event_ty = event_ident(entity);
    let marker_ty = marker_ident(entity);
    let typestate_handle_ty = super::typestate::handle_type_path(entity);
    let handle_ty = relative_root_type("DynamicFsmHandle", entity.path().namespace());
    let mismatch_ty = relative_root_type("FsmStateMismatch", entity.path().namespace());
    let state_module = raw_ident(format!(
        "{}_state",
        to_case(entity.path().name(), Case::Snake)
    ));
    let state_ty = raw_ident(format!(
        "{}DynamicState",
        to_case(entity.path().name(), Case::Pascal)
    ));

    let variants = entity
        .events()
        .map(|event| raw_ident(to_case(event.name(), Case::Pascal)));
    let state_arms = entity.events().enumerate().map(|(index, event)| {
        let index = Literal::usize_unsuffixed(index + 1);
        let variant = raw_ident(to_case(event.name(), Case::Pascal));
        quote! { #index => #state_ty::#variant }
    });
    let state_name_arms = entity.events().enumerate().map(|(index, event)| {
        let index = Literal::usize_unsuffixed(index + 1);
        let name = event.name().to_string();
        quote! { #index => #name }
    });

    let entity_name = entity.path().to_string();
    let mut transition_methods = Vec::new();
    for event in entity.events() {
        let mut sources = Vec::new();
        if event.name() == fsm.initial_state() {
            sources.push(0usize);
        }
        for transition in fsm
            .transitions()
            .iter()
            .filter(|transition| transition.target() == event.name())
        {
            let source_index = entity
                .events()
                .position(|source| source.name() == transition.source())
                .expect("validated FSM transition source")
                + 1;
            if !sources.contains(&source_index) {
                sources.push(source_index);
            }
        }
        transition_methods.push(transition_method(
            entity,
            event,
            &event_ty,
            &entity_name,
            &sources,
            opts,
        )?);
    }

    let state_conversions = entity.events().enumerate().map(|(index, event)| {
        let marker = raw_ident(to_case(event.name(), Case::Pascal));
        let state = Literal::usize_unsuffixed(index + 1);
        let state_name = event.name().to_string();
        quote! {
            impl ::quent_instrumentation::FsmState<#marker_ty> for #state_module::#marker {
                const DYNAMIC_STATE_INDEX: u8 = #state;
                const NAME: &'static str = #state_name;
            }

            impl #typestate_handle_ty<#marker_ty, #state_module::#marker> {
                /// Converts this typestate handle into a dynamic-state FSM handle.
                pub fn into_dynamic(self) -> #handle_ty<#marker_ty> {
                    #handle_ty { inner: self.inner, state: #state }
                }
            }
        }
    });

    Ok(quote! {
        /// Dynamic state of this FSM.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum #state_ty {
            /// No transition has been emitted yet.
            New,
            #(#variants),*
        }

        impl ::quent_instrumentation::FsmState<#marker_ty> for () {
            const DYNAMIC_STATE_INDEX: u8 = 0;
            const NAME: &'static str = "new";
        }

        impl #typestate_handle_ty<#marker_ty> {
            /// Converts this typestate handle into a dynamic-state FSM handle.
            pub fn into_dynamic(self) -> #handle_ty<#marker_ty> {
                #handle_ty { inner: self.inner, state: 0 }
            }
        }

        #(#state_conversions)*

        impl #handle_ty<#marker_ty> {
            /// Returns the current dynamic state.
            pub fn state(&self) -> #state_ty {
                match self.state {
                    0 => #state_ty::New,
                    #(#state_arms,)*
                    _ => unreachable!("generated dynamic FSM state is valid"),
                }
            }

            fn state_name(&self) -> &'static str {
                match self.state {
                    0 => "new",
                    #(#state_name_arms,)*
                    _ => unreachable!("generated dynamic FSM state is valid"),
                }
            }

            /// Converts this dynamic-state handle into the requested typestate.
            ///
            /// # Errors
            ///
            /// Returns [`FsmStateMismatch`] with ownership of this handle when
            /// its current state does not match `S`.
            pub fn try_into<S>(
                self,
            ) -> ::core::result::Result<
                #typestate_handle_ty<#marker_ty, S>,
                #mismatch_ty<Self>,
            >
            where
                S: ::quent_instrumentation::FsmState<#marker_ty>,
            {
                if self.state != S::DYNAMIC_STATE_INDEX {
                    let state = self.state_name();
                    return ::core::result::Result::Err(#mismatch_ty::new(
                        self,
                        #entity_name,
                        state,
                        S::NAME,
                    ));
                }
                ::core::result::Result::Ok(#typestate_handle_ty {
                    inner: self.inner,
                    _state: ::core::marker::PhantomData,
                })
            }

            #(#transition_methods)*
        }
    })
}

fn transition_method(
    entity: &Entity,
    event: &Event,
    event_ty: &syn::Ident,
    entity_name: &str,
    sources: &[usize],
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let method = raw_ident(to_case(event.name(), Case::Snake));
    let variant = raw_ident(to_case(event.name(), Case::Pascal));
    let docs = doc_attr_or(
        event.annotations().docs(),
        &format!("Transition this FSM into `{}`.", event.name()),
    );
    let params = event_params(entity, event, opts, Some(SEQUENCE_FIELD_NAME))?;
    let sequence_placeholder = quote! { 0 };
    let fields = event_fields(event, Some((SEQUENCE_FIELD_NAME, &sequence_placeholder)));
    let construct = event_construct(event_ty, &variant, &fields);
    let marker_ty = marker_ident(entity);
    let state_module = raw_ident(format!(
        "{}_state",
        to_case(entity.path().name(), Case::Snake)
    ));
    let sources = sources.iter().map(|source| {
        if *source == 0 {
            quote! {
                <() as ::quent_instrumentation::FsmState<#marker_ty>>::DYNAMIC_STATE_INDEX
            }
        } else {
            let source = entity
                .events()
                .nth(*source - 1)
                .expect("validated FSM transition source index");
            let source_marker = raw_ident(to_case(source.name(), Case::Pascal));
            quote! {
                <#state_module::#source_marker as
                    ::quent_instrumentation::FsmState<#marker_ty>>::DYNAMIC_STATE_INDEX
            }
        }
    });
    let target_marker = raw_ident(to_case(event.name(), Case::Pascal));
    let target = quote! {
        <#state_module::#target_marker as
            ::quent_instrumentation::FsmState<#marker_ty>>::DYNAMIC_STATE_INDEX
    };
    let target_name = event.name().to_string();

    Ok(quote! {
        #docs
        ///
        /// # Errors
        ///
        /// Returns [`FsmTransitionError`] when an active observer's transition is
        /// not valid from the current dynamic state. No-op observers skip this check.
        pub fn #method(
            &mut self,
            #(#params),*
        ) -> ::core::result::Result<(), ::quent_instrumentation::FsmTransitionError> {
            if self.inner.checks_transitions() && ![#(#sources),*].contains(&self.state) {
                return ::core::result::Result::Err(
                    ::quent_instrumentation::FsmTransitionError::new(
                        #entity_name,
                        self.state_name(),
                        #target_name,
                    ),
                );
            }
            self.inner.transition_mut(#construct);
            self.state = #target;
            ::core::result::Result::Ok(())
        }
    })
}
