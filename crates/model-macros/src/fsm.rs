// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeSet, HashMap, HashSet};

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::util::{field_has_attr, parse_resource_group_attr, to_snake_case};

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

/// Validates the FSM transition graph.
fn validate_fsm(fsm_name: &Ident, transitions: &[Transition]) -> syn::Result<()> {
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
            "FSM must have at least one `entry -> State` transition (use #[entry] on a field)",
        ));
    }
    if !has_exit_target {
        return Err(syn::Error::new_spanned(
            fsm_name,
            "FSM must have at least one `State -> exit` transition (use #[to(..., exit)] on a field)",
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
        forward
            .entry(from_key.clone())
            .or_default()
            .push(to_key.clone());
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

/// Parse `#[to(StateA, StateB, exit)]` attribute on a field.
fn parse_to_attr(field: &syn::Field) -> syn::Result<Vec<Ident>> {
    for attr in &field.attrs {
        if attr
            .path()
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "to")
        {
            let targets: syn::punctuated::Punctuated<Ident, syn::Token![,]> =
                attr.parse_args_with(syn::punctuated::Punctuated::parse_terminated)?;
            return Ok(targets.into_iter().collect());
        }
    }
    Ok(vec![])
}

/// Extract the type ident from a field's type (the last segment of the path).
fn type_ident(ty: &syn::Type) -> syn::Result<Ident> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            return Ok(seg.ident.clone());
        }
    }
    Err(syn::Error::new_spanned(
        ty,
        "expected a simple type path for FSM state field",
    ))
}

/// Check if a struct-level attribute is `#[resource(capacity = T)]`.
fn parse_resource_attr(input: &DeriveInput) -> syn::Result<Option<Ident>> {
    for attr in &input.attrs {
        if attr
            .path()
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "resource")
        {
            let mut cap: Option<Ident> = None;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("capacity") {
                    let value = meta.value()?;
                    cap = Some(value.parse::<Ident>()?);
                    Ok(())
                } else {
                    Err(meta.error("expected `capacity`"))
                }
            })?;
            return Ok(cap);
        }
    }
    Ok(None)
}

