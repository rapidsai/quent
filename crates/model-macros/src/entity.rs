// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

use crate::util::to_snake_case;

pub fn expand(item: TokenStream) -> syn::Result<TokenStream> {
    let input: ItemStruct = syn::parse2(item)?;
    let name = &input.ident;
    let entity_snake = to_snake_case(name);

    // Collect attribute defs from struct fields
    let attr_defs: Vec<TokenStream> = match &input.fields {
        syn::Fields::Named(named) => named
            .named
            .iter()
            .map(|field| {
                let field_name = field.ident.as_ref().unwrap().to_string();
                quote! {
                    quent_model::AttributeDef {
                        name: #field_name.to_string(),
                        value_type: quent_model::ValueType::String, // placeholder
                        optional: false,
                    }
                }
            })
            .collect(),
        _ => vec![],
    };

    let output = quote! {
        #input

        impl quent_model::Entity for #name {}

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_entity(quent_model::EntityDef {
                    name: #entity_snake.to_string(),
                    attributes: vec![#(#attr_defs,)*],
                    events: vec![],
                });
            }
        }
    };

    Ok(output)
}
