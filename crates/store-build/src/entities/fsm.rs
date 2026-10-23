// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of borrowing FSM transition handles.

use proc_macro2::TokenStream;
use quent_events_build::{Options, common::type_ident};
use quent_schema::Entity;
use quote::quote;

use super::{handles, store};

pub(super) struct FsmCode {
    pub(super) declarations: TokenStream,
    pub(super) store: store::FsmStoreCode,
}

pub(super) fn ident(entity: &Entity) -> syn::Ident {
    let entity = type_ident(entity.path().name());
    quote::format_ident!(
        "{}TransitionHandle",
        entity.to_string().trim_start_matches("r#")
    )
}

pub(super) fn generate(
    entity: &Entity,
    fsm: &quent_fsm::Fsm,
    opts: &Options,
    marker: &TokenStream,
    original: &TokenStream,
) -> FsmCode {
    let transition = ident(entity);
    let mut variants = Vec::new();
    let mut timestamp_arms = Vec::new();
    let mut name_arms = Vec::new();
    let mut sequence_arms = Vec::new();
    let mut final_arms = Vec::new();
    let mut event_impls = Vec::new();
    for event in entity.events() {
        let variant = type_ident(event.name());
        let handle = handles::ident(event);
        let name = event.name().to_string();
        let is_final = fsm.is_final_state(event.name());
        variants.push(quote! { #variant(#handle<'a>) });
        timestamp_arms
            .push(quote! { Self::#variant(handle) => ::quent_store::Timestamp::timestamp(handle) });
        name_arms.push(
            quote! { Self::#variant(handle) => ::quent_store::fsm::Transition::name(handle) },
        );
        sequence_arms.push(
            quote! { Self::#variant(handle) => ::quent_store::fsm::Sequence::sequence(handle) },
        );
        final_arms.push(
            quote! { Self::#variant(handle) => ::quent_store::fsm::Transition::is_final(handle) },
        );
        event_impls.push(quote! {
            impl ::quent_store::Timestamp for #handle<'_> {
                fn timestamp(&self) -> ::quent_store::TimeUnixNanoSec {
                    self.event.timestamp
                }
            }
            impl ::quent_store::fsm::Sequence for #handle<'_> {
                fn sequence(&self) -> u16 {
                    ::quent_store::fsm::Sequence::sequence(&self.event.data)
                }
            }
            impl ::quent_store::fsm::Transition for #handle<'_> {
                fn name(&self) -> &str { #name }
                fn is_final(&self) -> bool { #is_final }
            }
        });
    }
    let debug = opts.debug.then(|| quote! { #[derive(Debug)] });
    let docs = format!("Borrows a transition event of `{}`.", entity.path());
    FsmCode {
        declarations: quote! {
            #(#event_impls)*

            #[doc = #docs]
            #[derive(Clone, Copy)]
            #debug
            pub enum #transition<'a> {
                #(#variants,)*
            }
            impl ::quent_store::Timestamp for #transition<'_> {
                fn timestamp(&self) -> ::quent_store::TimeUnixNanoSec {
                    match self { #(#timestamp_arms,)* }
                }
            }
            impl ::quent_store::fsm::Sequence for #transition<'_> {
                fn sequence(&self) -> u16 {
                    match self { #(#sequence_arms,)* }
                }
            }
            impl ::quent_store::fsm::Transition for #transition<'_> {
                fn name(&self) -> &str {
                    match self { #(#name_arms,)* }
                }
                fn is_final(&self) -> bool {
                    match self { #(#final_arms,)* }
                }
            }
        },
        store: store::generate_fsm(entity, marker, original, &transition),
    }
}
