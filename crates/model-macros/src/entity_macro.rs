// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `entity!` proc macro implementation.
//!
//! ```ignore
//! // Self-event: attributes block, entity IS the event
//! entity! {
//!     Info {
//!         attributes: {
//!             message: String,
//!             source: Option<String>,
//!         },
//!     }
//! }
//!
//! // Multi-event: separate event types
//! entity! {
//!     FileStats {
//!         events: {
//!             checksum: Checksum,
//!             decompressed: Decompressed,
//!         },
//!     }
//! }
//!
//! // Resource group with auto-declaration
//! entity! {
//!     Worker: ResourceGroup<Parent = Cluster> {
//!         attributes: {
//!             details: Details,
//!         },
//!     }
//! }
//!
//! // Resource group with events + declaration marker
//! entity! {
//!     Engine: ResourceGroup {
//!         declaration: init,
//!         events: {
//!             init: Init,
//!             exit: Exit,
//!         },
//!     }
//! }
//!
//! // Root resource group
//! entity! {
//!     Cluster: ResourceGroup<Root = true> {}
//! }
//! ```

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Token, Type, braced};

use crate::util::{
    resolve_value_type, serde_bound, serde_crate_attr, serde_derives, to_snake_case,
};

struct InlineField {
    name: Ident,
    ty: Type,
}

struct EventEntry {
    alias: Ident,
    event_type: Ident,
}

/// Resource group metadata parsed from `: ResourceGroup<...>`.
struct ResourceGroupMeta {
    is_root: bool,
    parent_type: Option<Ident>,
}

enum EntityKind {
    /// Self-event: `attributes: { field: Type, ... }` — entity IS the event.
    SelfEvent(Vec<InlineField>),
    /// Multi-event: `events: { alias: Type, ... }`.
    MultiEvent(Vec<EventEntry>),
    /// Resource group with inline declaration attributes.
    ResourceGroupAttrs {
        meta: ResourceGroupMeta,
        fields: Vec<InlineField>,
    },
    /// Resource group with events and a declaration marker.
    ResourceGroupEvents {
        meta: ResourceGroupMeta,
        declaration: Option<Ident>,
        events: Vec<EventEntry>,
    },
}

struct EntityInput {
    user_attrs: Vec<syn::Attribute>,
    name: Ident,
    kind: EntityKind,
}

/// Parse optional `: ResourceGroup<...>` after the entity name.
fn parse_resource_group_meta(input: ParseStream) -> syn::Result<Option<ResourceGroupMeta>> {
    if !input.peek(Token![:]) {
        return Ok(None);
    }
    input.parse::<Token![:]>()?;
    let rg: Ident = input.parse()?;
    if rg != "ResourceGroup" {
        return Err(syn::Error::new_spanned(
            rg,
            "expected `ResourceGroup` after `:`",
        ));
    }

    let mut is_root = false;
    let mut parent_type = None;

    if input.peek(Token![<]) {
        input.parse::<Token![<]>()?;
        while !input.peek(Token![>]) {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "Root" => {
                    let val: syn::LitBool = input.parse()?;
                    is_root = val.value;
                }
                "Parent" => {
                    let ty: Ident = input.parse()?;
                    parent_type = Some(ty);
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        key,
                        format!("unexpected parameter `{other}`, expected `Root` or `Parent`"),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        input.parse::<Token![>]>()?;
    }

    Ok(Some(ResourceGroupMeta {
        is_root,
        parent_type,
    }))
}

/// Parse the body keywords: `attributes:`, `events:`, `declaration:`.
fn parse_body(
    content: syn::parse::ParseBuffer,
    rg_meta: Option<ResourceGroupMeta>,
) -> syn::Result<EntityKind> {
    let mut attributes: Vec<InlineField> = Vec::new();
    let mut events: Vec<EventEntry> = Vec::new();
    let mut declaration: Option<Ident> = None;
    let mut has_attributes = false;
    let mut has_events = false;

    while !content.is_empty() {
        let kw: Ident = content.parse()?;
        content.parse::<Token![:]>()?;

        match kw.to_string().as_str() {
            "attributes" => {
                has_attributes = true;
                let fields_content;
                braced!(fields_content in content);
                while !fields_content.is_empty() {
                    let field_name: Ident = fields_content.parse()?;
                    fields_content.parse::<Token![:]>()?;
                    let ty: Type = fields_content.parse()?;
                    attributes.push(InlineField {
                        name: field_name,
                        ty,
                    });
                    if fields_content.peek(Token![,]) {
                        fields_content.parse::<Token![,]>()?;
                    }
                }
            }
            "events" => {
                has_events = true;
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
            "declaration" => {
                let alias: Ident = content.parse()?;
                declaration = Some(alias);
            }
            other => {
                return Err(syn::Error::new_spanned(
                    kw,
                    format!(
                        "unexpected keyword `{other}`, expected `attributes`, `events`, or `declaration`"
                    ),
                ));
            }
        }

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }

    match rg_meta {
        Some(meta) => {
            if has_events && declaration.is_none() {
                Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "resource group with `events` requires `declaration: <alias>` to mark which event carries the resource group identity",
                ))
            } else if has_events {
                if let Some(ref decl) = declaration
                    && !events.iter().any(|e| e.alias == *decl)
                {
                    return Err(syn::Error::new_spanned(
                        decl,
                        format!("declaration alias `{}` does not match any event", decl),
                    ));
                }
                Ok(EntityKind::ResourceGroupEvents {
                    meta,
                    declaration,
                    events,
                })
            } else {
                Ok(EntityKind::ResourceGroupAttrs {
                    meta,
                    fields: attributes,
                })
            }
        }
        None => {
            if declaration.is_some() {
                Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "`declaration` is only valid on resource group entities (use `: ResourceGroup`)",
                ))
            } else if has_events && has_attributes {
                Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "cannot combine `attributes` and `events` on a non-resource-group entity; use either `attributes: { ... }` (self-event) or `events: { ... }` (multi-event)",
                ))
            } else if has_events {
                Ok(EntityKind::MultiEvent(events))
            } else if has_attributes {
                Ok(EntityKind::SelfEvent(attributes))
            } else {
                Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "entity! requires `attributes: { ... }` or `events: { ... }`",
                ))
            }
        }
    }
}

