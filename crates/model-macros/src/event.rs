// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemStruct, Token};

/// Parses `entity = EntityType`.
struct EventAttr {
    entity_type: Ident,
}

impl Parse for EventAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "entity" {
            return Err(syn::Error::new_spanned(key, "expected `entity = Type`"));
        }
        input.parse::<Token![=]>()?;
        let entity_type: Ident = input.parse()?;
        Ok(EventAttr { entity_type })
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let EventAttr { entity_type } = syn::parse2(attr)?;
    let input: ItemStruct = syn::parse2(item)?;
    let name = &input.ident;

    let output = quote! {
        #input

        impl quent_model::EntityEvent for #name {
            type Entity = #entity_type;
        }
    };

    Ok(output)
}
