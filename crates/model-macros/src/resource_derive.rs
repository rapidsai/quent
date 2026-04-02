// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `#[derive(Resource)]` and `#[derive(ResizableResource)]` implementations.
//!
//! Generates the full resource FSM from a simple struct definition:
//! - Initializing state (with instance_name, parent_group_id, resource_type_name + user init fields)
//! - Operating state (with `Capacity<V, K>` fields)
//! - Finalizing state (unit struct)
//! - Resizing state (ResizableResource only)
//! - FSM transition table, handle, event types
//! - Resource trait impl, ModelComponent, TransitionInfo, HasEventType

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{DeriveInput, Field};

use crate::util::to_snake_case;

/// Check if a field's type is `Capacity<...>`.
fn is_capacity_field(field: &Field) -> bool {
    if let syn::Type::Path(type_path) = &field.ty {
        type_path.path.segments.last()
            .is_some_and(|seg| seg.ident == "Capacity")
    } else {
        false
    }
}

/// Categorize fields into init fields (non-capacity) and capacity fields.
struct ResourceFields<'a> {
    /// Fields that go on the Initializing state (non-capacity).
    init_fields: Vec<&'a Field>,
    /// Fields that go on the Operating state (Capacity<V, K> type).
    capacity_fields: Vec<&'a Field>,
}

fn categorize_fields<'a>(input: &'a DeriveInput) -> syn::Result<ResourceFields<'a>> {
    let mut init_fields = Vec::new();
    let mut capacity_fields = Vec::new();

    match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(named) => {
                for field in &named.named {
                    if is_capacity_field(field) {
                        capacity_fields.push(field);
                    } else {
                        init_fields.push(field);
                    }
                }
            }
            syn::Fields::Unit => {} // Unit struct = unit resource (no capacity, no init fields)
            _ => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "Resource derive requires named fields or unit struct",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Resource can only be derived on structs",
            ));
        }
    }

    Ok(ResourceFields {
        init_fields,
        capacity_fields,
    })
}

pub fn expand_resource(input: DeriveInput) -> syn::Result<TokenStream> {
    expand_impl(input, false)
}

pub fn expand_resizable_resource(input: DeriveInput) -> syn::Result<TokenStream> {
    expand_impl(input, true)
}

