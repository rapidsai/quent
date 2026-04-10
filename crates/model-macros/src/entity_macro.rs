// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `entity!` proc macro implementation.
//!
//! Two forms:
//!
//! **Self-event entity** — inline fields, the entity IS the event:
//! ```ignore
//! entity! {
//!     Info {
//!         message: String,
//!         source: Option<String>,
//!     }
//! }
//! ```
//!
//! **Multi-event entity** — separate event types listed in `events:`:
//! ```ignore
//! entity! {
//!     FileStats {
//!         events: {
//!             checksum: Checksum,
//!             decompressed: Decompressed,
//!         },
//!     }
//! }
//! ```

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Token, Type, braced};

use crate::util::{resolve_value_type, serde_bound, serde_derives, to_snake_case};

struct InlineField {
    name: Ident,
    ty: Type,
}

struct EventEntry {
    alias: Ident,
    event_type: Ident,
}

enum EntityKind {
    /// Self-event: entity struct has inline fields and IS the event.
    SelfEvent(Vec<InlineField>),
    /// Multi-event: entity references separate event types.
    MultiEvent(Vec<EventEntry>),
    /// Resource group entity with optional parent, attributes, and events.
    ResourceGroup {
        is_root: bool,
        /// Parent group type (e.g., `Cluster`). Field name derived as snake_case,
        /// type wrapped in `Ref<T>`.
        parent_group: Option<Ident>,
        /// External attributes struct (e.g., `Details`).
        attributes: Option<syn::Path>,
        /// Event types (same as MultiEvent).
        events: Vec<EventEntry>,
    },
}

struct EntityInput {
    name: Ident,
    kind: EntityKind,
}

impl Parse for EntityInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let content;
        braced!(content in input);

        if content.is_empty() {
            return Err(syn::Error::new_spanned(
                name,
                "entity! requires either inline fields or an `events:` block",
            ));
        }

        // Peek to determine form from the first keyword.
        let fork = content.fork();
        let first: Ident = fork.parse()?;

        if first == "events" {
            // Multi-event: events: { alias: Type, ... }
            content.parse::<Ident>()?;
            content.parse::<Token![:]>()?;

            let events_content;
            braced!(events_content in content);
            let mut events = Vec::new();
            while !events_content.is_empty() {
                let alias: Ident = events_content.parse()?;
                events_content.parse::<Token![:]>()?;
                let event_type: Ident = events_content.parse()?;
                events.push(EventEntry { alias, event_type });
                if events_content.peek(Token![,]) {
                    events_content.parse::<Token![,]>()?;
                }
            }
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }

            Ok(EntityInput {
                name,
                kind: EntityKind::MultiEvent(events),
            })
        } else if first == "resource_group" {
            let mut is_root = false;
            let mut parent_group = None;
            let mut attributes = None;
            let mut events = Vec::new();

            while !content.is_empty() {
                let kw: Ident = content.parse()?;
                content.parse::<Token![:]>()?;

                match kw.to_string().as_str() {
                    "resource_group" => {
                        let val: Ident = content.parse()?;
                        is_root = val == "root";
                    }
                    "parent_group" => {
                        let ty: Ident = content.parse()?;
                        parent_group = Some(ty);
                    }
                    "attributes" => {
                        let path: syn::Path = content.parse()?;
                        attributes = Some(path);
                    }
                    "events" => {
                        let events_content;
                        braced!(events_content in content);
                        while !events_content.is_empty() {
                            let alias: Ident = events_content.parse()?;
                            events_content.parse::<Token![:]>()?;
                            let event_type: Ident = events_content.parse()?;
                            events.push(EventEntry { alias, event_type });
                            if events_content.peek(Token![,]) {
                                events_content.parse::<Token![,]>()?;
                            }
                        }
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            kw,
                            format!(
                                "unexpected keyword `{other}`, expected `resource_group`, `parent_group`, `attributes`, or `events`"
                            ),
                        ));
                    }
                }

                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }

            Ok(EntityInput {
                name,
                kind: EntityKind::ResourceGroup {
                    is_root,
                    parent_group,
                    attributes,
                    events,
                },
            })
        } else {
            // Self-event: inline fields
            let mut fields = Vec::new();
            while !content.is_empty() {
                let field_name: Ident = content.parse()?;
                content.parse::<Token![:]>()?;
                let ty: Type = content.parse()?;
                fields.push(InlineField {
                    name: field_name,
                    ty,
                });
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }

            Ok(EntityInput {
                name,
                kind: EntityKind::SelfEvent(fields),
            })
        }
    }
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: EntityInput = syn::parse2(input)?;
    match input.kind {
        EntityKind::SelfEvent(fields) => expand_self_event(&input.name, &fields),
        EntityKind::MultiEvent(events) => expand_multi_event(&input.name, &events),
        EntityKind::ResourceGroup {
            is_root,
            parent_group,
            attributes,
            events,
        } => expand_resource_group(
            &input.name,
            is_root,
            parent_group.as_ref(),
            attributes.as_ref(),
            &events,
        ),
    }
}

