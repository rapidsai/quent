// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `state!` proc macro implementation.
//!
//! Parses a state definition with optional attributes and resource usages.
//! Generates:
//! - A state struct with fields for attributes and `Usage<T>` per resource
//! - `StateMetadata` and `Extract*` trait impls
//! - A hidden `__quent_state_{name}` callback macro for `fsm!`
//!
//! Two forms of `attributes:` are supported:
//! - **Inline fields**: `attributes: { field: Type, ... }` — auto-adds
//!   `instance_name: String` and generates everything internally.
//! - **External struct**: `attributes: MyAttrsType` — delegates to a
//!   user-defined struct that implements `ExtractInstanceName` etc.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Path, Token, Type, braced};

use crate::util::{resolve_value_type, to_snake_case};

struct InlineField {
    name: Ident,
    ty: Type,
}

struct UsageEntry {
    alias: Ident,
    resource_type: Path,
}

enum AttributesDef {
    /// Inline field declarations — `instance_name: String` is auto-added.
    Inline(Vec<InlineField>),
    /// Reference to an external struct that implements `ExtractInstanceName` etc.
    ExternalStruct(Path),
}

struct StateInput {
    user_attrs: Vec<syn::Attribute>,
    name: Ident,
    attributes: Option<AttributesDef>,
    usages: Vec<UsageEntry>,
}

