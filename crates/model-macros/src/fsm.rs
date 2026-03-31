// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeSet, HashMap, HashSet};

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::Token;

use crate::util::to_snake_case;

/// A single transition: `From -> To`
struct Transition {
    from: TransitionEndpoint,
    to: TransitionEndpoint,
}

enum TransitionEndpoint {
    Entry,
    Exit,
    State(Ident),
}

impl TransitionEndpoint {
    fn as_state_ident(&self) -> Option<&Ident> {
        match self {
            TransitionEndpoint::State(ident) => Some(ident),
            _ => None,
        }
    }
}

impl Parse for TransitionEndpoint {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "entry" => Ok(TransitionEndpoint::Entry),
            "exit" => Ok(TransitionEndpoint::Exit),
            _ => Ok(TransitionEndpoint::State(ident)),
        }
    }
}

impl Parse for Transition {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let from: TransitionEndpoint = input.parse()?;
        input.parse::<Token![->]>()?;
        let to: TransitionEndpoint = input.parse()?;
        Ok(Transition { from, to })
    }
}

/// Parsed FSM attribute contents.
struct FsmAttr {
    transitions: Vec<Transition>,
    resource_capacity: Option<Ident>,
}

impl Parse for FsmAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut transitions = Vec::new();
        let mut resource_capacity = None;

        while !input.is_empty() {
            // Check for `resource(capacity = T)` directive
            if input.peek(syn::Ident) {
                let fork = input.fork();
                let ident: Ident = fork.parse()?;
                if ident == "resource" && fork.peek(syn::token::Paren) {
                    // Consume the ident from the real stream
                    let _: Ident = input.parse()?;
                    let content;
                    syn::parenthesized!(content in input);
                    let key: Ident = content.parse()?;
                    if key != "capacity" {
                        return Err(syn::Error::new_spanned(key, "expected `capacity`"));
                    }
                    content.parse::<Token![=]>()?;
                    resource_capacity = Some(content.parse::<Ident>()?);
                    // Consume optional trailing comma
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                    continue;
                }
            }

            // Parse as a transition
            let transition: Transition = input.parse()?;
            transitions.push(transition);

            // Consume optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(FsmAttr {
            transitions,
            resource_capacity,
        })
    }
}

/// Validates the FSM transition graph.
fn validate_fsm(
    fsm_name: &Ident,
    transitions: &[Transition],
) -> syn::Result<()> {
    // Collect all state names
    let mut states = BTreeSet::new();
    let mut has_entry = false;
    let mut has_exit_target = false;

    for t in transitions {
        if matches!(t.from, TransitionEndpoint::Entry) {
            has_entry = true;
        }
        if matches!(t.to, TransitionEndpoint::Exit) {
            has_exit_target = true;
        }
        if matches!(t.from, TransitionEndpoint::Exit) {
            return Err(syn::Error::new_spanned(
                fsm_name,
                "transitions out of `exit` are not allowed",
            ));
        }
        if matches!(t.to, TransitionEndpoint::Entry) {
            return Err(syn::Error::new_spanned(
                fsm_name,
                "transitions into `entry` are not allowed",
            ));
        }
        if let Some(ident) = t.from.as_state_ident() {
            states.insert(ident.to_string());
        }
        if let Some(ident) = t.to.as_state_ident() {
            states.insert(ident.to_string());
        }
    }

    if !has_entry {
        return Err(syn::Error::new_spanned(
            fsm_name,
            "FSM must have at least one `entry -> State` transition",
        ));
    }
    if !has_exit_target {
        return Err(syn::Error::new_spanned(
            fsm_name,
            "FSM must have at least one `State -> exit` transition",
        ));
    }

    // Build adjacency list for reachability checks
    let mut forward: HashMap<String, Vec<String>> = HashMap::new();
    let mut backward: HashMap<String, Vec<String>> = HashMap::new();

    for t in transitions {
        let from_key = match &t.from {
            TransitionEndpoint::Entry => "entry".to_string(),
            TransitionEndpoint::Exit => "exit".to_string(),
            TransitionEndpoint::State(i) => i.to_string(),
        };
        let to_key = match &t.to {
            TransitionEndpoint::Entry => "entry".to_string(),
            TransitionEndpoint::Exit => "exit".to_string(),
            TransitionEndpoint::State(i) => i.to_string(),
        };
        forward.entry(from_key.clone()).or_default().push(to_key.clone());
        backward.entry(to_key).or_default().push(from_key);
    }

    // Check: every state reachable from entry
    let reachable = reachable_from("entry", &forward);
    for state in &states {
        if !reachable.contains(state.as_str()) {
            return Err(syn::Error::new_spanned(
                fsm_name,
                format!("state `{state}` is not reachable from entry"),
            ));
        }
    }

    // Check: every state can reach exit
    let can_reach_exit = reachable_from("exit", &backward);
    for state in &states {
        if !can_reach_exit.contains(state.as_str()) {
            return Err(syn::Error::new_spanned(
                fsm_name,
                format!("state `{state}` cannot reach exit"),
            ));
        }
    }

    Ok(())
}