/// Self-event entity: generates the struct (with serde), EventMetadata,
/// event enum, observer with flat-arg method, data struct, trait impls.
fn expand_self_event(name: &Ident, fields: &[InlineField]) -> syn::Result<TokenStream> {
    let serde_derives = serde_derives();
    let serde_bound = serde_bound();
    let entity_snake = to_snake_case(name);
    let event_enum = format_ident!("{}Event", name);
    let observer_name = format_ident!("{}Observer", name);
    let data_struct = format_ident!("{}Data", name);
    let method_name = format_ident!("{}", entity_snake);

    let field_defs: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let fname = &f.name;
            let fty = &f.ty;
            quote! { pub #fname: #fty }
        })
        .collect();

    let param_defs: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let fname = &f.name;
            let fty = &f.ty;
            quote! { #fname: #fty }
        })
        .collect();

    let field_names: Vec<&Ident> = fields.iter().map(|f| &f.name).collect();

    // AttributeDefs for EventMetadata + ModelComponent
    let attr_defs: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let field_name = f.name.to_string();
            let (value_type_tokens, optional) = resolve_value_type(&f.ty);
            quote! {
                quent_model::AttributeDef {
                    name: #field_name.to_string(),
                    value_type: #value_type_tokens,
                    optional: #optional,
                }
            }
        })
        .collect();

    Ok(quote! {
        #[derive(#serde_derives)]
        pub struct #name {
            #(#field_defs,)*
        }

        impl quent_model::EventMetadata for #name {
            fn event_def() -> quent_model::EntityEventDef {
                quent_model::EntityEventDef {
                    name: #entity_snake.to_string(),
                    attributes: vec![#(#attr_defs,)*],
                }
            }
        }

        #[derive(#serde_derives)]
        pub enum #event_enum {
            #name(#name),
        }

        impl From<#name> for #event_enum {
            fn from(e: #name) -> Self {
                #event_enum::#name(e)
            }
        }

        #[derive(Clone)]
        pub struct #observer_name<E>
        where
            E: From<#event_enum> #serde_bound + Send + 'static,
        {
            tx: quent_model::EventSender<E>,
        }

        impl<E> #observer_name<E>
        where
            E: From<#event_enum> #serde_bound + Send + 'static,
        {
            pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                Self { tx: tx.clone() }
            }

            pub fn #method_name(&self, id: uuid::Uuid, #(#param_defs,)*) {
                self.tx.emit(id, #event_enum::from(#name { #(#field_names,)* }));
            }
        }

        #[derive(Default)]
        pub struct #data_struct {
            pub #method_name: Option<#name>,
        }

        impl quent_model::Entity for #name {}

        impl quent_model::HasEventType for #name {
            type Event = #event_enum;
        }

        impl quent_model::EntityData for #name {
            type Data = #data_struct;

            fn push(data: &mut Self::Data, event: Self::Event) {
                match event {
                    #event_enum::#name(e) => data.#method_name = Some(e),
                }
            }
        }

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_entity(quent_model::EntityDef {
                    name: #entity_snake.to_string(),
                    events: vec![
                        <#name as quent_model::EventMetadata>::event_def(),
                    ],
                });
            }
        }
    })
}