impl Parse for EntityInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let user_attrs = input.call(syn::Attribute::parse_outer)?;
        let name: Ident = input.parse()?;
        let rg_meta = parse_resource_group_meta(input)?;

        let content;
        braced!(content in input);

        // Empty body is valid for resource groups (e.g., `Cluster: ResourceGroup<Root = true> {}`)
        if content.is_empty() {
            return match rg_meta {
                Some(meta) => Ok(EntityInput {
                    user_attrs,
                    name,
                    kind: EntityKind::ResourceGroupAttrs {
                        meta,
                        fields: Vec::new(),
                    },
                }),
                None => Err(syn::Error::new_spanned(
                    name,
                    "entity! requires `attributes: { ... }` or `events: { ... }`",
                )),
            };
        }

        let kind = parse_body(content, rg_meta)?;
        Ok(EntityInput {
            user_attrs,
            name,
            kind,
        })
    }
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: EntityInput = syn::parse2(input)?;
    let ua = &input.user_attrs;
    match input.kind {
        EntityKind::SelfEvent(fields) => expand_self_event(&input.name, &fields, ua),
        EntityKind::MultiEvent(events) => expand_multi_event(&input.name, &events, ua),
        EntityKind::ResourceGroupAttrs { meta, fields } => {
            expand_rg_attrs(&input.name, &meta, &fields, ua)
        }
        EntityKind::ResourceGroupEvents {
            meta,
            declaration,
            events,
        } => expand_rg_events(&input.name, &meta, declaration.as_ref(), &events, ua),
    }
}

// Shared helpers

/// Common identifiers derived from the entity name.
struct EntityIdents {
    serde_derives: TokenStream,
    serde_crate_attr: TokenStream,
    serde_bound: TokenStream,
    entity_snake: String,
    event_enum: Ident,
    observer_name: Ident,
    data_struct: Ident,
}

impl EntityIdents {
    fn new(name: &Ident) -> Self {
        let entity_snake = to_snake_case(name);
        Self {
            serde_derives: serde_derives(),
            serde_crate_attr: serde_crate_attr(),
            serde_bound: serde_bound(),
            event_enum: format_ident!("{}Event", name),
            observer_name: format_ident!("{}Observer", name),
            data_struct: format_ident!("{}Data", name),
            entity_snake,
        }
    }
}

/// Precomputed token streams for event enum variants, From impls, data
/// fields, push arms, and event metadata defs.
struct EventCodegen {
    event_variants: Vec<TokenStream>,
    from_impls: Vec<TokenStream>,
    data_fields: Vec<TokenStream>,
    data_push_arms: Vec<TokenStream>,
    event_defs: Vec<TokenStream>,
}

