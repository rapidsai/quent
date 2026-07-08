// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::util::{resolve_value_type, to_snake_case};

/// Expand the Event derive macro.
///
/// Introspects the struct's fields and generates:
/// - An `EventMetadata` impl that returns an `EntityEventDef` with populated
///   attribute definitions (type-level metadata).
/// - An `ExtractAttributes` impl yielding one `Attribute` per field
///   (value-level extraction).
/// - A `ToAttributeValue` impl for `Self` so the struct can itself appear
///   as a field of another attribute struct or an inline `state!` attribute
///   (a `Vec` of it is handled syntactically via
///   [`crate::util::attribute_value_expr`], as the orphan rule forbids a
///   downstream `impl` on `Vec<Self>`).
pub fn expand_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let event_snake = to_snake_case(name);

    let value_impls = quote! {
        impl quent_model::analyze::ToAttributeValue for #name {
            fn to_attribute_value(&self) -> Option<quent_model::attributes::Value> {
                Some(quent_model::attributes::Value::Struct(
                    quent_model::attributes::Struct(
                        quent_model::analyze::ExtractAttributes::extract_attributes(self),
                    ),
                ))
            }
        }
    };

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

                    impl quent_model::analyze::ExtractAttributes for #name {
                        fn extract_attributes(&self) -> Vec<quent_model::attributes::Attribute> {
                            vec![]
                        }
                    }

                    #value_impls
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

    let extract_attr_tokens: Vec<TokenStream> = fields
        .iter()
        .map(|field| {
            let fname = field.ident.as_ref().unwrap();
            let field_name = fname.to_string();
            let value_expr = crate::util::attribute_value_expr(&quote! { self.#fname }, &field.ty);
            quote! {
                quent_model::attributes::Attribute {
                    key: #field_name.to_string(),
                    value: #value_expr,
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

        impl quent_model::analyze::ExtractAttributes for #name {
            fn extract_attributes(&self) -> Vec<quent_model::attributes::Attribute> {
                vec![#(#extract_attr_tokens,)*]
            }
        }

        #value_impls
    })
}
