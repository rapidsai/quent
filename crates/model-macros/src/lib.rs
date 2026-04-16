// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Proc macros for defining Quent application models.
//!
//! Function-like macros for model definitions:
//!
//! - `resource!` — resource with optional capacity and init attributes
//! - `entity!` — single-event, multi-event, or resource group entity
//! - `state!` — FSM state with attributes and resource usages
//! - `fsm!` — FSM with states, transitions, entry and exit points
//! - `model!` — model composition and event enum generation
//! - `instrumentation!` — context struct with observer factory methods
//!
//! Derive macro:
//!
//! - `#[derive(Attributes)]` — metadata for event payload and attribute structs

use proc_macro::TokenStream;

mod entity_macro;
mod event;
mod fsm_macro;
mod model_macro;
mod resource_derive;
mod resource_macro;
mod state_macro;
mod util;

/// Derive macro for struct types used as event payloads or nested attributes.
///
/// Generates `EventMetadata` impl so that the struct's field names and types
/// are available to the model and codegen.
#[proc_macro_derive(Attributes)]
pub fn derive_attributes(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    event::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for fixed-bounds resource definitions.
///
/// Used internally by `resource!` — prefer that macro for new code.
#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    resource_derive::expand_resource(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for resizable resource definitions.
///
/// Used internally by `resource!` — prefer that macro for new code.
#[proc_macro_derive(ResizableResource)]
pub fn derive_resizable_resource(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    resource_derive::expand_resizable_resource(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Composes model components into a model type and event enum.
///
/// ```ignore
/// model! {
///     App {
///         root: Cluster,
///         Worker,
///         Thread,
///         Task,
///     }
/// }
/// ```
#[proc_macro]
pub fn model(input: TokenStream) -> TokenStream {
    model_macro::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Generates the instrumentation context with observer factory methods.
///
/// ```ignore
/// instrumentation!(App);
/// ```
///
/// This generates `AppContext`, the entry point for instrumenting your
/// application. To start emitting events:
///
/// 1. Create a context: `let ctx = AppContext::try_new(Uuid::now_v7(), Some(exporter_options))?;`
/// 2. Get an observer: `let obs = ctx.cluster_observer();`
/// 3. Emit events: `obs.cluster(id, "my-cluster");`
///
/// For FSMs: the observer's entry method returns a handle for state transitions.
/// For resources: the observer's `initializing()` method returns a handle for
/// lifecycle transitions (`operating()`, `finalizing()`, `exit()`).
#[proc_macro]
pub fn instrumentation(input: TokenStream) -> TokenStream {
    model_macro::expand_instrumentation(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Defines an FSM with states, transitions, entry and exit points.
///
/// ```ignore
/// fsm! {
///     Task {
///         states: { queued: Queued, computing: Computing },
///         entry: queued,
///         exit_from: { computing },
///         transitions: { queued => computing },
///     }
/// }
/// ```
#[proc_macro]
pub fn fsm(input: TokenStream) -> TokenStream {
    fsm_macro::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Defines a state with optional attributes and resource usages.
///
/// ```ignore
/// state! {
///     Queued {
///         attributes: { priority: u32 },
///         usages: { queue: Queue },
///     }
/// }
/// ```
#[proc_macro]
pub fn state(input: TokenStream) -> TokenStream {
    state_macro::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Defines a resource with optional capacity and init attributes.
///
/// ```ignore
/// resource! { Thread }
///
/// resource! {
///     Memory {
///         capacity: { bytes: Option<u64> },
///     }
/// }
/// ```
#[proc_macro]
pub fn resource(input: TokenStream) -> TokenStream {
    resource_macro::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Defines an entity (single-event, multi-event, or resource group).
///
/// ```ignore
/// entity! {
///     Info {
///         attributes: { message: String },
///     }
/// }
///
/// entity! {
///     FileStats {
///         events: { checksum: Checksum, decompressed: Decompressed },
///     }
/// }
///
/// entity! {
///     Cluster: ResourceGroup<Root = true> {}
/// }
/// ```
#[proc_macro]
pub fn entity(input: TokenStream) -> TokenStream {
    entity_macro::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
