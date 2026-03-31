// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeSet, HashMap, HashSet};

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Token, punctuated::Punctuated};

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

/// The full transition table: `entry -> A, A -> B, B -> exit`
struct TransitionTable {
    transitions: Punctuated<Transition, Token![,]>,
}

impl Parse for TransitionTable {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let transitions = Punctuated::parse_terminated(input)?;
        Ok(TransitionTable { transitions })
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

/// Checks if the struct has a `#[quent_model::resource(capacity = X)]` attr
/// and returns the capacity state ident if found.
fn extract_resource_attr(item: &syn::ItemStruct) -> Option<Ident> {
    for attr in &item.attrs {
        let path = attr.path();
        let last_seg = path.segments.last()?;
        if last_seg.ident == "resource" {
            if let Ok(parsed) = attr.parse_args::<ResourceCapacityArg>() {
                return Some(parsed.capacity_state);
            }
        }
    }
    None
}

struct ResourceCapacityArg {
    capacity_state: Ident,
}

impl Parse for ResourceCapacityArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "capacity" {
            return Err(syn::Error::new_spanned(key, "expected `capacity`"));
        }
        input.parse::<Token![=]>()?;
        let capacity_state: Ident = input.parse()?;
        Ok(ResourceCapacityArg { capacity_state })
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let table: TransitionTable = syn::parse2(attr)?;
    let input: syn::ItemStruct = syn::parse2(item)?;
    let vis = &input.vis;
    let fsm_name = &input.ident;

    // Check for co-located #[resource] attribute
    let resource_capacity = extract_resource_attr(&input);

    let transitions: Vec<Transition> = table.transitions.into_iter().collect();
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

    // Optionally generate Resource trait impl
    let resource_impl = resource_capacity.map(|cap_state| {
        quote! {
            impl quent_model::Resource for #fsm_name {
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

        // --- ModelComponent impl ---
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
        #vis struct #fsm_name {
            id: uuid::Uuid,
            seq: u64,
            exited: bool,
        }

        // --- Resource impl (if #[resource] attribute was present) ---
        #resource_impl
    };

    Ok(output)
}
