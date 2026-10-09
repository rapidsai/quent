// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of entity-specific event accessor declarations.

use proc_macro2::TokenStream;
use quent_instrumentation_build::generated_module_ident;
use quent_schema::{Cardinality, Entity, Event};
use quote::quote;

/// The name, signature, and trait declaration of one event accessor.
pub(super) struct AccessorCode {
    /// The Rust method name, such as `init`.
    pub(super) method_ident: syn::Ident,
    /// The signature shared by the trait declaration and backend
    /// implementations.
    ///
    /// For example, `fn init(&self) -> Option<InitHandle<'_>>`.
    pub(super) method_signature: TokenStream,
    /// The documented trait method declaration, including its semicolon.
    ///
    /// For example, `fn init(&self) -> Option<InitHandle<'_>>;`.
    pub(super) method_decl: TokenStream,
}

pub(super) fn generate(entity: &Entity, event: &Event, handle: &syn::Ident) -> AccessorCode {
    let method = generated_module_ident(event.name());
    let (return_type, docs) =
        if event.cardinality() == Cardinality::Once && entity.events().count() == 1 {
            (
                quote! { #handle<'_> },
                quote! {
                    /// Borrows the entity's sole event.
                },
            )
        } else if event.cardinality() == Cardinality::Once {
            (
                quote! { ::core::option::Option<#handle<'_>> },
                quote! {
                    /// Borrows the recorded event, returning `None` when absent.
                },
            )
        } else {
            (
                quote! { impl ::core::iter::Iterator<Item = #handle<'_>> },
                quote! {
                    /// Borrows events in timestamp order; equal timestamps retain input order.
                },
            )
        };
    let method_signature = quote! { fn #method(&self) -> #return_type };
    let method_decl = quote! { #docs #method_signature; };
    AccessorCode {
        method_ident: method,
        method_signature,
        method_decl,
    }
}
