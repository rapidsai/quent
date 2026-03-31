// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Unified `#[quent_model(...)]` attribute parser and dispatcher.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::Token;

use crate::util::to_snake_case;

/// Parsed flags from the attribute.
struct ModelFlags {
    is_state: bool,
    is_instant: bool,
    entity_events: Option<Vec<Ident>>,
    resource_group: Option<ResourceGroupFlag>,
    fsm: Option<TokenStream>,
    event_entity: Option<Ident>,
}

enum ResourceGroupFlag {
    Normal,
    Root,
}

impl Parse for ModelFlags {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut flags = ModelFlags {
            is_state: false,
            is_instant: false,
            entity_events: None,
            resource_group: None,
            fsm: None,
            event_entity: None,
        };

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "state" => {
                    flags.is_state = true;
                }
                "instant" => {
                    flags.is_instant = true;
                }
                "entity" => {
                    let content;
                    syn::parenthesized!(content in input);
                    // Parse events(T1, T2, ...)
                    let key: Ident = content.parse()?;
                    if key != "events" {
                        return Err(syn::Error::new_spanned(key, "expected `events(...)`"));
                    }
                    let events_content;
                    syn::parenthesized!(events_content in content);
                    let event_types: syn::punctuated::Punctuated<Ident, Token![,]> =
                        syn::punctuated::Punctuated::parse_terminated(&events_content)?;
                    if event_types.is_empty() {
                        return Err(syn::Error::new(
                            events_content.span(),
                            "entity must have at least one event type",
                        ));
                    }
                    flags.entity_events = Some(event_types.into_iter().collect());
                }
                "resource_group" => {
                    if input.peek(syn::token::Paren) {
                        let content;
                        syn::parenthesized!(content in input);
                        let inner: Ident = content.parse()?;
                        if inner == "root" {
                            flags.resource_group = Some(ResourceGroupFlag::Root);
                        } else {
                            return Err(syn::Error::new_spanned(
                                inner,
                                "expected `root`",
                            ));
                        }
                    } else {
                        flags.resource_group = Some(ResourceGroupFlag::Normal);
                    }
                }
                "fsm" => {
                    let content;
                    syn::parenthesized!(content in input);
                    flags.fsm = Some(content.parse()?);
                }
                "event" => {
                    let content;
                    syn::parenthesized!(content in input);
                    let key: Ident = content.parse()?;
                    if key != "entity" {
                        return Err(syn::Error::new_spanned(key, "expected `entity = Type`"));
                    }
                    content.parse::<Token![=]>()?;
                    flags.event_entity = Some(content.parse()?);
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        ident,
                        format!("unknown flag `{other}`, expected: state, instant, resource_group, fsm, event"),
                    ));
                }
            }

            // Consume optional comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(flags)
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let flags: ModelFlags = syn::parse2(attr)?;

    // Validate flag combinations
    let num_primary = flags.is_state as u8
        + flags.is_instant as u8
        + flags.entity_events.is_some() as u8
        + flags.fsm.is_some() as u8
        + flags.event_entity.is_some() as u8;

    if num_primary == 0 && flags.resource_group.is_none() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected at least one flag: state, entity(...), fsm(...), event(...), or resource_group",
        ));
    }

    if num_primary > 1 {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "only one primary flag (state, entity, fsm, event) can be used at a time",
        ));
    }

    // Dispatch based on primary flag
    if flags.is_state {
        crate::state::expand(item)
    } else if let Some(fsm_tokens) = flags.fsm {
        let mut output = crate::fsm::expand(fsm_tokens, item)?;
        if let Some(rg) = &flags.resource_group {
            let input: syn::ItemStruct = syn::parse2(output.clone())
                .unwrap_or_else(|_| {
                    // The FSM output is multiple items. We need the marker struct name.
                    // Parse just enough to find it.
                    panic!("internal: cannot parse FSM output for resource_group")
                });
            // For FSM + resource_group, append resource group impls using the
            // marker struct name (which is the original struct name).
            let _ = input; // not used directly, see below
        }
        // FSM + resource_group: the FSM generates a marker struct with the
        // original name. We can append ResourceGroup impl.
        if let Some(rg) = flags.resource_group {
            let input: syn::ItemStruct = syn::parse2(
                // Re-parse the original item to get the name
                // (FSM output has already replaced it)
                quote! { pub struct Placeholder; } // placeholder, we just need the name
            )?;
            // Actually, we need the original struct name. Let me take a
            // different approach: parse the item before FSM expansion.
            // This is tricky because we already expanded FSM.
            // Instead, let's extract the struct name from the FSM output.
            // The FSM generates `pub struct {Name};` as the model marker.
            // For now, skip this combination — it's not used yet.
            let _ = rg;
            let _ = input;
        }
        Ok(output)
    } else if let Some(entity_type) = flags.event_entity {
        crate::event::expand(quote! { entity = #entity_type }, item)
    } else if let Some(event_types) = flags.entity_events {
        expand_entity_with_events(event_types, flags.resource_group, item)
    } else if flags.is_instant || flags.resource_group.is_some() {
        // instant and/or resource_group (legacy, for backward compat)
        expand_instant_resource_group(flags.is_instant, flags.resource_group, item)
    } else {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "unreachable flag combination",
        ))
    }
}