fn codegen_events(events: &[EventEntry], event_enum: &Ident) -> EventCodegen {
    let event_variants = events
        .iter()
        .map(|e| {
            let variant = format_ident!("{}", crate::util::to_pascal_case(&e.alias.to_string()));
            let ty = &e.event_type;
            quote! { #variant(#ty) }
        })
        .collect();

    // Only generate From<T> for event types that appear exactly once.
    // Duplicate types are ambiguous — use the handle methods instead.
    let type_counts: std::collections::HashMap<String, usize> = {
        let mut counts = std::collections::HashMap::new();
        for e in events {
            *counts.entry(e.event_type.to_string()).or_insert(0) += 1;
        }
        counts
    };
    let from_impls: Vec<TokenStream> = events
        .iter()
        .filter(|e| type_counts[&e.event_type.to_string()] == 1)
        .map(|e| {
            let variant = format_ident!("{}", crate::util::to_pascal_case(&e.alias.to_string()));
            let ty = &e.event_type;
            quote! {
                impl From<#ty> for #event_enum {
                    fn from(e: #ty) -> Self { #event_enum::#variant(e) }
                }
            }
        })
        .collect();

    let data_fields = events
        .iter()
        .map(|e| {
            let alias = &e.alias;
            let ty = &e.event_type;
            quote! { pub #alias: Option<#ty> }
        })
        .collect();

    let data_push_arms = events
        .iter()
        .map(|e| {
            let variant = format_ident!("{}", crate::util::to_pascal_case(&e.alias.to_string()));
            let alias = &e.alias;
            quote! { #event_enum::#variant(e) => data.#alias = Some(e) }
        })
        .collect();

    let event_defs = events
        .iter()
        .map(|e| {
            let ty = &e.event_type;
            quote! { <#ty as quent_model::EventMetadata>::event_def() }
        })
        .collect();

    EventCodegen {
        event_variants,
        from_impls,
        data_fields,
        data_push_arms,
        event_defs,
    }
}

/// Generate observer (+ optional handle) for event-based entities.
///
/// Single event: observer with a direct method.
/// Multiple events: handle with per-event methods + observer with `create()`.
fn gen_observer_and_handle(name: &Ident, events: &[EventEntry], ids: &EntityIdents) -> TokenStream {
    let event_enum = &ids.event_enum;
    let observer_name = &ids.observer_name;
    let serde_bound = &ids.serde_bound;
    let entity_snake = &ids.entity_snake;

    let doc_observer = format!(
        "Observer for `{name}` events.\n\n\
         An observer emits events for a model component. Obtain one from the \
         instrumentation context via `{entity_snake}_observer()`.\n\n\
         The type parameter `E` is the model's top-level event enum, allowing \
         the same component to be reused across different models."
    );

    if events.len() == 1 {
        let alias = &events[0].alias;
        let variant = format_ident!(
            "{}",
            crate::util::to_pascal_case(&events[0].alias.to_string())
        );
        let ty = &events[0].event_type;
        let doc_method = format!("Emit a {name} event.");
        quote! {
            #[doc = #doc_observer]
            #[doc(alias = "observer")]
            #[derive(Clone)]
            pub struct #observer_name<E>
            where E: From<#event_enum> #serde_bound + Send + 'static,
            {
                tx: quent_model::EventSender<E>,
            }

            impl<E> #observer_name<E>
            where E: From<#event_enum> #serde_bound + Send + 'static,
            {
                pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                    Self { tx: tx.clone() }
                }

                #[doc = #doc_method]
                pub fn #alias(&self, id: quent_model::uuid::Uuid, event: #ty) {
                    self.tx.emit(id, #event_enum::#variant(event));
                }
            }
        }
    } else {
        let handle_name = format_ident!("{}Handle", name);
        let doc_handle = format!("Handle for an active {name} entity instance.");
        let doc_handle_uuid = format!("Returns the UUID of this {name} entity.");
        let doc_create = format!("Create a new {name} handle for the given entity UUID.");

        let handle_methods: Vec<TokenStream> = events
            .iter()
            .map(|e| {
                let alias = &e.alias;
                let variant =
                    format_ident!("{}", crate::util::to_pascal_case(&e.alias.to_string()));
                let ty = &e.event_type;
                let doc_method = format!("Emit the `{}` event.", alias);
                quote! {
                    #[doc = #doc_method]
                    pub fn #alias(&self, event: #ty) {
                        self.tx.emit(self.id, #event_enum::#variant(event));
                    }
                }
            })
            .collect();

        quote! {
            #[doc = #doc_handle]
            #[doc(alias = "handle")]
            pub struct #handle_name<E>
            where E: From<#event_enum> #serde_bound + Send + 'static,
            {
                id: quent_model::uuid::Uuid,
                tx: quent_model::EventSender<E>,
            }

            impl<E> #handle_name<E>
            where E: From<#event_enum> #serde_bound + Send + 'static,
            {
                #[doc = #doc_handle_uuid]
                pub fn uuid(&self) -> quent_model::uuid::Uuid { self.id }
                #(#handle_methods)*
            }

            #[doc = #doc_observer]
            #[doc(alias = "observer")]
            #[derive(Clone)]
            pub struct #observer_name<E>
            where E: From<#event_enum> #serde_bound + Send + 'static,
            {
                tx: quent_model::EventSender<E>,
            }

            impl<E> #observer_name<E>
            where E: From<#event_enum> #serde_bound + Send + 'static,
            {
                pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                    Self { tx: tx.clone() }
                }

                #[doc = #doc_create]
                pub fn create(&self, id: quent_model::uuid::Uuid) -> #handle_name<E> {
                    #handle_name { id, tx: self.tx.clone() }
                }
            }
        }
    }
}

// Self-event entity

fn expand_self_event(
    name: &Ident,
    fields: &[InlineField],
    user_attrs: &[syn::Attribute],
) -> syn::Result<TokenStream> {
    let ids = EntityIdents::new(name);
    let serde_derives = &ids.serde_derives;
    let serde_crate_attr = &ids.serde_crate_attr;
    let serde_bound = &ids.serde_bound;
    let entity_snake = &ids.entity_snake;
    let event_enum = &ids.event_enum;
    let observer_name = &ids.observer_name;
    let data_struct = &ids.data_struct;
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

    let doc_struct = format!("`{name}` self-event entity.");
    let doc_event = format!("Events emitted by `{name}`.");
    let doc_observer = format!(
        "Observer for `{name}` events.\n\n\
         An observer emits events for a model component. Obtain one from the \
         instrumentation context via `{entity_snake}_observer()`.\n\n\
         The type parameter `E` is the model's top-level event enum, allowing \
         the same component to be reused across different models."
    );
    let doc_observer_method = format!("Emit a `{name}` event.");
    let doc_data =
        format!("Analyzer data for {name} \u{2014} stores one `Option<T>` per event type.");

    Ok(quote! {
        #(#user_attrs)*
        #[doc = #doc_struct]
        #[derive(#serde_derives)]
        #serde_crate_attr
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

        #[doc = #doc_event]
        #[doc(alias = "event")]
        #[derive(#serde_derives)]
        #serde_crate_attr
        pub enum #event_enum {
            #name(#name),
        }

        impl From<#name> for #event_enum {
            fn from(e: #name) -> Self { #event_enum::#name(e) }
        }

        #[doc = #doc_observer]
            #[doc(alias = "observer")]
        #[derive(Clone)]
        pub struct #observer_name<E>
        where E: From<#event_enum> #serde_bound + Send + 'static,
        {
            tx: quent_model::EventSender<E>,
        }

        impl<E> #observer_name<E>
        where E: From<#event_enum> #serde_bound + Send + 'static,
        {
            pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                Self { tx: tx.clone() }
            }

            #[doc = #doc_observer_method]
            #[doc(alias = "observer")]
            pub fn #method_name(&self, id: quent_model::uuid::Uuid, #(#param_defs,)*) {
                self.tx.emit(id, #event_enum::from(#name { #(#field_names,)* }));
            }
        }

        #[doc = #doc_data]
        #[doc(alias = "data")]
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
                        module_path: module_path!().to_string(),
                    events: vec![<#name as quent_model::EventMetadata>::event_def()],
                });
            }
        }
    })
}