/// Multi-event entity: generates event enum, handle with per-event methods,
/// observer with create(), data struct, trait impls.
fn expand_multi_event(name: &Ident, events: &[EventEntry]) -> syn::Result<TokenStream> {
    let serde_derives = serde_derives();
    let serde_bound = serde_bound();
    let entity_snake = to_snake_case(name);
    let event_enum = format_ident!("{}Event", name);
    let handle_name = format_ident!("{}Handle", name);
    let observer_name = format_ident!("{}Observer", name);
    let data_struct = format_ident!("{}Data", name);

    let event_types: Vec<&Ident> = events.iter().map(|e| &e.event_type).collect();
    let event_aliases: Vec<&Ident> = events.iter().map(|e| &e.alias).collect();

    let event_variants: Vec<TokenStream> =
        event_types.iter().map(|ty| quote! { #ty(#ty) }).collect();

    let from_impls: Vec<TokenStream> = event_types
        .iter()
        .map(|ty| {
            quote! {
                impl From<#ty> for #event_enum {
                    fn from(e: #ty) -> Self { #event_enum::#ty(e) }
                }
            }
        })
        .collect();

    let event_defs: Vec<TokenStream> = event_types
        .iter()
        .map(|ty| quote! { <#ty as quent_model::EventMetadata>::event_def() })
        .collect();

    let data_fields: Vec<TokenStream> = events
        .iter()
        .map(|e| {
            let alias = &e.alias;
            let ty = &e.event_type;
            quote! { pub #alias: Option<#ty> }
        })
        .collect();

    let data_push_arms: Vec<TokenStream> = events
        .iter()
        .map(|e| {
            let alias = &e.alias;
            let ty = &e.event_type;
            quote! { #event_enum::#ty(e) => data.#alias = Some(e) }
        })
        .collect();

    let handle_methods: Vec<TokenStream> = events
        .iter()
        .map(|e| {
            let alias = &e.alias;
            let ty = &e.event_type;
            quote! {
                pub fn #alias(&self, event: #ty) {
                    self.tx.emit(self.id, #event_enum::from(event));
                }
            }
        })
        .collect();

    // Single-event multi-event: direct observer method instead of handle
    let (observer_and_handle, _) = if events.len() == 1 {
        let alias = &event_aliases[0];
        let ty = &event_types[0];
        (
            quote! {
                #[derive(Clone)]
                pub struct #observer_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    tx: quent_model::EventSender<E>,
                }

                impl<E> #observer_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                        Self { tx: tx.clone() }
                    }

                    pub fn #alias(&self, id: uuid::Uuid, event: #ty) {
                        self.tx.emit(id, #event_enum::from(event));
                    }
                }
            },
            false,
        )
    } else {
        (
            quote! {
                pub struct #handle_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    id: uuid::Uuid,
                    tx: quent_model::EventSender<E>,
                }

                impl<E> #handle_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    pub fn uuid(&self) -> uuid::Uuid { self.id }

                    #(#handle_methods)*
                }

                #[derive(Clone)]
                pub struct #observer_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    tx: quent_model::EventSender<E>,
                }

                impl<E> #observer_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                        Self { tx: tx.clone() }
                    }

                    pub fn create(&self, id: uuid::Uuid) -> #handle_name<E> {
                        #handle_name { id, tx: self.tx.clone() }
                    }
                }
            },
            true,
        )
    };

    // The entity is a unit struct — no user-visible fields.
    Ok(quote! {
        pub struct #name;

        #[derive(#serde_derives)]
        pub enum #event_enum {
            #(#event_variants,)*
        }

        #(#from_impls)*

        #observer_and_handle

        #[derive(Default)]
        pub struct #data_struct {
            #(#data_fields,)*
        }

        impl quent_model::Entity for #name {}

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
            }
        }
    })
}

/// Resource group entity: generates declaration struct, event enum, observer,
/// ResourceGroup trait, and ModelComponent.
///
/// When no user event has the alias `declaration`, an auto-generated
/// `{Name}Declaration` struct (with `instance_name` and optional parent fields)
/// is emitted. When a user event IS aliased `declaration`, that event type is
/// used directly — no extra declaration struct is generated.
fn expand_resource_group(
    name: &Ident,
    is_root: bool,
    parent_group: Option<&Ident>,
    attributes: Option<&syn::Path>,
    events: &[EventEntry],
) -> syn::Result<TokenStream> {
    // Check if a user event already serves as the declaration.
    let has_user_decl = events.iter().any(|e| e.alias == "declaration");

    if has_user_decl {
        expand_resource_group_user_decl(name, is_root, events)
    } else {
        expand_resource_group_auto_decl(name, is_root, parent_group, attributes, events)
    }
}

