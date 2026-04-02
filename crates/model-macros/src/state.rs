// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::util::{resolve_value_type, to_snake_case};

/// Categorized fields of a state struct.
struct StateFields {
    /// Fields annotated with `#[usage]`.
    usages: Vec<UsageField>,
    /// Fields annotated with `#[deferred]`.
    deferred: Vec<DeferredField>,
    /// Fields annotated with `#[capacity]` (numeric capacity values).
    capacities: Vec<CapacityField>,
    /// Field annotated with `#[instance_name]` (at most one).
    instance_name_field: Option<Ident>,
    /// Field annotated with `#[parent_group]` (at most one).
    parent_group_field: Option<Ident>,
    /// Regular fields.
    regular: Vec<RegularField>,
}

struct CapacityField {
    name: String,
    ident: Ident,
    optional: bool,
}

struct UsageField {
    name: String,
    ident: Ident,
}

struct DeferredField {
    name: String,
    /// The inner type (unwrapped from Option<T>), as tokens for code generation.
    inner_ty: TokenStream,
    /// The inner type (unwrapped from Option<T>), as syn::Type for value type resolution.
    inner_type: syn::Type,
}

struct RegularField {
    name: String,
    #[allow(dead_code)]
    ident: Ident,
    ty: syn::Type,
    optional: bool,
}

use crate::util::field_has_attr as has_attr;

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

fn categorize_fields(input: &DeriveInput) -> syn::Result<StateFields> {
    let mut usages = Vec::new();
    let mut deferred = Vec::new();
    let mut capacities = Vec::new();
    let mut instance_name_field = None;
    let mut parent_group_field = None;
    let mut regular = Vec::new();

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(named) => &named.named,
            syn::Fields::Unit => {
                return Ok(StateFields {
                    usages,
                    deferred,
                    capacities,
                    instance_name_field,
                    parent_group_field,
                    regular,
                })
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "state structs must have named fields or be unit structs",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "State can only be derived on structs",
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
            usages.push(UsageField {
                name,
                ident: field_name.clone(),
            });
        } else if has_attr(field, "deferred") {
            if !is_option_type(&field.ty) {
                return Err(syn::Error::new_spanned(
                    field,
                    "deferred fields must be Option<T>",
                ));
            }
            let inner = unwrap_option_type(&field.ty).unwrap();
            let inner_ty = quote! { #inner };
            deferred.push(DeferredField { name, inner_ty, inner_type: inner.clone() });
        } else if has_attr(field, "capacity") {
            let optional = is_option_type(&field.ty);
            capacities.push(CapacityField {
                name,
                ident: field_name.clone(),
                optional,
            });
        } else if has_attr(field, "instance_name") {
            if instance_name_field.is_some() {
                return Err(syn::Error::new_spanned(
                    field,
                    "only one field can be annotated with #[instance_name]",
                ));
            }
            instance_name_field = Some(field_name.clone());
            // Also add as a regular field for metadata
            let optional = is_option_type(&field.ty);
            regular.push(RegularField {
                name,
                ident: field_name.clone(),
                ty: field.ty.clone(),
                optional,
            });
        } else if has_attr(field, "parent_group") {
            if parent_group_field.is_some() {
                return Err(syn::Error::new_spanned(
                    field,
                    "only one field can be annotated with #[parent_group]",
                ));
            }
            parent_group_field = Some(field_name.clone());
            // Also add as a regular field for metadata
            let optional = is_option_type(&field.ty);
            regular.push(RegularField {
                name,
                ident: field_name.clone(),
                ty: field.ty.clone(),
                optional,
            });
        } else {
            let optional = is_option_type(&field.ty);
            regular.push(RegularField {
                name,
                ident: field_name.clone(),
                ty: field.ty.clone(),
                optional,
            });
        }
    }

    Ok(StateFields {
        usages,
        deferred,
        capacities,
        instance_name_field,
        parent_group_field,
        regular,
    })
}