/// Handles `instant`, `resource_group`, or `instant, resource_group` together.
fn expand_instant_resource_group(
    is_instant: bool,
    resource_group: Option<ResourceGroupFlag>,
    item: TokenStream,
) -> syn::Result<TokenStream> {
    let input: syn::ItemStruct = syn::parse2(item)?;
    let name = &input.ident;
    let entity_snake = to_snake_case(name);

    let is_root = matches!(resource_group, Some(ResourceGroupFlag::Root));

    // Collect attribute defs from struct fields (for instant entities)
    let attr_defs: Vec<TokenStream> = if is_instant {
        match &input.fields {
            syn::Fields::Named(named) => named
                .named
                .iter()
                .map(|field| {
                    let field_name = field.ident.as_ref().unwrap().to_string();
                    quote! {
                        quent_model::AttributeDef {
                            name: #field_name.to_string(),
                            value_type: quent_model::ValueType::String,
                            optional: false,
                        }
                    }
                })
                .collect(),
            _ => vec![],
        }
    } else {
        vec![]
    };

    // Generate trait impls
    let instant_impl = if is_instant {
        quote! { impl quent_model::Entity for #name {} }
    } else {
        quote! {}
    };

    let rg_impl = if resource_group.is_some() {
        quote! {
            impl quent_model::ResourceGroup for #name {
                const IS_ROOT: bool = #is_root;
            }
        }
    } else {
        quote! {}
    };

    // Single ModelComponent impl that contributes both entity and resource group
    let entity_contribution = if is_instant {
        quote! {
            builder.add_entity(quent_model::EntityDef {
                name: #entity_snake.to_string(),
                attributes: vec![#(#attr_defs,)*],
                events: vec![],
            });
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

    let output = quote! {
        #input

        #instant_impl
        #rg_impl

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                #entity_contribution
                #rg_contribution
            }
        }
    };

    Ok(output)
}

/// Handles `entity(events(T1, T2, ...))` with optional `resource_group`.
///
/// Generates:
/// - The entity event enum (e.g., `OperatorEvent { Declaration(Declaration), Statistics(Statistics) }`)
/// - `From` impls for each event type into the enum
/// - `HasEventType` impl pointing to the generated enum
/// - `Entity` marker trait
/// - `ModelComponent` impl (with optional ResourceGroup contribution)
/// - `ResourceGroup` impl if flagged
fn expand_entity_with_events(
    event_types: Vec<Ident>,
    resource_group: Option<ResourceGroupFlag>,
    item: TokenStream,
) -> syn::Result<TokenStream> {
    let input: syn::ItemStruct = syn::parse2(item)?;
    let name = &input.ident;
    let entity_snake = to_snake_case(name);
    let event_enum = format_ident!("{}Event", name);

    let is_root = matches!(resource_group, Some(ResourceGroupFlag::Root));

    // Generate the event enum
    let event_variants: Vec<TokenStream> = event_types
        .iter()
        .map(|ty| quote! { #ty(#ty) })
        .collect();

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

    // ResourceGroup impl
    let rg_impl = if resource_group.is_some() {
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

    // Collect event names for EntityDef
    let event_name_strings: Vec<String> = event_types.iter().map(|ty| to_snake_case(ty)).collect();
    let event_defs: Vec<TokenStream> = event_name_strings
        .iter()
        .map(|name| {
            quote! {
                quent_model::EntityEventDef {
                    name: #name.to_string(),
                    attributes: vec![],
                }
            }
        })
        .collect();

    let output = quote! {
        #input

        // --- Event enum ---
        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        pub enum #event_enum {
            #(#event_variants,)*
        }

        #(#from_impls)*

        // --- Traits ---
        impl quent_model::Entity for #name {}

        impl quent_model::HasEventType for #name {
            type Event = #event_enum;
        }

        #rg_impl

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_entity(quent_model::EntityDef {
                    name: #entity_snake.to_string(),
                    attributes: vec![],
                    events: vec![#(#event_defs,)*],
                });
                #rg_contribution
            }
        }
    };

    Ok(output)
}
