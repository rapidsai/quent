// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `resource!` proc macro implementation.
//!
//! Transforms a DSL into the existing `#[derive(Resource)]` /
//! `#[derive(ResizableResource)]` format. The macro adds `capacity_` prefixes
//! and `Capacity<T>` wrappers so the user doesn't need them.
//!
//! ```ignore
//! resource! { Thread }
//!
//! resource! {
//!     Memory {
//!         capacity: { bytes: Option<u64> },
//!     }
//! }
//!
//! resource! {
//!     Channel {
//!         attributes: { source_id: Uuid, target_id: Uuid },
//!         capacity: { bytes: Option<u64> },
//!     }
//! }
//!
//! resource! {
//!     ResizableMemory {
//!         resizable: true,
//!         capacity: { bytes: u64 },
//!     }
//! }
//! ```

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Token, Type, braced};

struct CapacityField {
    name: Ident,
    ty: Type,
}

struct AttributeField {
    name: Ident,
    ty: Type,
}

struct ResourceInput {
    user_attrs: Vec<syn::Attribute>,
    name: Ident,
    resizable: bool,
    attributes: Vec<AttributeField>,
    capacities: Vec<CapacityField>,
    /// Capacity kind marker (e.g., `Rate`). Default is `Occupancy`.
    capacity_kind: Option<Ident>,
}

impl Parse for ResourceInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let user_attrs = input.call(syn::Attribute::parse_outer)?;
        let name: Ident = input.parse()?;

        // Unit resource: just a name, no body
        if input.is_empty() {
            return Ok(ResourceInput {
                user_attrs,
                name,
                resizable: false,
                attributes: Vec::new(),
                capacities: Vec::new(),
                capacity_kind: None,
            });
        }

        let content;
        braced!(content in input);

        let mut resizable = false;
        let mut attributes = Vec::new();
        let mut capacities = Vec::new();
        let mut capacity_kind = None;

        while !content.is_empty() {
            let kw: Ident = content.parse()?;

            match kw.to_string().as_str() {
                "resizable" => {
                    content.parse::<Token![:]>()?;
                    let val: syn::LitBool = content.parse()?;
                    resizable = val.value;
                }
                "attributes" => {
                    content.parse::<Token![:]>()?;
                    let fields_content;
                    braced!(fields_content in content);
                    while !fields_content.is_empty() {
                        let field_name: Ident = fields_content.parse()?;
                        fields_content.parse::<Token![:]>()?;
                        let ty: Type = fields_content.parse()?;
                        attributes.push(AttributeField {
                            name: field_name,
                            ty,
                        });
                        if fields_content.peek(Token![,]) {
                            fields_content.parse::<Token![,]>()?;
                        }
                    }
                }
                "capacity" => {
                    content.parse::<Token![:]>()?;
                    let fields_content;
                    braced!(fields_content in content);
                    // Check for optional kind flag (rate/occupancy) as first token.
                    if !fields_content.is_empty() {
                        let fork = fields_content.fork();
                        if let Ok(ident) = fork.parse::<Ident>() {
                            let name = ident.to_string();
                            if name == "rate" || name == "occupancy" {
                                let kind_ident: Ident = fields_content.parse()?;
                                capacity_kind = Some(kind_ident);
                                if fields_content.peek(Token![,]) {
                                    fields_content.parse::<Token![,]>()?;
                                }
                            }
                        }
                    }
                    while !fields_content.is_empty() {
                        let field_name: Ident = fields_content.parse()?;
                        let field_name_str = field_name.to_string();
                        if field_name_str == "rate" || field_name_str == "occupancy" {
                            return Err(syn::Error::new_spanned(
                                field_name,
                                format!(
                                    "`{field_name_str}` is a reserved keyword in capacity blocks (used to set the capacity kind); choose a different field name"
                                ),
                            ));
                        }
                        fields_content.parse::<Token![:]>()?;
                        let ty: Type = fields_content.parse()?;
                        capacities.push(CapacityField {
                            name: field_name,
                            ty,
                        });
                        if fields_content.peek(Token![,]) {
                            fields_content.parse::<Token![,]>()?;
                        }
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        kw,
                        format!(
                            "unexpected keyword `{other}`, expected `resizable`, `attributes`, or `capacity`"
                        ),
                    ));
                }
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(ResourceInput {
            user_attrs,
            name,
            resizable,
            attributes,
            capacities,
            capacity_kind,
        })
    }
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: ResourceInput = syn::parse2(input)?;
    let name = &input.name;
    let user_attrs = &input.user_attrs;

    let derive = if input.resizable {
        quote! { quent_model::ResizableResource }
    } else {
        quote! { quent_model::Resource }
    };

    // Attribute fields go directly on the struct (init fields for the resource derive)
    let attr_fields: Vec<TokenStream> = input
        .attributes
        .iter()
        .map(|f| {
            let fname = &f.name;
            let fty = &f.ty;
            quote! { pub #fname: #fty }
        })
        .collect();

    // Capacity fields get `capacity_` prefix and `Capacity<T, K>` wrapper
    let cap_fields: Vec<TokenStream> = input
        .capacities
        .iter()
        .map(|f| {
            let prefixed = quote::format_ident!("capacity_{}", f.name);
            let ty = &f.ty;
            if let Some(kind) = &input.capacity_kind {
                let kind_type = match kind.to_string().as_str() {
                    "rate" => quote! { quent_model::Rate },
                    _ => quote! { quent_model::Occupancy },
                };
                quote! { pub #prefixed: quent_model::Capacity<#ty, #kind_type> }
            } else {
                quote! { pub #prefixed: quent_model::Capacity<#ty> }
            }
        })
        .collect();

    if attr_fields.is_empty() && cap_fields.is_empty() {
        // Unit resource
        Ok(quote! {
            #(#user_attrs)*
            #[derive(#derive)]
            pub struct #name;
        })
    } else {
        Ok(quote! {
            #(#user_attrs)*
            #[derive(#derive)]
            pub struct #name {
                #(#attr_fields,)*
                #(#cap_fields,)*
            }
        })
    }
}