/// Expand the State derive macro. Only emits impl blocks and the Deferred enum.
/// Does NOT re-emit the struct (derive macros append, they don't replace).
pub fn expand_derive(input: DeriveInput) -> syn::Result<TokenStream> {
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
        // Uninhabitable enum -- no deferred fields
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
            let (value_type_tokens, _) = resolve_value_type(&r.ty);
            quote! {
                quent_model::AttributeDef {
                    name: #name.to_string(),
                    value_type: #value_type_tokens,
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
            let (value_type_tokens, _) = resolve_value_type(&d.inner_type);
            quote! {
                quent_model::AttributeDef {
                    name: #name.to_string(),
                    value_type: #value_type_tokens,
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

    // Generate ExtractCapacities impl.
    // For unit structs (no fields at all): emit a single "unit" capacity.
    // For structs with #[capacity] fields: emit one capacity per annotated field.
    // For structs without #[capacity] fields: emit empty vec.
    let is_unit_struct = fields.regular.is_empty()
        && fields.usages.is_empty()
        && fields.deferred.is_empty()
        && fields.capacities.is_empty();

    let extract_capacities_body = if is_unit_struct {
        quote! { vec![quent_model::analyze::ExtractedCapacity::unit()] }
    } else if fields.capacities.is_empty() {
        quote! { vec![] }
    } else {
        let extractions: Vec<TokenStream> = fields
            .capacities
            .iter()
            .map(|c| {
                let field_ident = &c.ident;
                let name = &c.name;
                if c.optional {
                    quote! {
                        quent_model::analyze::ExtractedCapacity {
                            name: #name,
                            value: self.#field_ident.map(|v| v as u64),
                        }
                    }
                } else {
                    quote! {
                        quent_model::analyze::ExtractedCapacity::new(#name, self.#field_ident as u64)
                    }
                }
            })
            .collect();
        quote! { vec![#(#extractions,)*] }
    };

    // Generate ExtractUsages impl from #[usage] fields.
    let extract_usages_body = if fields.usages.is_empty() {
        quote! { vec![] }
    } else {
        let extractions: Vec<TokenStream> = fields
            .usages
            .iter()
            .map(|u| {
                let field_ident = &u.ident;
                quote! {
                    quent_model::analyze::ExtractedUsage {
                        resource_id: self.#field_ident.resource_id.uuid(),
                        capacities: quent_model::analyze::ExtractCapacities::extract_capacities(
                            &self.#field_ident.capacity
                        ),
                    }
                }
            })
            .collect();
        quote! { vec![#(#extractions,)*] }
    };

    // Generate ExtractInstanceName impl.
    let extract_instance_name_body = match &fields.instance_name_field {
        Some(ident) => quote! { Some(self.#ident.as_str()) },
        None => quote! { None },
    };

    // Generate ExtractParentGroupId impl.
    let extract_parent_group_id_body = match &fields.parent_group_field {
        Some(ident) => quote! { Some(self.#ident.into()) },
        None => quote! { None },
    };

    // Generate HasParentGroup marker trait impl if applicable.
    let has_parent_group_impl = if fields.parent_group_field.is_some() {
        quote! {
            impl quent_model::HasParentGroup for #state_name_ident {}
        }
    } else {
        quote! {}
    };

    let output = quote! {
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

        // --- ExtractCapacities impl ---
        impl quent_model::analyze::ExtractCapacities for #state_name_ident {
            fn extract_capacities(&self) -> Vec<quent_model::analyze::ExtractedCapacity> {
                #extract_capacities_body
            }
        }

        // --- ExtractUsages impl ---
        impl quent_model::analyze::ExtractUsages for #state_name_ident {
            fn extract_usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> {
                #extract_usages_body
            }
        }

        // --- ExtractInstanceName impl ---
        impl quent_model::analyze::ExtractInstanceName for #state_name_ident {
            fn extract_instance_name(&self) -> Option<&str> {
                #extract_instance_name_body
            }
        }

        // --- ExtractParentGroupId impl ---
        impl quent_model::analyze::ExtractParentGroupId for #state_name_ident {
            fn extract_parent_group_id(&self) -> Option<uuid::Uuid> {
                #extract_parent_group_id_body
            }
        }

        // --- HasParentGroup marker (if applicable) ---
        #has_parent_group_impl
    };

    Ok(output)
}
