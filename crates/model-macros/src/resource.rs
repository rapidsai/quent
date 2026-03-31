// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Token};

/// Parses `capacity = StateName`.
struct ResourceAttr {
    capacity_state: Ident,
}

impl Parse for ResourceAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "capacity" {
            return Err(syn::Error::new_spanned(
                key,
                "expected `capacity = StateName`",
            ));
        }
        input.parse::<Token![=]>()?;
        let capacity_state: Ident = input.parse()?;
        Ok(ResourceAttr { capacity_state })
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let ResourceAttr { capacity_state } = syn::parse2(attr)?;

    // Extract the struct name from the item. The item is the raw struct before
    // #[fsm] processes it (since #[resource] is below #[fsm], it runs first
    // in Rust's outside-in attribute processing). We parse as a struct, extract
    // the name, then pass through the item unchanged and append the Resource
    // impl.
    let input: syn::ItemStruct = syn::parse2(item)?;
    let name = &input.ident;

    // Strip the #[resource] attr but keep all other attrs (including #[fsm])
    // for the next macro to process.
    let output = quote! {
        #input

        impl quent_model::Resource for #name {
            type CapacityValue = #capacity_state;
        }
    };

    Ok(output)
}
