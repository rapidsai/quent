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

struct ExtraVariant {
    variant_name: Ident,
    event_type: Path,
}

struct DefineModelInput {
    name: Ident,
    components: Vec<Path>,
    extras: Vec<ExtraVariant>,
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

        // Optional: extra { Variant: Type, ... }
        let mut extras = Vec::new();
        if input.peek(syn::Ident) {
            let kw: Ident = input.parse()?;
            if kw == "extra" {
                let extra_content;
                syn::braced!(extra_content in input);
                while !extra_content.is_empty() {
                    let variant_name: Ident = extra_content.parse()?;
                    extra_content.parse::<Token![:]>()?;
                    let event_type: Path = extra_content.parse()?;
                    extras.push(ExtraVariant {
                        variant_name,
                        event_type,
                    });
                    if extra_content.peek(Token![,]) {
                        extra_content.parse::<Token![,]>()?;
                    }
                }
            }
        }

        Ok(DefineModelInput {
            name,
            components,
            extras,
        })
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

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DefineModelInput = syn::parse2(input)?;
    let name = &input.name;

    let model_type = format_ident!("{}Model", name);
    let event_type = format_ident!("{}Event", name);

    let paths = &input.components;
    let variants: Vec<Ident> = input.components.iter().map(last_segment).collect();
    let event_types: Vec<Path> = input.components.iter().map(event_type_path).collect();

    let extra_variant_names: Vec<&Ident> = input.extras.iter().map(|e| &e.variant_name).collect();
    let extra_event_types: Vec<&Path> = input.extras.iter().map(|e| &e.event_type).collect();

    let output = quote! {
        pub type #model_type = quent_model::Model<(#(#paths,)*)>;

        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        pub enum #event_type {
            #(#variants(#event_types),)*
            #(#extra_variant_names(#extra_event_types),)*
        }

        #(
            impl From<#event_types> for #event_type {
                fn from(e: #event_types) -> Self {
                    #event_type::#variants(e)
                }
            }
        )*

        #(
            impl From<#extra_event_types> for #event_type {
                fn from(e: #extra_event_types) -> Self {
                    #event_type::#extra_variant_names(e)
                }
            }
        )*
    };

    Ok(output)
}
