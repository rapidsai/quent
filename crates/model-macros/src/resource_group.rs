// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemStruct};

use crate::util::to_snake_case;

/// Parses optional `root` parameter.
struct ResourceGroupAttr {
    is_root: bool,
}

impl Parse for ResourceGroupAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(ResourceGroupAttr { is_root: false });
        }
        let ident: Ident = input.parse()?;
        if ident == "root" {
            Ok(ResourceGroupAttr { is_root: true })
        } else {
            Err(syn::Error::new_spanned(
                ident,
                "expected `root` or no arguments",
            ))
        }
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let ResourceGroupAttr { is_root } = syn::parse2(attr)?;
    let input: ItemStruct = syn::parse2(item)?;
    let name = &input.ident;
    let group_snake = to_snake_case(name);

    let output = quote! {
        #input

        impl quent_model::ResourceGroup for #name {
            const IS_ROOT: bool = #is_root;
        }

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_resource_group(quent_model::ResourceGroupDef {
                    name: #group_snake.to_string(),
                    fixed_parent: None,
                    is_root: #is_root,
                });
            }
        }
    };

    Ok(output)
}
