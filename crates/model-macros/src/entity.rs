// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::util::{field_has_attr, parse_resource_group_attr, resolve_value_type, to_snake_case};

/// Extract the type ident from a field's type (the last segment of the path).
fn type_ident(ty: &syn::Type) -> syn::Result<Ident> {
    if let syn::Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
    {
        return Ok(seg.ident.clone());
    }
    Err(syn::Error::new_spanned(
        ty,
        "expected a simple type path for event field",
    ))
}

/// Expand the Entity derive macro.
///
/// Parses struct fields with `#[event]` attribute to extract event types.
/// If there are no `#[event]` fields, generates a simple entity with no events
/// (the "instant" entity case). If there are event fields, generates event
/// enum, observer, data struct, From impls, HasEventType, EntityData, and
/// ModelComponent.
///
/// Also detects `#[resource_group]`/`#[resource_group(root)]` outer attributes
/// and includes resource group metadata in ModelComponent if present.
///
/// Does NOT re-emit the struct (derive macros append).
pub fn expand_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let vis = &input.vis;
    let name = &input.ident;
    let entity_snake = to_snake_case(name);

    // Parse resource_group from outer attributes
    let resource_group = parse_resource_group_attr(&input);
    let is_root = resource_group.unwrap_or(false);

    // Collect event fields and non-event fields
    let mut event_types: Vec<Ident> = Vec::new();
    let mut regular_fields: Vec<&syn::Field> = Vec::new();

    match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(named) => {
                // Validate all fields are pub
                for field in &named.named {
                    if !matches!(field.vis, syn::Visibility::Public(_)) {
                        return Err(syn::Error::new_spanned(
                            field,
                            "Entity fields must be `pub` — they are part of the generated instrumentation API",
                        ));
                    }
                }
                for field in &named.named {
                    if field_has_attr(field, "event") {
                        event_types.push(type_ident(&field.ty)?);
                    } else {
                        regular_fields.push(field);
                    }
                }
            }
            syn::Fields::Unit => {}
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "Entity derive requires named fields or unit struct",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Entity can only be derived on structs",
            ));
        }
    };

    // Collect attribute defs from regular (non-event) fields
    let attr_defs: Vec<TokenStream> = regular_fields
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

    // When #[resource_group] is present, Entity derive generates both the
    // ResourceGroup trait impl and the ModelComponent contribution, so
    // #[derive(ResourceGroup)] is NOT needed alongside #[derive(Entity)].
    let rg_trait_impl = if resource_group.is_some() {
        quote! {
            impl quent_model::ResourceGroup for #name {
                const IS_ROOT: bool = #is_root;
            }
        }
    } else {
        quote! {}
    };

    let rg_contribution = if resource_group.is_some() {
        quote! {
            builder.add_resource_group(quent_model::ResourceGroupDef {
                name: #entity_snake.to_string(),
                fixed_parent: None,
                is_root: #is_root,
            });
        }
    } else {
        quote! {}
    };

    if event_types.is_empty() && resource_group.is_some() {
        // Resource group entity with no explicit events: generate implicit
        // declaration event for lifecycle management.
        let decl_struct = format_ident!("{}Declaration", name);
        let event_enum = format_ident!("{}Event", name);
        let observer_name = format_ident!("{}Observer", name);
        let data_struct = format_ident!("{}Data", name);
        let observer_method = format_ident!("{}", entity_snake);

        let (decl_fields, decl_attr_defs) = if is_root {
            (
                quote! { pub instance_name: String },
                quote! {
                    quent_model::AttributeDef {
                        name: "instance_name".to_string(),
                        value_type: quent_model::ValueType::String,
                        optional: false,
                    }
                },
            )
        } else {
            (
                quote! {
                    pub instance_name: String,
                    pub parent_group_id: uuid::Uuid
                },
                quote! {
                    quent_model::AttributeDef {
                        name: "instance_name".to_string(),
                        value_type: quent_model::ValueType::String,
                        optional: false,
                    },
                    quent_model::AttributeDef {
                        name: "parent_group_id".to_string(),
                        value_type: quent_model::ValueType::Uuid,
                        optional: false,
                    }
                },
            )
        };

        let output = quote! {
            // Declaration event struct
            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            #vis struct #decl_struct {
                #decl_fields,
            }

            // Event enum
            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            #vis enum #event_enum {
                Declaration(#decl_struct),
            }

            impl From<#decl_struct> for #event_enum {
                fn from(e: #decl_struct) -> Self {
                    #event_enum::Declaration(e)
                }
            }

            // Observer
            #[derive(Clone)]
            #vis struct #observer_name<E>
            where
                E: From<#event_enum> + serde::Serialize + Send + std::fmt::Debug + 'static,
            {
                tx: quent_model::EventSender<E>,
            }

            impl<E> #observer_name<E>
            where
                E: From<#event_enum> + serde::Serialize + Send + std::fmt::Debug + 'static,
            {
                pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                    Self { tx: tx.clone() }
                }

                pub fn #observer_method(&self, id: uuid::Uuid, event: #decl_struct) {
                    self.tx.emit(id, #event_enum::from(event));
                }
            }

            // Data struct for analyzer
            #[derive(Debug, Default)]
            #vis struct #data_struct {
                pub declaration: Option<#decl_struct>,
            }

            // Traits
            impl quent_model::Entity for #name {}

            #rg_trait_impl

            impl quent_model::HasEventType for #name {
                type Event = #event_enum;
            }

            impl quent_model::EntityData for #name {
                type Data = #data_struct;

                fn push(data: &mut Self::Data, event: Self::Event) {
                    match event {
                        #event_enum::Declaration(e) => data.declaration = Some(e),
                    }
                }
            }

            impl quent_model::ModelComponent for #name {
                fn collect(builder: &mut quent_model::ModelBuilder) {
                    builder.add_entity(quent_model::EntityDef {
                        name: #entity_snake.to_string(),
                        attributes: vec![#(#attr_defs,)*],
                        events: vec![
                            quent_model::EntityEventDef {
                                name: "declaration".to_string(),
                                attributes: vec![#decl_attr_defs],
                            }
                        ],
                    });
                    #rg_contribution
                }
            }
        };
        Ok(output)
    } else if event_types.is_empty() {
        // Simple entity (instant) with no events and no resource group
        let output = quote! {
            impl quent_model::Entity for #name {}

            #rg_trait_impl

            impl quent_model::ModelComponent for #name {
                fn collect(builder: &mut quent_model::ModelBuilder) {
                    builder.add_entity(quent_model::EntityDef {
                        name: #entity_snake.to_string(),
                        attributes: vec![#(#attr_defs,)*],
                        events: vec![],
                    });
                    #rg_contribution
                }
            }

        };
        Ok(output)
    } else {
        // Entity with events
        let event_enum = format_ident!("{}Event", name);

        // Generate the event enum
        let event_variants: Vec<TokenStream> =
            event_types.iter().map(|ty| quote! { #ty(#ty) }).collect();

        let from_impls: Vec<TokenStream> = event_types
            .iter()
            .map(|ty| {
                quote! {
                    impl From<#ty> for #event_enum {
                        fn from(e: #ty) -> Self {
                            #event_enum::#ty(e)
                        }
                    }
                }
            })
            .collect();

        // Collect event defs via EventMetadata trait.
        let event_defs: Vec<TokenStream> = event_types
            .iter()
            .map(|ty| {
                quote! {
                    <#ty as quent_model::EventMetadata>::event_def()
                }
            })
            .collect();

        // Generate the data struct: one Option<T> field per event type
        let data_struct = format_ident!("{}Data", name);
        let data_fields: Vec<TokenStream> = event_types
            .iter()
            .map(|ty| {
                let field_name = format_ident!("{}", to_snake_case(ty));
                quote! { pub #field_name: Option<#ty> }
            })
            .collect();

        let data_push_arms: Vec<TokenStream> = event_types
            .iter()
            .map(|ty| {
                let field_name = format_ident!("{}", to_snake_case(ty));
                quote! { #event_enum::#ty(e) => data.#field_name = Some(e) }
            })
            .collect();

        // Generate the observer struct with one method per event type
        let observer_name = format_ident!("{}Observer", name);
        let observer_methods: Vec<TokenStream> = event_types
            .iter()
            .map(|ty| {
                let method_name = format_ident!("{}", to_snake_case(ty));
                quote! {
                    pub fn #method_name(&self, id: uuid::Uuid, event: #ty) {
                        self.tx.emit(id, #event_enum::from(event));
                    }
                }
            })
            .collect();

        let output = quote! {
            // Event enum
            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            #vis enum #event_enum {
                #(#event_variants,)*
            }

            #(#from_impls)*

            // Observer
            #[derive(Clone)]
            #vis struct #observer_name<E>
            where
                E: From<#event_enum> + serde::Serialize + Send + std::fmt::Debug + 'static,
            {
                tx: quent_model::EventSender<E>,
            }

            impl<E> #observer_name<E>
            where
                E: From<#event_enum> + serde::Serialize + Send + std::fmt::Debug + 'static,
            {
                pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                    Self { tx: tx.clone() }
                }

                #(#observer_methods)*
            }

            // Data struct for analyzer
            #[derive(Debug, Default)]
            #vis struct #data_struct {
                #(#data_fields,)*
            }

            // Traits
            impl quent_model::Entity for #name {}

            #rg_trait_impl

            impl quent_model::HasEventType for #name {
                type Event = #event_enum;
            }

            impl quent_model::EntityData for #name {
                type Data = #data_struct;

                fn push(data: &mut Self::Data, event: Self::Event) {
                    match event {
                        #(#data_push_arms,)*
                    }
                }
            }

            impl quent_model::ModelComponent for #name {
                fn collect(builder: &mut quent_model::ModelBuilder) {
                    builder.add_entity(quent_model::EntityDef {
                        name: #entity_snake.to_string(),
                        attributes: vec![#(#attr_defs,)*],
                        events: vec![#(#event_defs,)*],
                    });
                    #rg_contribution
                }
            }

        };

        Ok(output)
    }
}
