// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::util::{resolve_value_type, to_snake_case};

/// Expand the Event derive macro.
///
/// Introspects the struct's fields and generates an `EventMetadata` impl
/// that returns an `EntityEventDef` with populated attributes.
pub fn expand_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let event_snake = to_snake_case(name);

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(named) => &named.named,
            syn::Fields::Unit => {
                return Ok(quote! {
                    impl quent_model::EventMetadata for #name {
                        fn event_def() -> quent_model::EntityEventDef {
                            quent_model::EntityEventDef {
                                name: #event_snake.to_string(),
                                attributes: vec![],
                            }
                        }
                    }
                });
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "Event derive requires named fields or unit struct",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Event can only be derived on structs",
            ));
        }
    };

    let attr_defs: Vec<TokenStream> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap().to_string();
            let (value_type_tokens, optional) = resolve_value_type(&field.ty);
            quote! {
                quent_model::AttributeDef {
                    name: #field_name.to_string(),
                    value_type: #value_type_tokens,
                    optional: #optional,
                }
            }
        })
        .collect();

    Ok(quote! {
        impl quent_model::EventMetadata for #name {
            fn event_def() -> quent_model::EntityEventDef {
                quent_model::EntityEventDef {
                    name: #event_snake.to_string(),
                    attributes: vec![#(#attr_defs,)*],
                }
            }
        }
    })
}