// Multi-event entity

fn expand_multi_event(
    name: &Ident,
    events: &[EventEntry],
    user_attrs: &[syn::Attribute],
) -> syn::Result<TokenStream> {
    let ids = EntityIdents::new(name);
    let serde_derives = &ids.serde_derives;
    let serde_crate_attr = &ids.serde_crate_attr;
    let entity_snake = &ids.entity_snake;
    let event_enum = &ids.event_enum;
    let data_struct = &ids.data_struct;

    let ec = codegen_events(events, event_enum);
    let observer_and_handle = gen_observer_and_handle(name, events, &ids);

    let event_variants = &ec.event_variants;
    let from_impls = &ec.from_impls;
    let data_fields = &ec.data_fields;
    let data_push_arms = &ec.data_push_arms;
    let event_defs = &ec.event_defs;

    let doc_marker = format!(
        "The `{name}` multi-event entity.\n\nThis is a compile-time marker representing the entity you declared. Use\n[`{name}Observer`] to emit events, not this type directly."
    );
    let doc_event = format!("Events emitted by {name}.");
    let doc_data =
        format!("Analyzer data for {name} \u{2014} stores one `Option<T>` per event type.");

    Ok(quote! {
        #(#user_attrs)*
        #[doc = #doc_marker]
        pub struct #name;

        #[doc = #doc_event]
        #[doc(alias = "event")]
        #[derive(#serde_derives)]
        #serde_crate_attr
        pub enum #event_enum {
            #(#event_variants,)*
        }

        #(#from_impls)*

        #observer_and_handle

        #[doc = #doc_data]
        #[doc(alias = "data")]
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
                match event { #(#data_push_arms,)* }
            }
        }

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_entity(quent_model::EntityDef {
                    name: #entity_snake.to_string(),
                        module_path: module_path!().to_string(),
                    events: vec![#(#event_defs,)*],
                });
            }
        }
    })
}

