// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::DeriveInput;

use crate::util::to_snake_case;

/// Check for `#[resource_group]` or `#[resource_group(root)]` outer attribute.
fn parse_resource_group_attr(input: &DeriveInput) -> syn::Result<bool> {
    for attr in &input.attrs {
        if attr
            .path()
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "resource_group")
        {
            // Check if it has (root) argument
            if let syn::Meta::List(list) = &attr.meta {
                if let Ok(ident) = syn::parse2::<Ident>(list.tokens.clone()) {
                    if ident == "root" {
                        return Ok(true);
                    }
                }
            }
            return Ok(false);
        }
    }
    Ok(false)
}

/// Expand the ResourceGroup derive macro.
///
/// Generates:
/// - `ResourceGroup` trait impl with IS_ROOT
/// - `ModelComponent` impl with resource group contribution
///
/// When combined with `Entity` or `Fsm` derives that also generate
/// `ModelComponent`, a conflict would arise. To resolve this:
/// - Entity and Fsm derives detect `#[resource_group]` and include the
///   resource group contribution in their own `ModelComponent` impl
/// - Entity and Fsm derives generate `ModelComponent`, so ResourceGroup
///   must NOT when used alongside them
///
/// Convention: When Entity or Fsm is also derived, do NOT also derive
/// ResourceGroup. Instead, just put `#[resource_group]` or
/// `#[resource_group(root)]` on the struct -- Entity/Fsm will detect it.
///
/// ResourceGroup derive is only for standalone resource groups (not entities
/// or FSMs).
///
/// Does NOT re-emit the struct (derive macros append).
pub fn expand_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let is_root = parse_resource_group_attr(&input)?;
    let group_snake = to_snake_case(name);

    let output = quote! {
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
