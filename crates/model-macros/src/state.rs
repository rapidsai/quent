// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Field, ItemStruct};

use crate::util::to_snake_case;

/// Categorized fields of a state struct.
struct StateFields {
    /// Fields annotated with `#[quent_model::usage]`.
    usages: Vec<UsageField>,
    /// Fields annotated with `#[quent_model::deferred]`.
    deferred: Vec<DeferredField>,
    /// Regular (non-usage, non-deferred) fields.
    regular: Vec<RegularField>,
}

struct UsageField {
    name: String,
}

struct DeferredField {
    name: String,
    /// The inner type (unwrapped from Option<T>).
    inner_ty: TokenStream,
}

struct RegularField {
    name: String,
    optional: bool,
}

fn has_attr(field: &Field, attr_name: &str) -> bool {
    field.attrs.iter().any(|a| {
        a.path()
            .segments
            .last()
            .is_some_and(|seg| seg.ident == attr_name)
    })
}

/// Tries to extract T from Option<T>.
fn unwrap_option_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        let seg = type_path.path.segments.last()?;
        if seg.ident == "Option" {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

/// Check if a type is Option<T>.
fn is_option_type(ty: &syn::Type) -> bool {
    unwrap_option_type(ty).is_some()
}

fn categorize_fields(item: &ItemStruct) -> syn::Result<StateFields> {
    let mut usages = Vec::new();
    let mut deferred = Vec::new();
    let mut regular = Vec::new();

    let fields = match &item.fields {
        syn::Fields::Named(named) => &named.named,
        syn::Fields::Unit => return Ok(StateFields { usages, deferred, regular }),
        _ => {
            return Err(syn::Error::new_spanned(
                item,
                "state structs must have named fields or be unit structs",
            ));
        }
    };

    for field in fields {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "unnamed fields not supported"))?;
        let name = field_name.to_string();

        if has_attr(field, "usage") {
            usages.push(UsageField { name });
        } else if has_attr(field, "deferred") {
            if !is_option_type(&field.ty) {
                return Err(syn::Error::new_spanned(
                    field,
                    "deferred fields must be Option<T>",
                ));
            }
            let inner = unwrap_option_type(&field.ty).unwrap();
            let inner_ty = quote! { #inner };
            deferred.push(DeferredField { name, inner_ty });
        } else {
            let optional = is_option_type(&field.ty);
            regular.push(RegularField { name, optional });
        }
    }

    Ok(StateFields { usages, deferred, regular })
}

pub fn expand(item: TokenStream) -> syn::Result<TokenStream> {
    let input: ItemStruct = syn::parse2(item)?;
    let vis = &input.vis;
    let state_name_ident = &input.ident;
    let state_snake = to_snake_case(state_name_ident);
    let fields = categorize_fields(&input)?;

    // Generate the deferred enum
    let deferred_ident = format_ident!("{}Deferred", state_name_ident);

    let deferred_variants: Vec<TokenStream> = fields
        .deferred
        .iter()
        .map(|d| {
            let variant = format_ident!("{}", crate::util::to_pascal_case(&d.name));
            let inner = &d.inner_ty;
            quote! { #variant(#inner) }
        })
        .collect();

    let deferred_enum = if fields.deferred.is_empty() {
        // Uninhabitable enum — no deferred fields
        quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            #vis enum #deferred_ident {}
        }
    } else {
        quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            #vis enum #deferred_ident {
                #(#deferred_variants,)*
            }
        }
    };

    // Generate attribute defs for regular fields
    let regular_attr_defs: Vec<TokenStream> = fields
        .regular
        .iter()
        .map(|r| {
            let name = &r.name;
            let optional = r.optional;
            // We use a placeholder value type here; the actual type mapping
            // will be refined when we have full type resolution.
            quote! {
                quent_model::AttributeDef {
                    name: #name.to_string(),
                    value_type: quent_model::ValueType::String, // placeholder
                    optional: #optional,
                }
            }
        })
        .collect();

    // Generate deferred attribute defs
    let deferred_attr_defs: Vec<TokenStream> = fields
        .deferred
        .iter()
        .map(|d| {
            let name = &d.name;
            quote! {
                quent_model::AttributeDef {
                    name: #name.to_string(),
                    value_type: quent_model::ValueType::String, // placeholder
                    optional: true,
                }
            }
        })
        .collect();

    // Generate usage defs
    let usage_defs: Vec<TokenStream> = fields
        .usages
        .iter()
        .map(|u| {
            let name = &u.name;
            quote! {
                quent_model::UsageDef {
                    field_name: #name.to_string(),
                    resource_name: String::new(), // resolved at collection time
                    capacities: vec![],
                }
            }
        })
        .collect();

    // Strip quent_model attributes from the original struct fields before
    // re-emitting, so the compiler doesn't complain about unknown attributes.
    let mut clean_item = input.clone();
    if let syn::Fields::Named(ref mut named) = clean_item.fields {
        for field in named.named.iter_mut() {
            field
                .attrs
                .retain(|a| {
                    let last = a.path().segments.last();
                    !last.is_some_and(|seg| seg.ident == "usage" || seg.ident == "deferred")
                });
        }
    }

    let output = quote! {
        // Re-emit the original struct with derives needed by the FSM event enums
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #clean_item

        // --- Deferred enum ---
        #deferred_enum

        // --- State trait impl ---
        impl quent_model::State for #state_name_ident {}

        // --- StateMetadata impl ---
        impl quent_model::StateMetadata for #state_name_ident {
            type Deferred = #deferred_ident;

            fn state_name() -> &'static str {
                #state_snake
            }

            fn state_def() -> quent_model::StateDef {
                quent_model::StateDef {
                    name: #state_snake.to_string(),
                    attributes: vec![#(#regular_attr_defs,)*],
                    deferred_attributes: vec![#(#deferred_attr_defs,)*],
                    usages: vec![#(#usage_defs,)*],
                }
            }
        }
    };

    Ok(output)
}