// Resource group with inline declaration attributes (no events)

fn expand_rg_attrs(
    name: &Ident,
    meta: &ResourceGroupMeta,
    fields: &[InlineField],
    user_attrs: &[syn::Attribute],
) -> syn::Result<TokenStream> {
    let ids = EntityIdents::new(name);
    let serde_derives = &ids.serde_derives;
    let serde_crate_attr = &ids.serde_crate_attr;
    let serde_bound = &ids.serde_bound;
    let entity_snake = &ids.entity_snake;
    let event_enum = &ids.event_enum;
    let observer_name = &ids.observer_name;
    let data_struct = &ids.data_struct;
    let decl_struct = format_ident!("{}Declaration", name);
    let decl_snake = format!("{}_declaration", entity_snake);
    let observer_method = format_ident!("{}", entity_snake);
    let is_root = meta.is_root;

    // Declaration struct: instance_name + parent + user fields
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

    // Parent field
    if let Some(parent_ty) = &meta.parent_type {
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
        decl_fields.push(quote! { pub parent_group_id: quent_model::uuid::Uuid });
        decl_attr_defs.push(quote! {
            quent_model::AttributeDef {
                name: "parent_group_id".to_string(),
                value_type: quent_model::ValueType::Uuid,
                optional: false,
            }
        });
        observer_params.push(quote! { parent_group_id: quent_model::uuid::Uuid });
        decl_field_inits.push(quote! { parent_group_id, });
    }

    // User inline fields
    for f in fields {
        let fname = &f.name;
        let fty = &f.ty;
        decl_fields.push(quote! { pub #fname: #fty });
        let field_name_str = fname.to_string();
        let (vt, optional) = resolve_value_type(fty);
        decl_attr_defs.push(quote! {
            quent_model::AttributeDef {
                name: #field_name_str.to_string(),
                value_type: #vt,
                optional: #optional,
            }
        });
        observer_params.push(quote! { #fname: #fty });
        decl_field_inits.push(quote! { #fname, });
    }

    let doc_marker = format!(
        "The `{name}` resource group entity.\n\nThis is a compile-time marker representing the resource group you declared. Use\n[`{name}Observer`] to emit events, not this type directly."
    );
    let doc_decl = format!("Declaration attributes for the {name} resource group.");
    let doc_event = format!("Events emitted by {name}.");
    let doc_observer = format!(
        "Observer for `{name}` resource group declarations.\n\n\
         An observer emits events for a model component. Obtain one from the \
         instrumentation context via `{entity_snake}_observer()`.\n\n\
         The type parameter `E` is the model's top-level event enum, allowing \
         the same component to be reused across different models."
    );
    let doc_observer_method = format!("Declare a new `{name}` resource group instance.");
    let doc_data =
        format!("Analyzer data for `{name}` \u{2014} stores one `Option<T>` per event type.");

    Ok(quote! {
        #(#user_attrs)*
        #[doc = #doc_marker]
        pub struct #name;

        #[doc = #doc_decl]
        #[derive(#serde_derives)]
        #serde_crate_attr
        pub struct #decl_struct {
            #(#decl_fields,)*
        }

        #[doc = #doc_event]
        #[doc(alias = "event")]
        #[derive(#serde_derives)]
        #serde_crate_attr
        pub enum #event_enum {
            Declaration(#decl_struct),
        }

        impl From<#decl_struct> for #event_enum {
            fn from(e: #decl_struct) -> Self { #event_enum::Declaration(e) }
        }

        #[doc = #doc_observer]
            #[doc(alias = "observer")]
        #[derive(Clone)]
        pub struct #observer_name<E>
        where E: From<#event_enum> #serde_bound + Send + 'static,
        {
            tx: quent_model::EventSender<E>,
        }

        impl<E> #observer_name<E>
        where E: From<#event_enum> #serde_bound + Send + 'static,
        {
            pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                Self { tx: tx.clone() }
            }

            #[doc = #doc_observer_method]
            #[doc(alias = "observer")]
            pub fn #observer_method(
                &self,
                id: quent_model::uuid::Uuid,
                instance_name: &str,
                #(#observer_params,)*
            ) -> quent_model::uuid::Uuid {
                let event = #decl_struct {
                    instance_name: instance_name.to_string(),
                    #(#decl_field_inits)*
                };
                self.tx.emit(id, #event_enum::from(event));
                id
            }
        }

        #[doc = #doc_data]
        #[doc(alias = "data")]
        #[derive(Default)]
        pub struct #data_struct {
            pub declaration: Option<#decl_struct>,
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
                }
            }
        }

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_entity(quent_model::EntityDef {
                    name: #entity_snake.to_string(),
                        module_path: module_path!().to_string(),
                    events: vec![
                        quent_model::EntityEventDef {
                            name: #decl_snake.to_string(),
                            attributes: vec![#(#decl_attr_defs,)*],
                        }
                    ],
                });
                builder.add_resource_group(quent_model::ResourceGroupDef {
                    name: #entity_snake.to_string(),
                    fixed_parent: None,
                    is_root: #is_root,
                    declaration_event: Some(#decl_snake.to_string()),
                });
            }
        }
    })
}

