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

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Field};

use crate::util::to_snake_case;

/// Generate StateMetadata + Extract* impls for a unit state (no data fields).
fn emit_unit_state_impls(
    state_ident: &proc_macro2::Ident,
    state_name: &str,
    extract_capacities: TokenStream,
    extract_instance_name: TokenStream,
    extract_parent_group_id: TokenStream,
) -> TokenStream {
    quote! {
        impl quent_model::StateMetadata for #state_ident {
            fn state_name() -> &'static str { #state_name }
            fn state_def() -> quent_model::StateDef {
                quent_model::StateDef {
                    name: #state_name.to_string(),
                    attributes: vec![],
                    usages: vec![],
                }
            }
        }

        impl quent_model::analyze::ExtractCapacities for #state_ident {
            fn extract_capacities(&self) -> Vec<quent_model::analyze::ExtractedCapacity> {
                #extract_capacities
            }
        }

        impl quent_model::analyze::ExtractUsages for #state_ident {
            fn extract_usages(&self) -> Vec<quent_model::analyze::ExtractedUsage> { vec![] }
        }

        impl quent_model::analyze::ExtractInstanceName for #state_ident {
            fn extract_instance_name(&self) -> Option<&str> { #extract_instance_name }
        }

        impl quent_model::analyze::ExtractParentGroupId for #state_ident {
            fn extract_parent_group_id(&self) -> Option<uuid::Uuid> { #extract_parent_group_id }
        }
    }
}

/// Check if a field's type is `Capacity<...>`.
fn is_capacity_field(field: &Field) -> bool {
    crate::util::is_capacity_type(&field.ty)
}

