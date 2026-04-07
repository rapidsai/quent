// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::util::{parse_resource_group_attr, serde_bound, serde_derives, to_snake_case};

/// If the field type is `EmitOnce<T>`, return the inner `T` ident.
fn extract_emits_once(ty: &syn::Type) -> Option<Ident> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let seg = type_path.path.segments.last()?;
    if seg.ident != "EmitOnce" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };
    let syn::GenericArgument::Type(syn::Type::Path(inner)) = args.args.first()? else {
        return None;
    };
    Some(inner.path.segments.last()?.ident.clone())
}

/// Expand the Entity derive macro.
///
/// Fields with type `EmitOnce<T>` declare event types (T must implement
/// `EventMetadata`). If a struct has named fields but none are `EmitOnce<T>`,
/// it is treated as a self-event entity (must also derive `Event`).
/// Unit structs produce entities with no events. If a resource group
/// unit struct has no fields, an implicit declaration event is generated.
///
/// Also detects `#[resource_group]`/`#[resource_group(root)]` outer attributes
/// and includes resource group metadata in ModelComponent if present.
///
/// Does NOT re-emit the struct (derive macros append).
pub fn expand_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let serde_derives = serde_derives();
    let serde_bound = serde_bound();
    let vis = &input.vis;
    let name = &input.ident;
    let entity_snake = to_snake_case(name);

    // Parse resource_group from outer attributes
    let resource_group = parse_resource_group_attr(&input);
    let is_root = resource_group.unwrap_or(false);

    // Detect EmitOnce<T> fields; if none found on a named-fields struct,
    // the struct itself is a single-event entity (self-event).
    let mut event_types: Vec<Ident> = Vec::new();

    match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(named) => {
                for field in &named.named {
                    if !matches!(field.vis, syn::Visibility::Public(_)) {
                        return Err(syn::Error::new_spanned(
                            field,
                            "Entity fields must be `pub` — they are part of the generated instrumentation API",
                        ));
                    }
                    if let Some(inner) = extract_emits_once(&field.ty) {
                        event_types.push(inner);
                    }
                }
                if event_types.is_empty() {
                    // No EmitOnce<T> fields — self-event entity
                    event_types.push(name.clone());
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
            #[derive(Debug #serde_derives)]
            #vis struct #decl_struct {
                #decl_fields,
            }

            #[derive(Debug #serde_derives)]
            #vis enum #event_enum {
                Declaration(#decl_struct),
            }

            impl From<#decl_struct> for #event_enum {
                fn from(e: #decl_struct) -> Self {
                    #event_enum::Declaration(e)
                }
            }

            #[derive(Clone)]
            #vis struct #observer_name<E>
            where
                E: From<#event_enum> #serde_bound + Send + std::fmt::Debug + 'static,
            {
                tx: quent_model::EventSender<E>,
            }

            impl<E> #observer_name<E>
            where
                E: From<#event_enum> #serde_bound + Send + std::fmt::Debug + 'static,
            {
                pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                    Self { tx: tx.clone() }
                }

                pub fn #observer_method(&self, id: uuid::Uuid, event: #decl_struct) {
                    self.tx.emit(id, #event_enum::from(event));
                }
            }

            #[derive(Debug, Default)]
            #vis struct #data_struct {
                pub declaration: Option<#decl_struct>,
            }

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
            #[derive(Debug #serde_derives)]
            #vis enum #event_enum {
                #(#event_variants,)*
            }

            #(#from_impls)*

            #[derive(Clone)]
            #vis struct #observer_name<E>
            where
                E: From<#event_enum> #serde_bound + Send + std::fmt::Debug + 'static,
            {
                tx: quent_model::EventSender<E>,
            }

            impl<E> #observer_name<E>
            where
                E: From<#event_enum> #serde_bound + Send + std::fmt::Debug + 'static,
            {
                pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                    Self { tx: tx.clone() }
                }

                #(#observer_methods)*
            }

            #[derive(Debug, Default)]
            #vis struct #data_struct {
                #(#data_fields,)*
            }

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
                        events: vec![#(#event_defs,)*],
                    });
                    #rg_contribution
                }
            }

        };

        Ok(output)
    }
}