/// Resource group where a user event with alias `declaration` serves as the
/// declaration event. No auto-generated `{Name}Declaration` struct is emitted.
fn expand_resource_group_user_decl(
    name: &Ident,
    is_root: bool,
    events: &[EventEntry],
) -> syn::Result<TokenStream> {
    let serde_derives = serde_derives();
    let serde_bound = serde_bound();
    let entity_snake = to_snake_case(name);
    let event_enum = format_ident!("{}Event", name);
    let observer_name = format_ident!("{}Observer", name);
    let data_struct = format_ident!("{}Data", name);

    let event_types: Vec<&Ident> = events.iter().map(|e| &e.event_type).collect();

    let event_variants: Vec<TokenStream> =
        event_types.iter().map(|ty| quote! { #ty(#ty) }).collect();

    let from_impls: Vec<TokenStream> = event_types
        .iter()
        .map(|ty| {
            quote! {
                impl From<#ty> for #event_enum {
                    fn from(e: #ty) -> Self { #event_enum::#ty(e) }
                }
            }
        })
        .collect();

    let data_fields: Vec<TokenStream> = events
        .iter()
        .map(|e| {
            let alias = &e.alias;
            let ty = &e.event_type;
            quote! { pub #alias: Option<#ty> }
        })
        .collect();

    let data_push_arms: Vec<TokenStream> = events
        .iter()
        .map(|e| {
            let alias = &e.alias;
            let ty = &e.event_type;
            quote! { #event_enum::#ty(e) => data.#alias = Some(e) }
        })
        .collect();

    let event_defs: Vec<TokenStream> = event_types
        .iter()
        .map(|ty| quote! { <#ty as quent_model::EventMetadata>::event_def() })
        .collect();

    // Generate observer + handle
    let (observer_and_handle, _) = if events.len() == 1 {
        let alias = &events[0].alias;
        let ty = &events[0].event_type;
        (
            quote! {
                #[derive(Clone)]
                pub struct #observer_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    tx: quent_model::EventSender<E>,
                }

                impl<E> #observer_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                        Self { tx: tx.clone() }
                    }

                    pub fn #alias(&self, id: uuid::Uuid, event: #ty) {
                        self.tx.emit(id, #event_enum::from(event));
                    }
                }
            },
            false,
        )
    } else {
        let handle_name = format_ident!("{}Handle", name);

        let handle_methods: Vec<TokenStream> = events
            .iter()
            .map(|e| {
                let alias = &e.alias;
                let ty = &e.event_type;
                quote! {
                    pub fn #alias(&self, event: #ty) {
                        self.tx.emit(self.id, #event_enum::from(event));
                    }
                }
            })
            .collect();

        (
            quote! {
                pub struct #handle_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    id: uuid::Uuid,
                    tx: quent_model::EventSender<E>,
                }

                impl<E> #handle_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    pub fn uuid(&self) -> uuid::Uuid { self.id }
                    #(#handle_methods)*
                }

                #[derive(Clone)]
                pub struct #observer_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    tx: quent_model::EventSender<E>,
                }

                impl<E> #observer_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                        Self { tx: tx.clone() }
                    }

                    pub fn create(&self, id: uuid::Uuid) -> #handle_name<E> {
                        #handle_name { id, tx: self.tx.clone() }
                    }
                }
            },
            true,
        )
    };

    Ok(quote! {
        pub struct #name;

        #[derive(#serde_derives)]
        pub enum #event_enum {
            #(#event_variants,)*
        }

        #(#from_impls)*

        #observer_and_handle

        #[derive(Default)]
        pub struct #data_struct {
            #(#data_fields,)*
        }

        impl quent_model::Entity for #name {}

        impl quent_model::ResourceGroup for #name {
            const IS_ROOT: bool = #is_root;
        }

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
                builder.add_resource_group(quent_model::ResourceGroupDef {
                    name: #entity_snake.to_string(),
                    fixed_parent: None,
                    is_root: #is_root,
                });
            }
        }
    })
}

