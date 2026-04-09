// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::util::{resolve_value_type, to_snake_case};

/// Categorized fields of a state struct.
struct StateFields {
    /// Fields with `Usage<T>` type (detected automatically).
    usages: Vec<UsageField>,
    /// Fields with `Capacity<V, K>` type (detected by type, not annotation).
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
    /// The inner resource type T from Usage<T>.
    resource_ty: syn::Type,
}

struct RegularField {
    name: String,
    ty: syn::Type,
    optional: bool,
}

use crate::util::{field_has_attr as has_attr, is_capacity_type};

/// Extract the inner type T from `Usage<T>`, or return None.
fn extract_usage_inner(ty: &syn::Type) -> Option<syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let seg = type_path.path.segments.last()?;
    if seg.ident != "Usage" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };
    let syn::GenericArgument::Type(inner) = args.args.first()? else {
        return None;
    };
    Some(inner.clone())
}

/// Tries to extract T from Option<T>.
fn unwrap_option_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        let seg = type_path.path.segments.last()?;
        if seg.ident == "Option"
            && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
        {
            return Some(inner);
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
                    capacities,
                    instance_name_field,
                    parent_group_field,
                    regular,
                });
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

        if let Some(resource_ty) = extract_usage_inner(&field.ty) {
            usages.push(UsageField {
                name,
                ident: field_name.clone(),
                resource_ty,
            });
        } else if has_attr(field, "deferred") {
            return Err(syn::Error::new_spanned(
                field,
                "#[deferred] is not yet implemented. \
                 See https://github.com/NVIDIA/quent/issues/75",
            ));
        } else if is_capacity_type(&field.ty) {
            if !name.starts_with("capacity_") {
                return Err(syn::Error::new_spanned(
                    field_name,
                    "Capacity fields must be prefixed with `capacity_` (e.g., `capacity_bytes`)",
                ));
            }
            capacities.push(CapacityField {
                name,
                ident: field_name.clone(),
                optional: false,
            });
        } else if unwrap_option_type(&field.ty).is_some_and(is_capacity_type) {
            // Option<Capacity<V, K>> — should not occur, but handle gracefully
            capacities.push(CapacityField {
                name,
                ident: field_name.clone(),
                optional: true,
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
                ty: field.ty.clone(),
                optional,
            });
        } else {
            let optional = is_option_type(&field.ty);
            regular.push(RegularField {
                name,
                ty: field.ty.clone(),
                optional,
            });
        }
    }

    Ok(StateFields {
        usages,
        capacities,
        instance_name_field,
        parent_group_field,
        regular,
    })
}

