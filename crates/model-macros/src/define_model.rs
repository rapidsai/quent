// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `define_model!` proc macro implementation.
//!
//! Syntax:
//! ```ignore
//! define_model! {
//!     Simulator {
//!         quent_query_engine_model::Engine,
//!         task::Task,
//!         quent_stdlib::Memory,
//!     }
//! }
//! ```
//!
//! Generates `SimulatorModel` (type alias) and `SimulatorEvent` (event enum).

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Path, Token};

struct DefineModelInput {
    name: Ident,
    components: Vec<Path>,
}

impl Parse for DefineModelInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let content;
        syn::braced!(content in input);

        let mut components = Vec::new();
        while !content.is_empty() {
            components.push(content.parse::<Path>()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(DefineModelInput { name, components })
    }
}

/// Extract the last segment of a path as an Ident.
fn last_segment(path: &Path) -> Ident {
    path.segments.last().unwrap().ident.clone()
}

/// Given a path like `foo::bar::Baz`, construct `foo::bar::BazEvent`.
fn event_type_path(path: &Path) -> Path {
    let mut event_path = path.clone();
    if let Some(last) = event_path.segments.last_mut() {
        last.ident = format_ident!("{}Event", last.ident);
    }
    event_path
}

/// Build a nested tuple type from a list of paths, chunking into groups of 16.
fn nested_tuple(paths: &[Path]) -> TokenStream {
    if paths.len() <= 16 {
        quote! { (#(#paths,)*) }
    } else {
        let chunks: Vec<TokenStream> = paths
            .chunks(16)
            .map(|chunk| quote! { (#(#chunk,)*) })
            .collect();
        quote! { (#(#chunks,)*) }
    }
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let serde_derives = crate::util::serde_derives();
    let input: DefineModelInput = syn::parse2(input)?;
    let name = &input.name;

    let model_type = format_ident!("{}Model", name);
    let event_type = format_ident!("{}Event", name);

    let paths = &input.components;
    let variants: Vec<Ident> = input.components.iter().map(last_segment).collect();
    let event_types: Vec<Path> = input.components.iter().map(event_type_path).collect();
    let model_tuple = nested_tuple(paths);

    let output = quote! {
        pub type #model_type = quent_model::Model<#model_tuple>;


        #[derive(Debug #serde_derives)]
        pub enum #event_type {
            #(#variants(#event_types),)*
        }

        #(
            impl From<#event_types> for #event_type {
                fn from(e: #event_types) -> Self {
                    #event_type::#variants(e)
                }
            }
        )*

        #[doc(hidden)]
        pub use quent_model as __quent;
    };

    Ok(output)
}