impl Parse for StateInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let user_attrs = input.call(syn::Attribute::parse_outer)?;
        let name: Ident = input.parse()?;

        let content;
        braced!(content in input);

        let mut attributes = None;
        let mut usages = Vec::new();

        while !content.is_empty() {
            let kw: Ident = content.parse()?;
            content.parse::<Token![:]>()?;

            match kw.to_string().as_str() {
                "attributes" => {
                    if content.peek(syn::token::Brace) {
                        // Inline: attributes: { field: Type, ... }
                        let fields_content;
                        braced!(fields_content in content);
                        let mut fields = Vec::new();
                        while !fields_content.is_empty() {
                            let field_name: Ident = fields_content.parse()?;
                            fields_content.parse::<Token![:]>()?;
                            let ty: Type = fields_content.parse()?;
                            fields.push(InlineField {
                                name: field_name,
                                ty,
                            });
                            if fields_content.peek(Token![,]) {
                                fields_content.parse::<Token![,]>()?;
                            }
                        }
                        attributes = Some(AttributesDef::Inline(fields));
                    } else {
                        // External struct: attributes: QueuedAttrs
                        let ty: Path = content.parse()?;
                        attributes = Some(AttributesDef::ExternalStruct(ty));
                    }
                }
                "usages" => {
                    let usages_content;
                    braced!(usages_content in content);
                    while !usages_content.is_empty() {
                        let alias: Ident = usages_content.parse()?;
                        usages_content.parse::<Token![:]>()?;
                        let resource_type: Path = usages_content.parse()?;
                        usages.push(UsageEntry {
                            alias,
                            resource_type,
                        });
                        if usages_content.peek(Token![,]) {
                            usages_content.parse::<Token![,]>()?;
                        }
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        kw,
                        format!("unexpected keyword `{other}`, expected `attributes` or `usages`"),
                    ));
                }
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(StateInput {
            user_attrs,
            name,
            attributes,
            usages,
        })
    }
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: StateInput = syn::parse2(input)?;
    let name = &input.name;
    let name_snake = to_snake_case(name);
    let serde_derives = crate::util::serde_derives();
    let serde_crate_attr = crate::util::serde_crate_attr();
    let callback_name = format_ident!("__quent_state_{}", name_snake);

    // All usages are optional — the state may or may not use a given resource.
    let usage_field_defs: Vec<TokenStream> = input
        .usages
        .iter()
        .map(|u| {
            let alias = &u.alias;
            let resource_type = &u.resource_type;
            quote! { pub #alias: Option<quent_model::Usage<#resource_type>> }
        })
        .collect();

    // Generate struct, metadata, extract impls, and callback based on attribute mode.
    let user_attrs = &input.user_attrs;
    let (struct_def, attr_defs_tokens, extract_instance_name_body, extract_parent_group_id_body) =
        match &input.attributes {
            Some(AttributesDef::Inline(fields)) => expand_inline_attrs(
                name,
                fields,
                &usage_field_defs,
                &serde_derives,
                &serde_crate_attr,
                user_attrs,
            ),
            Some(AttributesDef::ExternalStruct(path)) => expand_external_attrs(
                name,
                path,
                &usage_field_defs,
                &serde_derives,
                &serde_crate_attr,
                user_attrs,
            ),
            None => expand_no_attrs(
                name,
                &usage_field_defs,
                &serde_derives,
                &serde_crate_attr,
                user_attrs,
            ),
        };

    // Usage defs for StateMetadata
    let usage_def_tokens: Vec<TokenStream> = input
        .usages
        .iter()
        .map(|u| {
            let alias_str = u.alias.to_string();
            let resource_type = &u.resource_type;
            let resource_type_str = quote!(#resource_type).to_string();
            quote! {
                quent_model::UsageDef {
                    field_name: #alias_str.to_string(),
                    resource_name: <#resource_type as quent_model::Resource>::RESOURCE_NAME
                        .to_string(),
                    resource_type_path: #resource_type_str.to_string(),
                }
            }
        })
        .collect();

    let state_metadata = quote! {
        impl quent_model::StateMetadata for #name {
            fn state_name() -> &'static str {
                #name_snake
            }
            fn state_def() -> quent_model::StateDef {
                quent_model::StateDef {
                    name: #name_snake.to_string(),
                    attributes: #attr_defs_tokens,
                    usages: vec![#(#usage_def_tokens,)*],
                }
            }
        }
    };

    // ExtractCapacities: delegate to each usage's capacity, skipping None
    let extract_capacities_body = if input.usages.is_empty() {
        quote! { vec![] }
    } else {
        let extractions: Vec<TokenStream> = input
            .usages
            .iter()
            .map(|u| {
                let alias = &u.alias;
                quote! {
                    if let Some(ref usage) = self.#alias {
                        caps.extend(
                            quent_model::analyze::ExtractCapacities::extract_capacities(
                                &usage.capacity
                            )
                        );
                    }
                }
            })
            .collect();
        quote! {
            {
                let mut caps = Vec::new();
                #(#extractions)*
                caps
            }
        }
    };

    // ExtractUsages: collect present usages, skipping None
    let extract_usages_body = if input.usages.is_empty() {
        quote! { vec![] }
    } else {
        let extractions: Vec<TokenStream> = input
            .usages
            .iter()
            .map(|u| {
                let alias = &u.alias;
                quote! {
                    if let Some(ref usage) = self.#alias {
                        usages.push(quent_model::analyze::ExtractedUsage {
                            resource_id: usage.resource_id.uuid(),
                            capacities: quent_model::analyze::ExtractCapacities::extract_capacities(
                                &usage.capacity
                            ),
                        });
                    }
                }
            })
            .collect();
        quote! {
            {
                let mut usages = Vec::new();
                #(#extractions)*
                usages
            }
        }
    };

    let extract_impls = quote! {
        impl quent_model::analyze::ExtractCapacities for #name {
            fn extract_capacities(&self) -> Vec<quent_model::analyze::ExtractedCapacity> {
                #extract_capacities_body
            }
        }

        impl quent_model::analyze::ExtractUsages for #name {
            fn extract_usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> {
                #extract_usages_body
            }
        }

        impl quent_model::analyze::ExtractInstanceName for #name {
            fn extract_instance_name(&self) -> Option<&str> {
                #extract_instance_name_body
            }
        }

        impl quent_model::analyze::ExtractParentGroupId for #name {
            fn extract_parent_group_id(&self) -> Option<quent_model::uuid::Uuid> {
                #extract_parent_group_id_body
            }
        }
    };

    // Callback macro for fsm! integration.
    // Each usage is Option<Usage<T>> — concrete type so bare None works.
    // Callers use `Some(usage(value))` for present usages.
    let usage_params: Vec<TokenStream> = input
        .usages
        .iter()
        .map(|u| {
            let alias = &u.alias;
            let resource_type = &u.resource_type;
            quote! { #alias: Option<quent_model::Usage<#resource_type>>, }
        })
        .collect();

    let usage_field_inits: Vec<TokenStream> = input
        .usages
        .iter()
        .map(|u| {
            let alias = &u.alias;
            quote! { #alias, }
        })
        .collect();

    let entry_alias = format_ident!("{}", name_snake);

    let (attrs_params, attrs_field_inits) = match &input.attributes {
        Some(AttributesDef::Inline(fields)) => {
            // instance_name: &str as first param, then each inline field
            let mut params = vec![quote! { instance_name: &str, }];
            let mut inits = vec![quote! { instance_name: instance_name.to_string(), }];
            for f in fields {
                let fname = &f.name;
                let fty = &f.ty;
                params.push(quote! { #fname: #fty, });
                inits.push(quote! { #fname, });
            }
            (params, inits)
        }
        Some(AttributesDef::ExternalStruct(path)) => {
            (vec![quote! { attrs: #path, }], vec![quote! { attrs, }])
        }
        None => (vec![], vec![]),
    };

    let state_callback = quote! {
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #callback_name {
            (entry_method $vis:vis $handle:ident $transition:ident $observer_tx:ident) => {
                $vis fn #entry_alias(
                    &self,
                    id: quent_model::uuid::Uuid,
                    #(#attrs_params)*
                    #(#usage_params)*
                ) -> $handle<E> {
                    let state = #name {
                        #(#attrs_field_inits)*
                        #(#usage_field_inits)*
                    };
                    let mut handle = $handle {
                        id,
                        seq: 0,
                        exited: false,
                        tx: self.$observer_tx.clone(),
                    };
                    handle.emit_transition($transition::from(state));
                    handle
                }
            };
            (transition_method $vis:vis $transition:ident) => {
                $vis fn #entry_alias(
                    &mut self,
                    #(#attrs_params)*
                    #(#usage_params)*
                ) {
                    let state = #name {
                        #(#attrs_field_inits)*
                        #(#usage_field_inits)*
                    };
                    self.emit_transition($transition::from(state));
                }
            };
        }
    };

    let output = quote! {
        #struct_def
        #state_metadata
        #extract_impls
        #state_callback
    };

    Ok(output)
}

/// Inline attributes: auto-adds `instance_name: String`, generates flat struct.
fn expand_inline_attrs(
    name: &Ident,
    fields: &[InlineField],
    usage_field_defs: &[TokenStream],
    serde_derives: &TokenStream,
    serde_crate_attr: &TokenStream,
    user_attrs: &[syn::Attribute],
) -> (TokenStream, TokenStream, TokenStream, TokenStream) {
    let inline_field_defs: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let fname = &f.name;
            let fty = &f.ty;
            quote! { pub #fname: #fty }
        })
        .collect();

    let doc_state = format!("FSM state {name} with inline attributes.");
    let struct_def = quote! {
        #(#user_attrs)*
        #[doc = #doc_state]
        #[derive(#serde_derives)]
        #serde_crate_attr
        pub struct #name {
            pub instance_name: String,
            #(#inline_field_defs,)*
            #(#usage_field_defs,)*
        }
    };

    // Attribute defs for StateMetadata: instance_name + inline fields
    let mut attr_def_tokens: Vec<TokenStream> = vec![quote! {
        quent_model::AttributeDef {
            name: "instance_name".to_string(),
            value_type: quent_model::ValueType::String,
            optional: false,
        }
    }];
    for f in fields {
        let field_name = f.name.to_string();
        let (value_type_tokens, optional) = resolve_value_type(&f.ty);
        attr_def_tokens.push(quote! {
            quent_model::AttributeDef {
                name: #field_name.to_string(),
                value_type: #value_type_tokens,
                optional: #optional,
            }
        });
    }

    let attr_defs = quote! { vec![#(#attr_def_tokens,)*] };
    let extract_instance_name = quote! { Some(&self.instance_name) };
    let extract_parent_group_id = quote! { None };

    (
        struct_def,
        attr_defs,
        extract_instance_name,
        extract_parent_group_id,
    )
}

/// External struct attributes: delegates to the struct's trait impls.
fn expand_external_attrs(
    name: &Ident,
    attrs_ty: &Path,
    usage_field_defs: &[TokenStream],
    serde_derives: &TokenStream,
    serde_crate_attr: &TokenStream,
    user_attrs: &[syn::Attribute],
) -> (TokenStream, TokenStream, TokenStream, TokenStream) {
    let serde_flatten = if cfg!(feature = "serde") {
        quote! { #[serde(flatten)] }
    } else {
        quote! {}
    };

    let doc_state = format!("FSM state {name} with external attributes.");
    let struct_def = quote! {
        #(#user_attrs)*
        #[doc = #doc_state]
        #[derive(#serde_derives)]
        #serde_crate_attr
        pub struct #name {
            #serde_flatten
            pub attrs: #attrs_ty,
            #(#usage_field_defs,)*
        }
    };

    let attr_defs = quote! {
        <#attrs_ty as quent_model::EventMetadata>::event_def().attributes
    };
    let extract_instance_name = quote! {
        quent_model::analyze::ExtractInstanceName::extract_instance_name(&self.attrs)
    };
    let extract_parent_group_id = quote! {
        quent_model::analyze::ExtractParentGroupId::extract_parent_group_id(&self.attrs)
    };

    (
        struct_def,
        attr_defs,
        extract_instance_name,
        extract_parent_group_id,
    )
}

/// No attributes: usage-only state.
fn expand_no_attrs(
    name: &Ident,
    usage_field_defs: &[TokenStream],
    serde_derives: &TokenStream,
    serde_crate_attr: &TokenStream,
    user_attrs: &[syn::Attribute],
) -> (TokenStream, TokenStream, TokenStream, TokenStream) {
    let doc_state = format!("FSM state {name}.");
    let struct_def = quote! {
        #(#user_attrs)*
        #[doc = #doc_state]
        #[derive(#serde_derives)]
        #serde_crate_attr
        pub struct #name {
            #(#usage_field_defs,)*
        }
    };

    let attr_defs = quote! { vec![] };
    let extract_instance_name = quote! { None };
    let extract_parent_group_id = quote! { None };

    (
        struct_def,
        attr_defs,
        extract_instance_name,
        extract_parent_group_id,
    )
}
