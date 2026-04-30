// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `fsm!` proc macro implementation.
//!
//! Parses the new FSM syntax with explicit states, entry, exit_from, and
//! transitions. Generates the transition enum, observer, handle, and
//! ModelComponent with struct-arg entry and transition methods.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Path, Token, braced};

use crate::util::to_snake_case;

struct StateEntry {
    alias: Ident,
    ty: Path,
}

struct ResourceGroupMeta {
    is_root: bool,
    _parent_type: Option<Ident>,
}

struct FsmInput {
    user_attrs: Vec<syn::Attribute>,
    name: Ident,
    resource_group: Option<ResourceGroupMeta>,
    states: Vec<StateEntry>,
    entry: Ident,
    exit_from: Vec<Ident>,
    transitions: Vec<(Ident, Ident)>,
}

impl Parse for FsmInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let user_attrs = input.call(syn::Attribute::parse_outer)?;
        let name: Ident = input.parse()?;

        // Optional `: ResourceGroup<...>`
        let resource_group = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            let rg: Ident = input.parse()?;
            if rg != "ResourceGroup" {
                return Err(syn::Error::new_spanned(rg, "expected `ResourceGroup`"));
            }
            let mut is_root = false;
            let mut _parent_type = None;
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
                            _parent_type = Some(input.parse::<Ident>()?);
                        }
                        other => {
                            return Err(syn::Error::new_spanned(
                                key,
                                format!("unexpected `{other}`, expected `Root` or `Parent`"),
                            ));
                        }
                    }
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                input.parse::<Token![>]>()?;
            }
            Some(ResourceGroupMeta {
                is_root,
                _parent_type,
            })
        } else {
            None
        };

        let content;
        braced!(content in input);

        // Parse states: { alias: Type, ... }
        let states_kw: Ident = content.parse()?;
        if states_kw != "states" {
            return Err(syn::Error::new_spanned(states_kw, "expected `states`"));
        }
        content.parse::<Token![:]>()?;
        let states_content;
        braced!(states_content in content);
        let mut states = Vec::new();
        while !states_content.is_empty() {
            let alias: Ident = states_content.parse()?;
            states_content.parse::<Token![:]>()?;
            let ty: Path = states_content.parse()?;
            states.push(StateEntry { alias, ty });
            if states_content.peek(Token![,]) {
                states_content.parse::<Token![,]>()?;
            }
        }
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }

        // Parse entry: alias
        let entry_kw: Ident = content.parse()?;
        if entry_kw != "entry" {
            return Err(syn::Error::new_spanned(entry_kw, "expected `entry`"));
        }
        content.parse::<Token![:]>()?;
        let entry: Ident = content.parse()?;
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }

        // Parse exit_from: { alias, ... }
        let exit_kw: Ident = content.parse()?;
        if exit_kw != "exit_from" {
            return Err(syn::Error::new_spanned(exit_kw, "expected `exit_from`"));
        }
        content.parse::<Token![:]>()?;
        let exit_content;
        braced!(exit_content in content);
        let mut exit_from = Vec::new();
        while !exit_content.is_empty() {
            exit_from.push(exit_content.parse::<Ident>()?);
            if exit_content.peek(Token![,]) {
                exit_content.parse::<Token![,]>()?;
            }
        }
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }

        // Parse transitions: { from => to, ... }
        let trans_kw: Ident = content.parse()?;
        if trans_kw != "transitions" {
            return Err(syn::Error::new_spanned(trans_kw, "expected `transitions`"));
        }
        content.parse::<Token![:]>()?;
        let trans_content;
        braced!(trans_content in content);
        let mut transitions = Vec::new();
        while !trans_content.is_empty() {
            let from: Ident = trans_content.parse()?;
            trans_content.parse::<Token![=>]>()?;
            let to: Ident = trans_content.parse()?;
            transitions.push((from, to));
            if trans_content.peek(Token![,]) {
                trans_content.parse::<Token![,]>()?;
            }
        }
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }

        Ok(FsmInput {
            user_attrs,
            name,
            resource_group,
            states,
            entry,
            exit_from,
            transitions,
        })
    }
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: FsmInput = syn::parse2(input)?;
    let name = &input.name;
    let fsm_snake = to_snake_case(name);
    let serde_derives = crate::util::serde_derives();
    let serde_crate_attr = crate::util::serde_crate_attr();
    let serde_bound = crate::util::serde_bound();

    let transition_enum = format_ident!("{}Transition", name);
    let event_type = format_ident!("{}Event", name);
    let handle_name = format_ident!("{}Handle", name);
    let observer_name = format_ident!("{}Observer", name);

    // Resource group support
    let (rg_trait_impl, rg_contribution) = if let Some(ref rg) = input.resource_group {
        let is_root = rg.is_root;
        (
            quote! {
                impl quent_model::ResourceGroup for #name {
                    const IS_ROOT: bool = #is_root;
                }
            },
            {
                let entry_str = input.entry.to_string();
                quote! {
                    builder.add_resource_group(quent_model::ResourceGroupDef {
                        name: #fsm_snake.to_string(),
                        fixed_parent: None,
                        is_root: #is_root,
                        declaration_event: Some(#entry_str.to_string()),
                    });
                }
            },
        )
    } else {
        (quote! {}, quote! {})
    };

    // State type paths and aliases
    let state_aliases: Vec<&Ident> = input.states.iter().map(|s| &s.alias).collect();
    let state_types: Vec<&Path> = input.states.iter().map(|s| &s.ty).collect();
    let state_pascal_names: Vec<Ident> = input
        .states
        .iter()
        .map(|s| format_ident!("{}", crate::util::to_pascal_case(&s.alias.to_string())))
        .collect();

    // Transition enum variants
    let transition_variants: Vec<TokenStream> = state_pascal_names
        .iter()
        .zip(state_types.iter())
        .map(|(pascal, ty)| quote! { #pascal(#ty) })
        .collect();

    // From impls
    let from_impls: Vec<TokenStream> = state_pascal_names
        .iter()
        .zip(state_types.iter())
        .map(|(pascal, ty)| {
            quote! {
                impl From<#ty> for #transition_enum {
                    fn from(s: #ty) -> Self { #transition_enum::#pascal(s) }
                }
            }
        })
        .collect();

    // Entry state info
    let entry_alias = &input.entry;
    let _entry_state = input
        .states
        .iter()
        .find(|s| s.alias == input.entry)
        .ok_or_else(|| {
            syn::Error::new_spanned(&input.entry, "entry state not found in states list")
        })?;

    // Validate exit_from aliases exist in states list
    for exit_ident in &input.exit_from {
        if !input.states.iter().any(|s| s.alias == *exit_ident) {
            return Err(syn::Error::new_spanned(
                exit_ident,
                format!("exit_from state `{}` not found in states list", exit_ident),
            ));
        }
    }

    // Validate transition from/to aliases exist in states list
    for (from, to) in &input.transitions {
        if !input.states.iter().any(|s| s.alias == *from) {
            return Err(syn::Error::new_spanned(
                from,
                format!("transition source `{}` not found in states list", from),
            ));
        }
        if !input.states.iter().any(|s| s.alias == *to) {
            return Err(syn::Error::new_spanned(
                to,
                format!("transition target `{}` not found in states list", to),
            ));
        }
    }

    // TransitionInfo impl arms
    let name_arms: Vec<TokenStream> = state_pascal_names
        .iter()
        .zip(state_aliases.iter())
        .map(|(pascal, alias)| {
            let alias_str = alias.to_string();
            quote! { #transition_enum::#pascal(_) => #alias_str }
        })
        .collect();

    let usages_arms: Vec<TokenStream> = state_pascal_names
        .iter()
        .zip(state_types.iter())
        .map(|(pascal, _ty)| {
            quote! { #transition_enum::#pascal(data) => quent_model::analyze::ExtractUsages::extract_usages(data) }
        })
        .collect();

    let instance_name_arms: Vec<TokenStream> = state_pascal_names
        .iter()
        .zip(state_types.iter())
        .map(|(pascal, _ty)| {
            quote! { #transition_enum::#pascal(data) => quent_model::analyze::ExtractInstanceName::extract_instance_name(data) }
        })
        .collect();

    let parent_group_id_arms: Vec<TokenStream> = state_pascal_names
        .iter()
        .zip(state_types.iter())
        .map(|(pascal, _ty)| {
            quote! { #transition_enum::#pascal(data) => quent_model::analyze::ExtractParentGroupId::extract_parent_group_id(data) }
        })
        .collect();

    // State defs for ModelComponent
    let state_def_calls: Vec<TokenStream> = state_types
        .iter()
        .map(|ty| quote! { <#ty as quent_model::StateMetadata>::state_def() })
        .collect();

    // Transition defs for ModelComponent
    let mut transition_def_tokens = Vec::new();
    // entry -> first state
    {
        let entry_name = entry_alias.to_string();
        transition_def_tokens.push(quote! {
            quent_model::TransitionDef {
                from: quent_model::TransitionEndpoint::Entry,
                to: quent_model::TransitionEndpoint::State(#entry_name.to_string()),
            }
        });
    }
    // state -> state transitions
    for (from, to) in &input.transitions {
        let from_str = from.to_string();
        let to_str = to.to_string();
        transition_def_tokens.push(quote! {
            quent_model::TransitionDef {
                from: quent_model::TransitionEndpoint::State(#from_str.to_string()),
                to: quent_model::TransitionEndpoint::State(#to_str.to_string()),
            }
        });
    }
    // exit_from -> exit
    for exit_state in &input.exit_from {
        let exit_str = exit_state.to_string();
        transition_def_tokens.push(quote! {
            quent_model::TransitionDef {
                from: quent_model::TransitionEndpoint::State(#exit_str.to_string()),
                to: quent_model::TransitionEndpoint::Exit,
            }
        });
    }

    // Generate observer and handle methods via state callback macros.
    // Methods are always flat-arg: instance_name + attributes + optional usages.
    let entry_callback = format_ident!("__quent_state_{}", entry_alias.to_string());
    let observer_methods = quote! {
        #entry_callback!(entry_method pub #handle_name #transition_enum tx);
    };

    let handle_methods = {
        let transition_callback_invocations: Vec<TokenStream> = input
            .states
            .iter()
            .map(|s| {
                let callback = format_ident!("__quent_state_{}", s.alias.to_string());
                quote! {
                    #callback!(transition_method pub #transition_enum);
                }
            })
            .collect();
        quote! { #(#transition_callback_invocations)* }
    };

    let user_attrs = &input.user_attrs;
    let doc_marker = format!(
        "The `{name}` finite state machine.\n\nThis is a compile-time marker representing the FSM you declared. Use\n[`{name}Observer`] to create instances, not this type directly."
    );
    let doc_transition = format!("State transitions for the {name} FSM.");
    let doc_event = format!("Event type alias for {name} FSM transitions.");
    let doc_handle = format!("Handle for an active {name} FSM instance.");
    let doc_handle_uuid = format!("Returns the UUID of this {name} FSM instance.");
    let doc_handle_exit = format!("Transition the {name} FSM to the exit state.");
    let doc_observer = format!(
        "Observer for `{name}` FSM instances.\n\n\
         An observer emits events for a model component. Obtain one from the \
         instrumentation context via the corresponding observer method. \
         Call the entry state method to create an FSM handle.\n\n\
         The type parameter `E` is the model's top-level event enum, allowing \
         the same component to be reused across different models."
    );

    let output = quote! {
        #(#user_attrs)*
        #[doc = #doc_marker]
        pub struct #name;

        #[doc = #doc_transition]
        #[doc(alias = "transition")]
        #[derive(#serde_derives)]
        #serde_crate_attr
        pub enum #transition_enum {
            #(#transition_variants,)*
            Exit,
        }

        #(#from_impls)*

        impl quent_model::analyze::TransitionInfo for #transition_enum {
            fn state_name(&self) -> &'static str {
                match self {
                    #(#name_arms,)*
                    #transition_enum::Exit => "exit",
                }
            }

            fn usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> {
                match self {
                    #(#usages_arms,)*
                    #transition_enum::Exit => vec![],
                }
            }

            fn instance_name(&self) -> Option<&str> {
                match self {
                    #(#instance_name_arms,)*
                    #transition_enum::Exit => None,
                }
            }

            fn parent_group_id(&self) -> Option<quent_model::uuid::Uuid> {
                match self {
                    #(#parent_group_id_arms,)*
                    #transition_enum::Exit => None,
                }
            }

            fn fsm_type_name() -> &'static str { #fsm_snake }

            fn collect_model(builder: &mut quent_model::ModelBuilder) {
                <#name as quent_model::ModelComponent>::collect(builder);
            }
        }

        #[doc = #doc_event]
        pub type #event_type = quent_model::FsmEvent<#transition_enum>;

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                {
                    let entry_str = stringify!(#entry_alias);
                    builder.add_fsm(quent_model::FsmDef {
                        name: #fsm_snake.to_string(),
                        module_path: module_path!().to_string(),
                        entry: entry_str.to_string(),
                        states: vec![#(#state_def_calls,)*],
                        transitions: vec![#(#transition_def_tokens,)*],
                    });
                }
                #rg_contribution
            }
        }

        #rg_trait_impl

        #[doc = #doc_handle]
        #[doc(alias = "handle")]
        pub struct #handle_name<E>
        where
            E: From<#event_type> #serde_bound + Send + 'static,
        {
            id: quent_model::uuid::Uuid,
            seq: u64,
            exited: bool,
            tx: quent_model::EventSender<E>,
        }

        impl<E> #handle_name<E>
        where
            E: From<#event_type> #serde_bound + Send + 'static,
        {
            #[doc = #doc_handle_uuid]
            pub fn uuid(&self) -> quent_model::uuid::Uuid { self.id }

            fn transition(&mut self, state: impl Into<#transition_enum>) {
                self.emit_transition(state.into());
            }

            #[doc = #doc_handle_exit]
            pub fn exit(&mut self) {
                if !self.exited {
                    self.emit_transition(#transition_enum::Exit);
                    self.exited = true;
                }
            }

            fn emit_transition(&mut self, state: #transition_enum) {
                let seq = self.seq;
                self.seq += 1;
                let event = quent_model::FsmEvent { seq, state };
                self.tx.send(quent_model::Event::new(
                    self.id,
                    quent_model::timestamp(),
                    E::from(event),
                ));
            }

            #handle_methods
        }

        impl<E> Drop for #handle_name<E>
        where
            E: From<#event_type> #serde_bound + Send + 'static,
        {
            fn drop(&mut self) { self.exit(); }
        }

        #[doc = #doc_observer]
        #[doc(alias = "observer")]
        #[derive(Clone)]
        pub struct #observer_name<E>
        where
            E: From<#event_type> #serde_bound + Send + 'static,
        {
            tx: quent_model::EventSender<E>,
        }

        impl<E> #observer_name<E>
        where
            E: From<#event_type> #serde_bound + Send + 'static,
        {
            pub fn new(tx: &quent_model::EventSender<E>) -> Self {
                Self { tx: tx.clone() }
            }

            #observer_methods
        }

        impl quent_model::HasEventType for #name {
            type Event = quent_model::FsmEvent<#transition_enum>;
        }
    };

    Ok(output)
}
