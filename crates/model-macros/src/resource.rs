// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::TokenStream;
use quote::quote;

pub fn expand(_attr: TokenStream, _item: TokenStream) -> syn::Result<TokenStream> {
    Ok(quote! {
        compile_error!(
            "#[quent_model::resource] is not a standalone attribute. \
             Use `resource(capacity = T)` inside #[quent_model::fsm(...)] instead."
        );
    })
}