/// Resource group with an auto-generated `{Name}Declaration` struct.
fn expand_resource_group_auto_decl(
    name: &Ident,
    is_root: bool,
    parent_group: Option<&Ident>,
    attributes: Option<&syn::Path>,
    events: &[EventEntry],
) -> syn::Result<TokenStream> {
    let serde_derives = serde_derives();
    let serde_bound = serde_bound();
    let entity_snake = to_snake_case(name);
    let decl_struct = format_ident!("{}Declaration", name);
    let event_enum = format_ident!("{}Event", name);
    let observer_name = format_ident!("{}Observer", name);
    let data_struct = format_ident!("{}Data", name);
    let observer_method = format_ident!("{}", entity_snake);

    // Build declaration fields, observer params, and attribute defs
    let mut decl_fields = vec![quote! { pub instance_name: String }];
    let mut decl_attr_defs = vec![quote! {
        quent_model::AttributeDef {
            name: "instance_name".to_string(),
            value_type: quent_model::ValueType::String,
            optional: false,
        }
    }];
    let mut observer_params: Vec<TokenStream> = Vec::new();
    let mut decl_field_inits: Vec<TokenStream> = Vec::new();

    if let Some(parent_ty) = parent_group {
        let field_name = format_ident!("{}", to_snake_case(parent_ty));
        decl_fields.push(quote! { pub #field_name: quent_model::Ref<#parent_ty> });
        decl_attr_defs.push(quote! {
            quent_model::AttributeDef {
                name: stringify!(#field_name).to_string(),
                value_type: quent_model::ValueType::Ref(stringify!(#parent_ty).to_string()),
                optional: false,
            }
        });
        observer_params.push(quote! { #field_name: quent_model::Ref<#parent_ty> });
        decl_field_inits.push(quote! { #field_name, });
    } else if !is_root {
        decl_fields.push(quote! { pub parent_group_id: uuid::Uuid });
        decl_attr_defs.push(quote! {
            quent_model::AttributeDef {
                name: "parent_group_id".to_string(),
                value_type: quent_model::ValueType::Uuid,
                optional: false,
            }
        });
        observer_params.push(quote! { parent_group_id: uuid::Uuid });
        decl_field_inits.push(quote! { parent_group_id, });
    }

    if let Some(attrs_path) = attributes {
        let field_name = format_ident!(
            "{}",
            to_snake_case(
                &attrs_path
                    .segments
                    .last()
                    .expect("attributes path must have segments")
                    .ident
            )
        );
        let serde_flatten = if cfg!(feature = "serde") {
            quote! { #[serde(flatten)] }
        } else {
            quote! {}
        };
        decl_fields.push(quote! {
            #serde_flatten
            pub #field_name: #attrs_path
        });
        observer_params.push(quote! { #field_name: #attrs_path });
        decl_field_inits.push(quote! { #field_name, });
    }

    let attr_defs_expr = if let Some(attrs_path) = attributes {
        quote! {
            {
                let mut defs = vec![#(#decl_attr_defs,)*];
                defs.extend(<#attrs_path as quent_model::EventMetadata>::event_def().attributes);
                defs
            }
        }
    } else {
        quote! { vec![#(#decl_attr_defs,)*] }
    };

    // Event variants: Declaration is always present, plus user events.
    let event_types: Vec<&Ident> = events.iter().map(|e| &e.event_type).collect();

    let extra_variants: Vec<TokenStream> =
        event_types.iter().map(|ty| quote! { #ty(#ty) }).collect();

    let extra_from_impls: Vec<TokenStream> = event_types
        .iter()
        .map(|ty| {
            quote! {
                impl From<#ty> for #event_enum {
                    fn from(e: #ty) -> Self { #event_enum::#ty(e) }
                }
            }
        })
        .collect();

    let extra_data_fields: Vec<TokenStream> = events
        .iter()
        .map(|e| {
            let alias = &e.alias;
            let ty = &e.event_type;
            quote! { pub #alias: Option<#ty> }
        })
        .collect();

    let extra_push_arms: Vec<TokenStream> = events
        .iter()
        .map(|e| {
            let alias = &e.alias;
            let ty = &e.event_type;
            quote! { #event_enum::#ty(e) => data.#alias = Some(e) }
        })
        .collect();

    let extra_event_defs: Vec<TokenStream> = event_types
        .iter()
        .map(|ty| quote! { <#ty as quent_model::EventMetadata>::event_def() })
        .collect();

    // Handle + observer pattern: if there are events, generate a handle
    let (handle_code, observer_return) = if events.is_empty() {
        // No events: observer method returns Uuid directly
        (
            quote! {},
            quote! {
                pub fn #observer_method(
                    &self,
                    id: uuid::Uuid,
                    instance_name: &str,
                    #(#observer_params,)*
                ) -> uuid::Uuid {
                    let event = #decl_struct {
                        instance_name: instance_name.to_string(),
                        #(#decl_field_inits)*
                    };
                    self.tx.emit(id, #event_enum::from(event));
                    id
                }
            },
        )
    } else {
        // Events present: observer returns a handle
        let handle_name = format_ident!("{}Handle", name);

        let handle_methods: Vec<TokenStream> = events
            .iter()
            .map(|e| {
                let alias = &e.alias;
                let ty = &e.event_type;
                quote! {
                    pub fn #alias(&self, event: #ty) {
                        self.tx.emit(self.id, #event_enum::from(event));
                    }
                }
            })
            .collect();

        (
            quote! {
                pub struct #handle_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    id: uuid::Uuid,
                    tx: quent_model::EventSender<E>,
                }

                impl<E> #handle_name<E>
                where
                    E: From<#event_enum> #serde_bound + Send + 'static,
                {
                    pub fn uuid(&self) -> uuid::Uuid { self.id }
                    #(#handle_methods)*
                }
            },
            quote! {
                pub fn #observer_method(
                    &self,
                    id: uuid::Uuid,
                    instance_name: &str,
                    #(#observer_params,)*
                ) -> #handle_name<E> {
                    let event = #decl_struct {
                        instance_name: instance_name.to_string(),
                        #(#decl_field_inits)*
                    };
                    self.tx.emit(id, #event_enum::from(event));
                    #handle_name { id, tx: self.tx.clone() }
                }
            },
        )
    };

    Ok(quote! {
        pub struct #name;

        #[derive(#serde_derives)]
        pub struct #decl_struct {
            #(#decl_fields,)*
        }

        #[derive(#serde_derives)]
        pub enum #event_enum {
            Declaration(#decl_struct),
            #(#extra_variants,)*
        }

        impl From<#decl_struct> for #event_enum {
            fn from(e: #decl_struct) -> Self {
                #event_enum::Declaration(e)
            }
        }

        #(#extra_from_impls)*

        #handle_code

        #[derive(Clone)]
        pub struct #observer_name<E>
        where
            E: From<#event_enum> #serde_bound + Send + 'static,
        {
            tx: quent_model::EventSender<E>,
        }

        impl<E> #observer_name<E>
        where
            E: From<#event_enum> #serde_bound + Send + 'static,
        {
            pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                Self { tx: tx.clone() }
            }

            #observer_return
        }

        #[derive(Default)]
        pub struct #data_struct {
            pub declaration: Option<#decl_struct>,
            #(#extra_data_fields,)*
        }

        impl quent_model::Entity for #name {}

        impl quent_model::ResourceGroup for #name {
            const IS_ROOT: bool = #is_root;
        }

        impl quent_model::HasEventType for #name {
            type Event = #event_enum;
        }

        impl quent_model::EntityData for #name {
            type Data = #data_struct;

            fn push(data: &mut Self::Data, event: Self::Event) {
                match event {
                    #event_enum::Declaration(e) => data.declaration = Some(e),
                    #(#extra_push_arms,)*
                }
            }
        }

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_entity(quent_model::EntityDef {
                    name: #entity_snake.to_string(),
                    events: vec![
                        quent_model::EntityEventDef {
                            name: concat!(#entity_snake, "_declaration").to_string(),
                            attributes: #attr_defs_expr,
                        },
                        #(#extra_event_defs,)*
                    ],
                });
                builder.add_resource_group(quent_model::ResourceGroupDef {
                    name: #entity_snake.to_string(),
                    fixed_parent: None,
                    is_root: #is_root,
                });
            }
        }
    })
}