/// Expand the Fsm derive macro.
///
/// Parses struct fields with `#[entry]` and `#[to(...)]` attributes to build
/// the transition table, then generates transition enum, deferred enum, event
/// type alias, handle struct, ModelComponent impl, TransitionInfo impl, and
/// HasEventType impl.
///
/// Does NOT re-emit the struct (derive macros append).
pub fn expand_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let vis = &input.vis;
    let fsm_name = &input.ident;

    // Parse fields to extract state types and transitions
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(named) => &named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    fsm_name,
                    "Fsm derive requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                fsm_name,
                "Fsm can only be derived on structs",
            ));
        }
    };

    // Validate all fields are pub
    for field in fields {
        if !matches!(field.vis, syn::Visibility::Public(_)) {
            return Err(syn::Error::new_spanned(
                field,
                "FSM fields must be `pub` — they are part of the generated instrumentation API",
            ));
        }
    }

    // Parse resource(capacity = T) from outer attributes
    let resource_capacity = parse_resource_attr(&input)?;

    // Parse resource_group from outer attributes
    let resource_group = parse_resource_group_attr(&input);

    // Collect state idents and transitions from fields
    let mut transitions = Vec::new();
    let mut state_idents: Vec<Ident> = Vec::new();
    let mut entry_state_type: Option<Ident> = None;
    let mut entry_state_field_name: Option<String> = None;
    let mut seen = HashSet::new();

    for field in fields {
        let state_type = type_ident(&field.ty)?;
        let is_entry = field_has_attr(field, "entry");
        let to_targets = parse_to_attr(field)?;

        // Track state ordering
        if seen.insert(state_type.to_string()) {
            state_idents.push(state_type.clone());
        }

        // Entry transition: entry -> this state
        if is_entry {
            entry_state_type = Some(state_type.clone());
            entry_state_field_name = field.ident.as_ref().map(|i| i.to_string());
            transitions.push(Transition {
                from: TransitionEndpoint::Entry,
                to: TransitionEndpoint::State(state_type.clone()),
            });
        }

        // Outgoing transitions: this state -> each target
        for target in &to_targets {
            let to = if target == "exit" {
                TransitionEndpoint::Exit
            } else {
                TransitionEndpoint::State(target.clone())
            };
            transitions.push(Transition {
                from: TransitionEndpoint::State(state_type.clone()),
                to,
            });
        }
    }

    validate_fsm(fsm_name, &transitions)?;

    let entry_state_type = entry_state_type.expect("validate_fsm ensures entry exists");
    let entry_constructor = format_ident!(
        "{}",
        entry_state_field_name.expect("entry field has a name")
    );

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

    // Generate TransitionInfo impl arms
    let transition_name_arms: Vec<TokenStream> = state_idents
        .iter()
        .map(|ident| {
            let name = to_snake_case(ident);
            quote! { #transition_enum::#ident(_) => #name }
        })
        .collect();

    let transition_usages_arms: Vec<TokenStream> = state_idents
        .iter()
        .map(|ident| {
            quote! {
                #transition_enum::#ident(data) => quent_model::analyze::ExtractUsages::extract_usages(data)
            }
        })
        .collect();

    let transition_instance_name_arms: Vec<TokenStream> = state_idents
        .iter()
        .map(|ident| {
            quote! {
                #transition_enum::#ident(data) => quent_model::analyze::ExtractInstanceName::extract_instance_name(data)
            }
        })
        .collect();

    let transition_parent_group_id_arms: Vec<TokenStream> = state_idents
        .iter()
        .map(|ident| {
            quote! {
                #transition_enum::#ident(data) => quent_model::analyze::ExtractParentGroupId::extract_parent_group_id(data)
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

    // Resource group contribution
    let rg_contribution = if let Some(is_root) = resource_group {
        quote! {
            builder.add_resource_group(quent_model::ResourceGroupDef {
                name: #fsm_snake.to_string(),
                fixed_parent: None,
                is_root: #is_root,
            });
        }
    } else {
        quote! {}
    };

    let rg_trait_impl = if let Some(is_root) = resource_group {
        quote! {
            impl quent_model::ResourceGroup for #fsm_name {
                const IS_ROOT: bool = #is_root;
            }
        }
    } else {
        quote! {}
    };

    // Compile-time enforcement: non-root resource group FSMs must have
    // a #[parent_group] field on their entry state.
    let rg_parent_group_assert = if resource_group == Some(false) {
        let entry_type = &entry_state_type;
        quote! {
            const _: () = {
                fn _assert_entry_has_parent_group<T: quent_model::HasParentGroup>() {}
                fn _check() {
                    _assert_entry_has_parent_group::<#entry_type>();
                }
            };
        }
    } else {
        quote! {}
    };

    let output = quote! {
        // --- Transition enum ---
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #vis enum #transition_enum {
            #(#transition_variants,)*
            Exit,
        }

        #(#from_impls)*

        // --- TransitionInfo impl ---
        impl quent_model::analyze::TransitionInfo for #transition_enum {
            fn state_name(&self) -> &'static str {
                match self {
                    #(#transition_name_arms,)*
                    #transition_enum::Exit => "exit",
                }
            }

            fn usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> {
                match self {
                    #(#transition_usages_arms,)*
                    #transition_enum::Exit => vec![],
                }
            }

            fn instance_name(&self) -> Option<&str> {
                match self {
                    #(#transition_instance_name_arms,)*
                    #transition_enum::Exit => None,
                }
            }

            fn parent_group_id(&self) -> Option<uuid::Uuid> {
                match self {
                    #(#transition_parent_group_id_arms,)*
                    #transition_enum::Exit => None,
                }
            }

            fn fsm_type_name() -> &'static str {
                #fsm_snake
            }

            fn collect_model(builder: &mut quent_model::ModelBuilder) {
                <#fsm_name as quent_model::ModelComponent>::collect(builder);
            }
        }

        // --- Deferred enum ---
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #vis enum #deferred_enum {
            #(#state_idents(<#state_idents as quent_model::StateMetadata>::Deferred),)*
        }

        // --- Event type alias ---
        #vis type #event_type = quent_model::FsmEvent<#transition_enum, #deferred_enum>;

        // --- ModelComponent impl (on the original struct) ---
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
                #rg_contribution
            }
        }

        // --- The FSM handle struct ---
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
            pub fn #entry_constructor(tx: &quent_model::EventSender<E>, state: #entry_state_type) -> Self {
                let id = uuid::Uuid::now_v7();
                let mut handle = Self {
                    id,
                    seq: 0,
                    exited: false,
                    tx: tx.clone(),
                };
                handle.emit_transition(state.into());
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

            // TODO: add debug_assert for transition validation. The valid
            // transitions are known at codegen time from the #[to(...)]
            // attributes. Embedding them as a static lookup table here would
            // allow runtime checks (in debug builds) that the sequence of
            // states follows the declared transition graph.
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

        // --- HasEventType impl ---
        impl quent_model::HasEventType for #fsm_name {
            type Event = quent_model::FsmEvent<#transition_enum, #deferred_enum>;
        }

        // --- ResourceGroup trait impl (if applicable) ---
        #rg_trait_impl

        // --- Compile-time parent group assertion (non-root resource groups) ---
        #rg_parent_group_assert

        // --- Resource marker (if resource directive was present) ---
        #resource_impl

    };

    Ok(output)
}