fn reachable_from(start: &str, adj: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut visited = HashSet::new();
    let mut stack = vec![start.to_string()];
    while let Some(node) = stack.pop() {
        if visited.insert(node.clone()) {
            if let Some(neighbors) = adj.get(&node) {
                for n in neighbors {
                    stack.push(n.clone());
                }
            }
        }
    }
    visited
}

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let fsm_attr: FsmAttr = syn::parse2(attr)?;
    let input: syn::ItemStruct = syn::parse2(item)?;
    let vis = &input.vis;
    let fsm_name = &input.ident;

    let transitions = fsm_attr.transitions;
    let resource_capacity = fsm_attr.resource_capacity;
    validate_fsm(fsm_name, &transitions)?;

    // Collect unique state idents (preserving order)
    let mut state_idents: Vec<Ident> = Vec::new();
    let mut seen = HashSet::new();
    for t in &transitions {
        for endpoint in [&t.from, &t.to] {
            if let TransitionEndpoint::State(ident) = endpoint {
                if seen.insert(ident.to_string()) {
                    state_idents.push(ident.clone());
                }
            }
        }
    }

    let fsm_snake = to_snake_case(fsm_name);

    // Names for generated types
    let transition_enum = format_ident!("{}Transition", fsm_name);
    let deferred_enum = format_ident!("{}Deferred", fsm_name);
    let event_type = format_ident!("{}Event", fsm_name);
    let handle_name = format_ident!("{}Handle", fsm_name);

    // Generate transition enum variants
    let transition_variants: Vec<TokenStream> = state_idents
        .iter()
        .map(|ident| quote! { #ident(#ident) })
        .collect();

    // Generate From impls: State -> TransitionEnum
    let from_impls: Vec<TokenStream> = state_idents
        .iter()
        .map(|ident| {
            quote! {
                impl From<#ident> for #transition_enum {
                    fn from(s: #ident) -> Self {
                        #transition_enum::#ident(s)
                    }
                }
            }
        })
        .collect();

    // Generate transition def tokens for ModelComponent
    let transition_def_tokens: Vec<TokenStream> = transitions
        .iter()
        .map(|t| {
            let from_token = match &t.from {
                TransitionEndpoint::Entry => quote! { quent_model::TransitionEndpoint::Entry },
                TransitionEndpoint::Exit => quote! { quent_model::TransitionEndpoint::Exit },
                TransitionEndpoint::State(i) => {
                    let name = to_snake_case(i);
                    quote! { quent_model::TransitionEndpoint::State(#name.to_string()) }
                }
            };
            let to_token = match &t.to {
                TransitionEndpoint::Entry => quote! { quent_model::TransitionEndpoint::Entry },
                TransitionEndpoint::Exit => quote! { quent_model::TransitionEndpoint::Exit },
                TransitionEndpoint::State(i) => {
                    let name = to_snake_case(i);
                    quote! { quent_model::TransitionEndpoint::State(#name.to_string()) }
                }
            };
            quote! {
                quent_model::TransitionDef {
                    from: #from_token,
                    to: #to_token,
                }
            }
        })
        .collect();

    // Generate state_def collection calls
    let state_def_calls: Vec<TokenStream> = state_idents
        .iter()
        .map(|ident| {
            quote! {
                <#ident as quent_model::StateMetadata>::state_def()
            }
        })
        .collect();

    let resource_marker = format_ident!("{}Resource", fsm_name);
    let resource_impl = resource_capacity.map(|cap_state| {
        quote! {
            #vis struct #resource_marker;

            impl quent_model::Resource for #resource_marker {
                type CapacityValue = #cap_state;
            }
        }
    });

    let output = quote! {
        // --- Transition enum ---
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #vis enum #transition_enum {
            #(#transition_variants,)*
            Exit,
        }

        #(#from_impls)*

        // --- Deferred enum ---
        // Nested: wraps per-state deferred types. States without deferred
        // fields have uninhabitable deferred types and no variant here.
        // For now, we generate a variant for every state and rely on the
        // per-state deferred type being uninhabitable when there are no
        // deferred fields.
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #vis enum #deferred_enum {
            #(#state_idents(<#state_idents as quent_model::StateMetadata>::Deferred),)*
        }

        // --- Event type alias ---
        #vis type #event_type = quent_model::FsmEvent<#transition_enum, #deferred_enum>;

        // --- Model metadata marker (non-generic) ---
        #vis struct #fsm_name;

        impl quent_model::ModelComponent for #fsm_name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_fsm(quent_model::FsmDef {
                    name: #fsm_snake.to_string(),
                    states: vec![
                        #(#state_def_calls,)*
                    ],
                    transitions: vec![
                        #(#transition_def_tokens,)*
                    ],
                });
            }
        }

        // --- The FSM handle struct ---
        // Generic over E: the top-level event type (e.g., SimulatorEvent).
        // Requires E: From<FsmEventType> so the handle can wrap its events.
        #vis struct #handle_name<E>
        where
            E: From<#event_type> + serde::Serialize + Send + std::fmt::Debug + 'static,
        {
            id: uuid::Uuid,
            seq: u64,
            exited: bool,
            tx: quent_model::EventSender<E>,
        }

        impl<E> #handle_name<E>
        where
            E: From<#event_type> + serde::Serialize + Send + std::fmt::Debug + 'static,
        {
            /// Creates a new FSM instance, emitting the entry transition event.
            pub fn new(tx: &quent_model::EventSender<E>, initial_state: impl Into<#transition_enum>) -> Self {
                let id = uuid::Uuid::now_v7();
                let mut handle = Self {
                    id,
                    seq: 0,
                    exited: false,
                    tx: tx.clone(),
                };
                handle.emit_transition(initial_state.into());
                handle
            }

            /// Returns the raw UUID of this FSM instance.
            pub fn uuid(&self) -> uuid::Uuid {
                self.id
            }

            /// Transitions to a new state, emitting a transition event.
            pub fn transition(&mut self, state: impl Into<#transition_enum>) {
                self.emit_transition(state.into());
            }

            /// Explicitly exits the FSM, emitting the exit event.
            pub fn exit(&mut self) {
                if !self.exited {
                    self.emit_transition(#transition_enum::Exit);
                    self.exited = true;
                }
            }

            fn emit_transition(&mut self, state: #transition_enum) {
                let seq = self.seq;
                self.seq += 1;
                let event = quent_model::FsmEvent::Transition { seq, state };
                self.tx.send(quent_model::Event::new(
                    self.id,
                    quent_model::timestamp(),
                    E::from(event),
                ));
            }

            fn emit_deferred(&mut self, deferred: #deferred_enum) {
                let seq = self.seq;
                self.seq += 1;
                let event = quent_model::FsmEvent::Deferred { seq, deferred };
                self.tx.send(quent_model::Event::new(
                    self.id,
                    quent_model::timestamp(),
                    E::from(event),
                ));
            }
        }

        impl<E> Drop for #handle_name<E>
        where
            E: From<#event_type> + serde::Serialize + Send + std::fmt::Debug + 'static,
        {
            fn drop(&mut self) {
                self.exit();
            }
        }

        // --- Resource marker (if resource directive was present) ---
        #resource_impl
    };

    Ok(output)
}