// Resource group with events (+ optional declaration marker)

fn expand_rg_events(
    name: &Ident,
    meta: &ResourceGroupMeta,
    declaration: Option<&Ident>,
    events: &[EventEntry],
    user_attrs: &[syn::Attribute],
) -> syn::Result<TokenStream> {
    let ids = EntityIdents::new(name);
    let serde_derives = &ids.serde_derives;
    let serde_crate_attr = &ids.serde_crate_attr;
    let entity_snake = &ids.entity_snake;
    let event_enum = &ids.event_enum;
    let data_struct = &ids.data_struct;
    let is_root = meta.is_root;

    let ec = codegen_events(events, event_enum);
    let observer_and_handle = gen_observer_and_handle(name, events, &ids);

    let event_variants = &ec.event_variants;
    let from_impls = &ec.from_impls;
    let data_fields = &ec.data_fields;
    let data_push_arms = &ec.data_push_arms;
    let event_defs = &ec.event_defs;

    let declaration_event_tokens = match declaration {
        Some(d) => {
            let s = d.to_string();
            quote! { Some(#s.to_string()) }
        }
        None => quote! { None },
    };

    let doc_marker = format!(
        "The `{name}` resource group entity.\n\nThis is a compile-time marker representing the resource group you declared. Use\n[`{name}Observer`] to emit events, not this type directly."
    );
    let doc_event = format!("Events emitted by {name}.");
    let doc_data =
        format!("Analyzer data for {name} \u{2014} stores one `Option<T>` per event type.");

    Ok(quote! {
        #(#user_attrs)*
        #[doc = #doc_marker]
        pub struct #name;

        #[doc = #doc_event]
        #[doc(alias = "event")]
        #[derive(#serde_derives)]
        #serde_crate_attr
        pub enum #event_enum {
            #(#event_variants,)*
        }

        #(#from_impls)*

        #observer_and_handle

        #[doc = #doc_data]
        #[doc(alias = "data")]
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
                match event { #(#data_push_arms,)* }
            }
        }

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_entity(quent_model::EntityDef {
                    name: #entity_snake.to_string(),
                        module_path: module_path!().to_string(),
                    events: vec![#(#event_defs,)*],
                });
                builder.add_resource_group(quent_model::ResourceGroupDef {
                    name: #entity_snake.to_string(),
                    fixed_parent: None,
                    is_root: #is_root,
                    declaration_event: #declaration_event_tokens,
                });
            }
        }
    })
}