fn expand_impl(input: DeriveInput, resizable: bool) -> syn::Result<TokenStream> {
    let vis = &input.vis;
    let name = &input.ident;
    let name_snake = to_snake_case(name);

    let fields = categorize_fields(&input)?;

    // Names for generated types
    let init_state = format_ident!("{}Initializing", name);
    let op_state = format_ident!("{}Operating", name);
    let fin_state = format_ident!("{}Finalizing", name);
    let resize_state = format_ident!("{}Resizing", name);
    let transition_enum = format_ident!("{}Transition", name);
    let deferred_enum = format_ident!("{}Deferred", name);
    let event_type = format_ident!("{}Event", name);
    let handle_name = format_ident!("{}Handle", name);
    let resource_marker = format_ident!("{}Resource", name);

    // Generate init state fields: standard metadata + user init fields
    let user_init_field_defs: Vec<TokenStream> = fields
        .init_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            quote! { pub #ident: #ty }
        })
        .collect();

    let user_init_field_names: Vec<&Ident> = fields
        .init_fields
        .iter()
        .filter_map(|f| f.ident.as_ref())
        .collect();

    // Generate operating state fields (capacity fields only)
    let capacity_field_defs: Vec<TokenStream> = fields
        .capacity_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            let attrs = &f.attrs;
            quote! { #(#attrs)* pub #ident: #ty }
        })
        .collect();

    let capacity_field_names: Vec<&Ident> = fields
        .capacity_fields
        .iter()
        .filter_map(|f| f.ident.as_ref())
        .collect();

    // Generate ExtractCapacities for the operating state
    // Capacity fields are `Capacity<V, K>` — access inner value via `.value`
    let capacity_extractions: Vec<TokenStream> = fields
        .capacity_fields
        .iter()
        .filter_map(|f| {
            let ident = f.ident.as_ref()?;
            let name_str = ident.to_string();
            // Check if the Capacity's inner value type (V) is Option<T>
            // by inspecting the first type argument of Capacity<V, K>
            let inner_is_option = if let syn::Type::Path(tp) = &f.ty {
                tp.path.segments.last().and_then(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.first().and_then(|arg| {
                            if let syn::GenericArgument::Type(syn::Type::Path(inner_tp)) = arg {
                                inner_tp.path.segments.last()
                                    .map(|s| s.ident == "Option")
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                }).unwrap_or(false)
            } else {
                false
            };
            if inner_is_option {
                Some(quote! {
                    quent_model::analyze::ExtractedCapacity {
                        name: #name_str,
                        value: self.#ident.value.map(|v| v as u64),
                    }
                })
            } else {
                Some(quote! {
                    quent_model::analyze::ExtractedCapacity::new(#name_str, self.#ident.value as u64)
                })
            }
        })
        .collect();

    let extract_capacities_body = if capacity_extractions.is_empty() {
        quote! { vec![quent_model::analyze::ExtractedCapacity::unit()] }
    } else {
        quote! { vec![#(#capacity_extractions,)*] }
    };

    // Operating state definition
    let op_state_def = if capacity_field_defs.is_empty() {
        // Unit resource — empty operating state
        quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            #vis struct #op_state;

            impl quent_model::State for #op_state {}

            impl quent_model::StateMetadata for #op_state {
                type Deferred = #deferred_enum;
                fn state_name() -> &'static str { "operating" }
                fn state_def() -> quent_model::StateDef {
                    quent_model::StateDef {
                        name: "operating".to_string(),
                        attributes: vec![],
                        deferred_attributes: vec![],
                        usages: vec![],
                    }
                }
            }

            impl quent_model::analyze::ExtractCapacities for #op_state {
                fn extract_capacities(&self) -> Vec<quent_model::analyze::ExtractedCapacity> {
                    vec![quent_model::analyze::ExtractedCapacity::unit()]
                }
            }

            impl quent_model::analyze::ExtractUsages for #op_state {
                fn extract_usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> { vec![] }
            }

            impl quent_model::analyze::ExtractInstanceName for #op_state {
                fn extract_instance_name(&self) -> Option<&str> { None }
            }

            impl quent_model::analyze::ExtractParentGroupId for #op_state {
                fn extract_parent_group_id(&self) -> Option<uuid::Uuid> { None }
            }
        }
    } else {
        quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            #vis struct #op_state {
                #(#capacity_field_defs,)*
            }

            impl quent_model::State for #op_state {}

            impl quent_model::StateMetadata for #op_state {
                type Deferred = #deferred_enum;
                fn state_name() -> &'static str { "operating" }
                fn state_def() -> quent_model::StateDef {
                    quent_model::StateDef {
                        name: "operating".to_string(),
                        attributes: vec![],
                        deferred_attributes: vec![],
                        usages: vec![],
                    }
                }
            }

            impl quent_model::analyze::ExtractCapacities for #op_state {
                fn extract_capacities(&self) -> Vec<quent_model::analyze::ExtractedCapacity> {
                    #extract_capacities_body
                }
            }

            impl quent_model::analyze::ExtractUsages for #op_state {
                fn extract_usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> { vec![] }
            }

            impl quent_model::analyze::ExtractInstanceName for #op_state {
                fn extract_instance_name(&self) -> Option<&str> { None }
            }

            impl quent_model::analyze::ExtractParentGroupId for #op_state {
                fn extract_parent_group_id(&self) -> Option<uuid::Uuid> { None }
            }
        }
    };

    // Transition variants and FSM structure
    let (transition_variants, transition_name_arms, transition_usages_arms,
         transition_instance_name_arms, transition_parent_group_id_arms,
         transition_defs, state_defs, from_impls,
         handle_transition_methods, resizing_code) = if resizable {
        // ResizableResource: init -> operating <-> resizing -> finalizing -> exit
        let variants = quote! {
            #init_state(#init_state),
            #op_state(#op_state),
            #resize_state(#resize_state),
            #fin_state(#fin_state),
            Exit,
        };

        let name_arms = quote! {
            #transition_enum::#init_state(_) => "initializing",
            #transition_enum::#op_state(_) => "operating",
            #transition_enum::#resize_state(_) => "resizing",
            #transition_enum::#fin_state(_) => "finalizing",
            #transition_enum::Exit => "exit",
        };

        let usages_arms = quote! {
            #transition_enum::#init_state(data) => quent_model::analyze::ExtractUsages::extract_usages(data),
            #transition_enum::#op_state(data) => quent_model::analyze::ExtractUsages::extract_usages(data),
            #transition_enum::#resize_state(data) => quent_model::analyze::ExtractUsages::extract_usages(data),
            #transition_enum::#fin_state(data) => quent_model::analyze::ExtractUsages::extract_usages(data),
            #transition_enum::Exit => vec![],
        };

        let instance_name_arms = quote! {
            #transition_enum::#init_state(data) => quent_model::analyze::ExtractInstanceName::extract_instance_name(data),
            #transition_enum::#op_state(data) => quent_model::analyze::ExtractInstanceName::extract_instance_name(data),
            #transition_enum::#resize_state(data) => quent_model::analyze::ExtractInstanceName::extract_instance_name(data),
            #transition_enum::#fin_state(data) => quent_model::analyze::ExtractInstanceName::extract_instance_name(data),
            #transition_enum::Exit => None,
        };

        let parent_group_id_arms = quote! {
            #transition_enum::#init_state(data) => quent_model::analyze::ExtractParentGroupId::extract_parent_group_id(data),
            #transition_enum::#op_state(data) => quent_model::analyze::ExtractParentGroupId::extract_parent_group_id(data),
            #transition_enum::#resize_state(data) => quent_model::analyze::ExtractParentGroupId::extract_parent_group_id(data),
            #transition_enum::#fin_state(data) => quent_model::analyze::ExtractParentGroupId::extract_parent_group_id(data),
            #transition_enum::Exit => None,
        };

        let tdefs = quote! {
            quent_model::TransitionDef { from: quent_model::TransitionEndpoint::Entry, to: quent_model::TransitionEndpoint::State("initializing".to_string()) },
            quent_model::TransitionDef { from: quent_model::TransitionEndpoint::State("initializing".to_string()), to: quent_model::TransitionEndpoint::State("operating".to_string()) },
            quent_model::TransitionDef { from: quent_model::TransitionEndpoint::State("operating".to_string()), to: quent_model::TransitionEndpoint::State("resizing".to_string()) },
            quent_model::TransitionDef { from: quent_model::TransitionEndpoint::State("resizing".to_string()), to: quent_model::TransitionEndpoint::State("operating".to_string()) },
            quent_model::TransitionDef { from: quent_model::TransitionEndpoint::State("operating".to_string()), to: quent_model::TransitionEndpoint::State("finalizing".to_string()) },
            quent_model::TransitionDef { from: quent_model::TransitionEndpoint::State("finalizing".to_string()), to: quent_model::TransitionEndpoint::Exit },
        };

        let sdefs = quote! {
            <#init_state as quent_model::StateMetadata>::state_def(),
            <#op_state as quent_model::StateMetadata>::state_def(),
            <#resize_state as quent_model::StateMetadata>::state_def(),
            <#fin_state as quent_model::StateMetadata>::state_def(),
        };

        let froms = quote! {
            impl From<#init_state> for #transition_enum { fn from(s: #init_state) -> Self { #transition_enum::#init_state(s) } }
            impl From<#op_state> for #transition_enum { fn from(s: #op_state) -> Self { #transition_enum::#op_state(s) } }
            impl From<#resize_state> for #transition_enum { fn from(s: #resize_state) -> Self { #transition_enum::#resize_state(s) } }
            impl From<#fin_state> for #transition_enum { fn from(s: #fin_state) -> Self { #transition_enum::#fin_state(s) } }
        };

        let methods = quote! {
            pub fn operating(&mut self, state: #op_state) { self.transition(state); }
            pub fn resizing(&mut self, state: #resize_state) { self.transition(state); }
            pub fn finalizing(&mut self) { self.transition(#fin_state); }
        };

        let resize_code = quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            #vis struct #resize_state;

            impl quent_model::State for #resize_state {}

            impl quent_model::StateMetadata for #resize_state {
                type Deferred = #deferred_enum;
                fn state_name() -> &'static str { "resizing" }
                fn state_def() -> quent_model::StateDef {
                    quent_model::StateDef {
                        name: "resizing".to_string(),
                        attributes: vec![],
                        deferred_attributes: vec![],
                        usages: vec![],
                    }
                }
            }

            impl quent_model::analyze::ExtractCapacities for #resize_state {
                fn extract_capacities(&self) -> Vec<quent_model::analyze::ExtractedCapacity> { vec![] }
            }
            impl quent_model::analyze::ExtractUsages for #resize_state {
                fn extract_usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> { vec![] }
            }
            impl quent_model::analyze::ExtractInstanceName for #resize_state {
                fn extract_instance_name(&self) -> Option<&str> { None }
            }
            impl quent_model::analyze::ExtractParentGroupId for #resize_state {
                fn extract_parent_group_id(&self) -> Option<uuid::Uuid> { None }
            }
        };

        (variants, name_arms, usages_arms, instance_name_arms, parent_group_id_arms, tdefs, sdefs, froms, methods, resize_code)
    } else {
        // Fixed resource: init -> operating -> finalizing -> exit
        let variants = quote! {
            #init_state(#init_state),
            #op_state(#op_state),
            #fin_state(#fin_state),
            Exit,
        };

        let name_arms = quote! {
            #transition_enum::#init_state(_) => "initializing",
            #transition_enum::#op_state(_) => "operating",
            #transition_enum::#fin_state(_) => "finalizing",
            #transition_enum::Exit => "exit",
        };

        let usages_arms = quote! {
            #transition_enum::#init_state(data) => quent_model::analyze::ExtractUsages::extract_usages(data),
            #transition_enum::#op_state(data) => quent_model::analyze::ExtractUsages::extract_usages(data),
            #transition_enum::#fin_state(data) => quent_model::analyze::ExtractUsages::extract_usages(data),
            #transition_enum::Exit => vec![],
        };

        let instance_name_arms = quote! {
            #transition_enum::#init_state(data) => quent_model::analyze::ExtractInstanceName::extract_instance_name(data),
            #transition_enum::#op_state(data) => quent_model::analyze::ExtractInstanceName::extract_instance_name(data),
            #transition_enum::#fin_state(data) => quent_model::analyze::ExtractInstanceName::extract_instance_name(data),
            #transition_enum::Exit => None,
        };

        let parent_group_id_arms = quote! {
            #transition_enum::#init_state(data) => quent_model::analyze::ExtractParentGroupId::extract_parent_group_id(data),
            #transition_enum::#op_state(data) => quent_model::analyze::ExtractParentGroupId::extract_parent_group_id(data),
            #transition_enum::#fin_state(data) => quent_model::analyze::ExtractParentGroupId::extract_parent_group_id(data),
            #transition_enum::Exit => None,
        };

        let tdefs = quote! {
            quent_model::TransitionDef { from: quent_model::TransitionEndpoint::Entry, to: quent_model::TransitionEndpoint::State("initializing".to_string()) },
            quent_model::TransitionDef { from: quent_model::TransitionEndpoint::State("initializing".to_string()), to: quent_model::TransitionEndpoint::State("operating".to_string()) },
            quent_model::TransitionDef { from: quent_model::TransitionEndpoint::State("operating".to_string()), to: quent_model::TransitionEndpoint::State("finalizing".to_string()) },
            quent_model::TransitionDef { from: quent_model::TransitionEndpoint::State("finalizing".to_string()), to: quent_model::TransitionEndpoint::Exit },
        };

        let sdefs = quote! {
            <#init_state as quent_model::StateMetadata>::state_def(),
            <#op_state as quent_model::StateMetadata>::state_def(),
            <#fin_state as quent_model::StateMetadata>::state_def(),
        };

        let froms = quote! {
            impl From<#init_state> for #transition_enum { fn from(s: #init_state) -> Self { #transition_enum::#init_state(s) } }
            impl From<#op_state> for #transition_enum { fn from(s: #op_state) -> Self { #transition_enum::#op_state(s) } }
            impl From<#fin_state> for #transition_enum { fn from(s: #fin_state) -> Self { #transition_enum::#fin_state(s) } }
        };

        let methods = quote! {
            pub fn operating(&mut self, state: #op_state) { self.transition(state); }
            pub fn finalizing(&mut self) { self.transition(#fin_state); }
        };

        (variants, name_arms, usages_arms, instance_name_arms, parent_group_id_arms, tdefs, sdefs, froms, methods, quote! {})
    };

    let output = quote! {
        // --- Initializing state ---
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #vis struct #init_state {
            pub instance_name: String,
            pub parent_group_id: uuid::Uuid,
            pub resource_type_name: String,
            #(#user_init_field_defs,)*
        }

        impl quent_model::State for #init_state {}

        impl quent_model::StateMetadata for #init_state {
            type Deferred = #deferred_enum;
            fn state_name() -> &'static str { "initializing" }
            fn state_def() -> quent_model::StateDef {
                quent_model::StateDef {
                    name: "initializing".to_string(),
                    attributes: vec![],
                    deferred_attributes: vec![],
                    usages: vec![],
                }
            }
        }

        impl quent_model::analyze::ExtractCapacities for #init_state {
            fn extract_capacities(&self) -> Vec<quent_model::analyze::ExtractedCapacity> { vec![] }
        }
        impl quent_model::analyze::ExtractUsages for #init_state {
            fn extract_usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> { vec![] }
        }
        impl quent_model::analyze::ExtractInstanceName for #init_state {
            fn extract_instance_name(&self) -> Option<&str> { Some(&self.instance_name) }
        }
        impl quent_model::analyze::ExtractParentGroupId for #init_state {
            fn extract_parent_group_id(&self) -> Option<uuid::Uuid> { Some(self.parent_group_id) }
        }

        // --- Operating state ---
        #op_state_def

        // --- Finalizing state ---
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #vis struct #fin_state;

        impl quent_model::State for #fin_state {}

        impl quent_model::StateMetadata for #fin_state {
            type Deferred = #deferred_enum;
            fn state_name() -> &'static str { "finalizing" }
            fn state_def() -> quent_model::StateDef {
                quent_model::StateDef {
                    name: "finalizing".to_string(),
                    attributes: vec![],
                    deferred_attributes: vec![],
                    usages: vec![],
                }
            }
        }

        impl quent_model::analyze::ExtractCapacities for #fin_state {
            fn extract_capacities(&self) -> Vec<quent_model::analyze::ExtractedCapacity> { vec![] }
        }
        impl quent_model::analyze::ExtractUsages for #fin_state {
            fn extract_usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> { vec![] }
        }
        impl quent_model::analyze::ExtractInstanceName for #fin_state {
            fn extract_instance_name(&self) -> Option<&str> { None }
        }
        impl quent_model::analyze::ExtractParentGroupId for #fin_state {
            fn extract_parent_group_id(&self) -> Option<uuid::Uuid> { None }
        }

        // --- Resizing state (ResizableResource only) ---
        #resizing_code

        // --- Deferred enum (empty — resources have no deferred fields) ---
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #vis enum #deferred_enum {}

        // --- Transition enum ---
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #vis enum #transition_enum {
            #transition_variants
        }

        #from_impls

        // --- TransitionInfo ---
        impl quent_model::analyze::TransitionInfo for #transition_enum {
            fn state_name(&self) -> &'static str {
                match self { #transition_name_arms }
            }
            fn usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> {
                match self { #transition_usages_arms }
            }
            fn instance_name(&self) -> Option<&str> {
                match self { #transition_instance_name_arms }
            }
            fn parent_group_id(&self) -> Option<uuid::Uuid> {
                match self { #transition_parent_group_id_arms }
            }
            fn fsm_type_name() -> &'static str { #name_snake }
            fn collect_model(builder: &mut quent_model::ModelBuilder) {
                <#name as quent_model::ModelComponent>::collect(builder);
            }
        }

        // --- Event type alias ---
        #vis type #event_type = quent_model::FsmEvent<#transition_enum, #deferred_enum>;

        // --- HasEventType ---
        impl quent_model::HasEventType for #name {
            type Event = quent_model::FsmEvent<#transition_enum, #deferred_enum>;
        }

        // --- Resource marker ---
        #vis struct #resource_marker;

        impl quent_model::Resource for #resource_marker {
            type CapacityValue = #op_state;
        }

        // --- ModelComponent ---
        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_fsm(quent_model::FsmDef {
                    name: #name_snake.to_string(),
                    states: vec![#state_defs],
                    transitions: vec![#transition_defs],
                });
            }
        }

        // --- Handle ---
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
            pub fn new(tx: &quent_model::EventSender<E>, initial_state: #init_state) -> Self {
                let id = uuid::Uuid::now_v7();
                let mut handle = Self { id, seq: 0, exited: false, tx: tx.clone() };
                handle.emit_transition(#transition_enum::from(initial_state));
                handle
            }

            pub fn uuid(&self) -> uuid::Uuid { self.id }

            #handle_transition_methods

            pub fn exit(&mut self) {
                if !self.exited {
                    self.emit_transition(#transition_enum::Exit);
                    self.exited = true;
                }
            }

            fn transition(&mut self, state: impl Into<#transition_enum>) {
                self.emit_transition(state.into());
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
        }

        impl<E> Drop for #handle_name<E>
        where
            E: From<#event_type> + serde::Serialize + Send + std::fmt::Debug + 'static,
        {
            fn drop(&mut self) { self.exit(); }
        }
    };

    Ok(output)
}
