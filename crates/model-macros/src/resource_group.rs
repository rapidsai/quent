// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemStruct, Token};

use crate::util::to_snake_case;

/// Parses optional `parent = ParentType`.
struct ResourceGroupAttr {
    parent: Option<Ident>,
}

impl Parse for ResourceGroupAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(ResourceGroupAttr { parent: None });
        }
        let key: Ident = input.parse()?;
        if key != "parent" {
            return Err(syn::Error::new_spanned(key, "expected `parent = Type`"));
        }
        input.parse::<Token![=]>()?;
        let parent: Ident = input.parse()?;
        Ok(ResourceGroupAttr {
            parent: Some(parent),
        })
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let ResourceGroupAttr { parent } = syn::parse2(attr)?;
    let input: ItemStruct = syn::parse2(item)?;
    let name = &input.ident;
    let group_snake = to_snake_case(name);

    let fixed_parent_token = match &parent {
        Some(p) => {
            let parent_snake = to_snake_case(p);
            quote! { Some(#parent_snake.to_string()) }
        }
        None => quote! { None },
    };

    let output = quote! {
        #input

        impl quent_model::ResourceGroup for #name {}

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_resource_group(quent_model::ResourceGroupDef {
                    name: #group_snake.to_string(),
                    fixed_parent: #fixed_parent_token,
                });
            }
        }
    };

    Ok(output)
}