/// Extract the inner value type V from `Capacity<V>` or `Capacity<V, K>`.
fn extract_capacity_inner(ty: &syn::Type) -> Option<syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let seg = type_path.path.segments.last()?;
    if seg.ident != "Capacity" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };
    let syn::GenericArgument::Type(inner) = args.args.first()? else {
        return None;
    };
    Some(inner.clone())
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
    let serde_derives = crate::util::serde_derives();
    let serde_bound = crate::util::serde_bound();
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
    let event_type = format_ident!("{}Event", name);
    let handle_name = format_ident!("{}Handle", name);
    let observer_name = format_ident!("{}Observer", name);
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

    let user_init_field_names: Vec<&proc_macro2::Ident> = fields
        .init_fields
        .iter()
        .filter_map(|f| f.ident.as_ref())
        .collect();

    let user_init_param_defs: Vec<TokenStream> = fields
        .init_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            quote! { #ident: #ty }
        })
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
        let impls = emit_unit_state_impls(
            &op_state,
            "operating",
            quote! { vec![quent_model::analyze::ExtractedCapacity::unit()] },
            quote! { None },
            quote! { None },
        );
        quote! {
            #[derive(#serde_derives)]
            #vis struct #op_state;
            #impls
        }
    } else {
        let impls = emit_unit_state_impls(
            &op_state,
            "operating",
            extract_capacities_body,
            quote! { None },
            quote! { None },
        );
        quote! {
            #[derive(#serde_derives)]
            #vis struct #op_state {
                #(#capacity_field_defs,)*
            }
            #impls
        }
    };

    // Generate flat operating method parameters from capacity fields
    let operating_params: Vec<TokenStream> = fields
        .capacity_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let inner_ty =
                extract_capacity_inner(&f.ty).expect("capacity field must be Capacity<V, K>");
            quote! { #ident: #inner_ty }
        })
        .collect();

    let operating_field_inits: Vec<TokenStream> = fields
        .capacity_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            quote! { #ident: quent_model::Capacity::new(#ident) }
        })
        .collect();

    // Transition variants and FSM structure
    let (
        transition_variants,
        transition_name_arms,
        transition_usages_arms,
        transition_instance_name_arms,
        transition_parent_group_id_arms,
        transition_defs,
        state_defs,
        from_impls,
        handle_transition_methods,
        resizing_code,
    ) = if resizable {
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
            pub fn operating(&mut self, #(#operating_params,)*) {
                self.transition(#op_state { #(#operating_field_inits,)* });
            }
            pub fn resizing(&mut self) { self.transition(#resize_state); }
            pub fn finalizing(&mut self) { self.transition(#fin_state); }
        };

        let resize_impls = emit_unit_state_impls(
            &resize_state,
            "resizing",
            quote! { vec![] },
            quote! { None },
            quote! { None },
        );
        let resize_code = quote! {
            #[derive(#serde_derives)]
            #vis struct #resize_state;
            #resize_impls
        };

        (
            variants,
            name_arms,
            usages_arms,
            instance_name_arms,
            parent_group_id_arms,
            tdefs,
            sdefs,
            froms,
            methods,
            resize_code,
        )
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

        let methods = if operating_params.is_empty() {
            quote! {
                pub fn operating(&mut self) { self.transition(#op_state); }
                pub fn finalizing(&mut self) { self.transition(#fin_state); }
            }
        } else {
            quote! {
                pub fn operating(&mut self, #(#operating_params,)*) {
                    self.transition(#op_state { #(#operating_field_inits,)* });
                }
                pub fn finalizing(&mut self) { self.transition(#fin_state); }
            }
        };

        (
            variants,
            name_arms,
            usages_arms,
            instance_name_arms,
            parent_group_id_arms,
            tdefs,
            sdefs,
            froms,
            methods,
            quote! {},
        )
    };

    let init_state_impls = emit_unit_state_impls(
        &init_state,
        "initializing",
        quote! { vec![] },
        quote! { Some(&self.instance_name) },
        quote! { Some(self.parent_group_id) },
    );

    let fin_state_impls = emit_unit_state_impls(
        &fin_state,
        "finalizing",
        quote! { vec![] },
        quote! { None },
        quote! { None },
    );

    let output = quote! {
        #[derive(#serde_derives)]
        #vis struct #init_state {
            pub instance_name: String,
            pub parent_group_id: uuid::Uuid,
            pub resource_type_name: String,
            #(#user_init_field_defs,)*
        }

        #init_state_impls

        #op_state_def

        #[derive(#serde_derives)]
        #vis struct #fin_state;
        #fin_state_impls

        #resizing_code

        #[derive(#serde_derives)]
        #vis enum #transition_enum {
            #transition_variants
        }

        #from_impls

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

        #vis type #event_type = quent_model::FsmEvent<#transition_enum>;

        impl quent_model::HasEventType for #name {
            type Event = quent_model::FsmEvent<#transition_enum>;
        }

        #vis struct #resource_marker;

        impl quent_model::Resource for #resource_marker {
            type CapacityValue = #op_state;
            const RESOURCE_NAME: &'static str = #name_snake;
        }

        impl quent_model::Resource for #name {
            type CapacityValue = #op_state;
            const RESOURCE_NAME: &'static str = #name_snake;
        }

        impl quent_model::ModelComponent for #name {
            fn collect(builder: &mut quent_model::ModelBuilder) {
                builder.add_fsm(quent_model::FsmDef {
                    name: #name_snake.to_string(),
                    states: vec![#state_defs],
                    transitions: vec![#transition_defs],
                });
            }
        }

        #[derive(Clone)]
        #vis struct #observer_name<E>
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

            pub fn initializing(&self, id: uuid::Uuid, instance_name: &str, parent_group_id: uuid::Uuid, #(#user_init_param_defs,)*) -> #handle_name<E> {
                let state = #init_state {
                    instance_name: instance_name.to_string(),
                    parent_group_id,
                    resource_type_name: #name_snake.to_string(),
                    #(#user_init_field_names,)*
                };
                let mut handle = #handle_name { id, seq: 0, exited: false, tx: self.tx.clone() };
                handle.emit_transition(#transition_enum::from(state));
                handle
            }
        }

        #vis struct #handle_name<E>
        where
            E: From<#event_type> #serde_bound + Send + 'static,
        {
            id: uuid::Uuid,
            seq: u64,
            exited: bool,
            tx: quent_model::EventSender<E>,
        }

        impl<E> #handle_name<E>
        where
            E: From<#event_type> #serde_bound + Send + 'static,
        {
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
            E: From<#event_type> #serde_bound + Send + 'static,
        {
            fn drop(&mut self) { self.exit(); }
        }
    };

    Ok(output)
}
