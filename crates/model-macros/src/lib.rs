// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Proc macros for defining Quent application models.
//!
//! Four derive macros replace the former `#[quent_model(...)]` attribute:
//!
//! ```ignore
//! #[derive(State)]
//! pub struct Planning;
//!
//! #[derive(Fsm)]
//! pub struct MyFsm {
//!     #[entry] #[to(exit)]
//!     init: Init,
//! }
//!
//! #[derive(Entity)]
//! pub struct Engine {
//!     pub init: Init,
//! }
//!
//! #[derive(Entity)]
//! #[resource_group(root)]
//! pub struct Engine {
//!     pub init: Init,
//! }
//! ```

use proc_macro::TokenStream;

mod define_model;
mod entity;
mod event;
mod fsm;
mod resource_derive;
mod state;
mod util;

/// Derive macro for FSM state structs.
///
/// Field-level attributes:
/// - `#[instance_name]` — marks the field carrying the instance name
///
/// Capacity fields are detected by type (`Capacity<V, K>`), not by annotation.
#[proc_macro_derive(State, attributes(deferred, instance_name, parent_group))]
pub fn derive_state(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    state::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for entity event structs.
///
/// Generates `EventMetadata` impl so that `#[derive(Entity)]` can populate
/// `EntityEventDef.attributes` with the event's field names and types.
#[proc_macro_derive(Event)]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    event::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for FSM definitions.
///
/// The struct must have named fields where each field's type is a state type.
/// Field-level attributes:
/// - `#[entry]` — marks this state as an entry point
/// - `#[to(StateA, StateB, exit)]` — declares transitions from this state
///
/// Struct-level attributes (optional):
/// - `#[resource_group]` / `#[resource_group(root)]` — resource group metadata
#[proc_macro_derive(Fsm, attributes(entry, to, resource_group))]
pub fn derive_fsm(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    fsm::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for entity definitions.
///
/// All named fields are event types (must derive `Event`).
/// Unit structs produce entities with no events.
///
/// Struct-level attributes (optional):
/// - `#[resource_group]` / `#[resource_group(root)]` — resource group metadata
#[proc_macro_derive(Entity, attributes(resource_group))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    entity::expand_derive(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for fixed-bounds resource definitions.
///
/// Generates the full resource FSM (Initializing → Operating → Finalizing → exit),
/// state structs, handle, event types, and Resource trait impl.
///
/// Fields with `Capacity<V, K>` type go on the generated Operating state.
/// Other fields go on the generated Initializing state alongside standard
/// metadata fields (instance_name, parent_group_id, resource_type_name).
/// Unit structs produce a unit resource with no capacity.
#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    resource_derive::expand_resource(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for resizable resource definitions.
///
/// Same as `Resource` but adds a Resizing state and the operating ↔ resizing cycle.
#[proc_macro_derive(ResizableResource)]
pub fn derive_resizable_resource(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    resource_derive::expand_resizable_resource(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Generates a model type alias and event enum from a list of components.
///
/// ```ignore
/// define_model! {
///     Simulator {
///         quent_query_engine_model::Engine,
///         task::Task,
///         quent_stdlib::Memory,
///     }
/// }
/// ```
///
/// Generates `SimulatorModel` (type alias) and `SimulatorEvent` (event enum).
/// Variant names are derived from the last path segment.
#[proc_macro]
pub fn define_model(input: TokenStream) -> TokenStream {
    define_model::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