/// Expand the State derive macro. Only emits impl blocks.
/// Does NOT re-emit the struct (derive macros append, they don't replace).
pub fn expand_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let state_name_ident = &input.ident;
    let state_snake = to_snake_case(state_name_ident);
    let fields = categorize_fields(&input)?;

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

    // Generate usage defs
    let usage_defs: Vec<TokenStream> = fields
        .usages
        .iter()
        .map(|u| {
            let name = &u.name;
            let resource_ty = &u.resource_ty;
            let resource_ty_str = quote!(#resource_ty).to_string();
            quote! {
                quent_model::UsageDef {
                    field_name: #name.to_string(),
                    resource_name: <#resource_ty as quent_model::Resource>::RESOURCE_NAME.to_string(),
                    resource_type_path: #resource_ty_str.to_string(),
                }
            }
        })
        .collect();

    // Generate ExtractCapacities impl.
    // For unit structs (no fields at all): emit a single "unit" capacity.
    // For structs with Capacity<V, K> fields: emit one capacity per field.
    // For structs without Capacity<V, K> fields: emit empty vec.
    let is_unit_struct =
        fields.regular.is_empty() && fields.usages.is_empty() && fields.capacities.is_empty();

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
                    // Option<Capacity<V, K>> — extract value from inner Capacity
                    quote! {
                        quent_model::analyze::ExtractedCapacity {
                            name: #name,
                            value: self.#field_ident.as_ref().map(|v| v.value as u64),
                        }
                    }
                } else {
                    // Capacity<V, K> — access .value
                    quote! {
                        quent_model::analyze::ExtractedCapacity::new(#name, self.#field_ident.value as u64)
                    }
                }
            })
            .collect();
        quote! { vec![#(#extractions,)*] }
    };

    // Generate ExtractUsages impl from Usage<T> fields.
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

    // Generate flat params and construction for the fsm! callback macro.
    let callback_macro_name = format_ident!("__quent_state_{}", state_snake);
    let state_method_ident = format_ident!("{}", state_snake);

    let mut flat_params: Vec<TokenStream> = Vec::new();
    let mut flat_construction: Vec<TokenStream> = Vec::new();

    // Instance name field → &str param
    if let Some(ref instance_name) = fields.instance_name_field {
        flat_params.push(quote! { #instance_name: &str });
        flat_construction.push(quote! { #instance_name: #instance_name.to_string() });
    }

    // Parent group field
    if let Some(ref parent_group) = fields.parent_group_field {
        flat_params.push(quote! { #parent_group: uuid::Uuid });
        flat_construction.push(quote! { #parent_group: #parent_group.into() });
    }

    // Regular fields (non-instance_name, non-parent_group)
    for rf in &fields.regular {
        let rf_ident = format_ident!("{}", rf.name);
        let rf_ty = &rf.ty;
        if fields
            .instance_name_field
            .as_ref()
            .is_some_and(|n| n == &rf_ident)
        {
            continue; // already handled
        }
        if fields
            .parent_group_field
            .as_ref()
            .is_some_and(|n| n == &rf_ident)
        {
            continue; // already handled
        }
        flat_params.push(quote! { #rf_ident: #rf_ty });
        flat_construction.push(quote! { #rf_ident });
    }

    // Usage fields → Ref<T> + capacity arg
    for usage in &fields.usages {
        let u_ident = &usage.ident;
        let resource_ty = &usage.resource_ty;
        let ref_param = format_ident!("{}", usage.name);
        flat_params.push(quote! { #ref_param: quent_model::Ref<#resource_ty> });
        let cap_param = format_ident!("{}_capacity", usage.name);
        flat_params
            .push(quote! { #cap_param: <#resource_ty as quent_model::Resource>::CapacityValue });
        flat_construction.push(quote! {
            #u_ident: quent_model::Usage {
                resource_id: #ref_param,
                capacity: #cap_param,
            }
        });
    }

    let output = quote! {
        impl quent_model::StateMetadata for #state_name_ident {
            fn state_name() -> &'static str {
                #state_snake
            }

            fn state_def() -> quent_model::StateDef {
                quent_model::StateDef {
                    name: #state_snake.to_string(),
                    attributes: vec![#(#regular_attr_defs,)*],
                    usages: vec![#(#usage_defs,)*],
                }
            }
        }

        impl quent_model::analyze::ExtractCapacities for #state_name_ident {
            fn extract_capacities(&self) -> Vec<quent_model::analyze::ExtractedCapacity> {
                #extract_capacities_body
            }
        }

        impl quent_model::analyze::ExtractUsages for #state_name_ident {
            fn extract_usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> {
                #extract_usages_body
            }
        }

        impl quent_model::analyze::ExtractInstanceName for #state_name_ident {
            fn extract_instance_name(&self) -> Option<&str> {
                #extract_instance_name_body
            }
        }

        impl quent_model::analyze::ExtractParentGroupId for #state_name_ident {
            fn extract_parent_group_id(&self) -> Option<uuid::Uuid> {
                #extract_parent_group_id_body
            }
        }

        #has_parent_group_impl

        // Hidden callback macro for fsm! — provides flat params and construction.
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #callback_macro_name {
            // Entry method: generates a method on the observer that creates a handle
            (entry_method $vis:vis $handle:ident $transition:ident) => {
                $vis fn #state_method_ident(&self, id: uuid::Uuid, #(#flat_params,)*) -> $handle<E> {
                    let state = #state_name_ident { #(#flat_construction,)* };
                    let mut handle = $handle { id, seq: 0, exited: false, tx: self.tx.clone() };
                    handle.emit_transition($transition::from(state));
                    handle
                }
            };
            // Transition method: generates a method on the handle
            (transition_method $vis:vis $transition:ident) => {
                $vis fn #state_method_ident(&mut self, #(#flat_params,)*) {
                    let state = #state_name_ident { #(#flat_construction,)* };
                    self.emit_transition($transition::from(state));
                }
            };
        }
    };

    Ok(output)
}
