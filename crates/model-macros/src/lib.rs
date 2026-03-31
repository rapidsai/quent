// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Proc macros for defining Quent application models.

use proc_macro::TokenStream;

mod entity;
mod event;
mod fsm;
mod resource;
mod resource_group;
mod state;
mod util;

/// Declares an FSM with a transition table.
///
/// The transitions are listed inside the attribute:
///
/// ```ignore
/// #[quent_model::fsm(
///     entry -> Queueing,
///     Queueing -> Computing,
///     Computing -> exit,
/// )]
/// pub struct Task;
/// ```
///
/// The macro validates:
/// - All states referenced in transitions exist as types implementing `State`
/// - Every state is reachable from entry
/// - Every state can reach exit
/// - No transitions leave exit
///
/// Generates:
/// - A transition enum (`TaskTransition`)
/// - A deferred enum (`TaskDeferred`)
/// - An event type alias (`TaskEvent = FsmEvent<TaskTransition, TaskDeferred>`)
/// - `ModelComponent` impl for metadata collection
/// - `From` impls for state types into the transition enum
#[proc_macro_attribute]
pub fn fsm(attr: TokenStream, item: TokenStream) -> TokenStream {
    fsm::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Declares a state with transition attributes.
///
/// Fields may be annotated with:
/// - `#[quent_model::usage]`: marks a `Usage<T>` field as a resource usage
/// - `#[quent_model::deferred]`: marks an `Option<T>` field as settable after
///   transition
///
/// ```ignore
/// #[quent_model::state]
/// pub struct Computing {
///     #[quent_model::usage]
///     pub thread: Usage<Thread>,
///     #[quent_model::deferred]
///     pub rows_processed: Option<u64>,
/// }
/// ```
#[proc_macro_attribute]
pub fn state(_attr: TokenStream, item: TokenStream) -> TokenStream {
    state::expand(item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Marks an FSM as a resource, enabling `Usage<T>` references.
///
/// Requires a `capacity` parameter naming the state type whose fields
/// define the resource's capacity:
///
/// ```ignore
/// #[quent_model::fsm(...)]
/// #[quent_model::resource(capacity = Operating)]
/// pub struct Memory;
/// ```
#[proc_macro_attribute]
pub fn resource(attr: TokenStream, item: TokenStream) -> TokenStream {
    resource::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Declares a plain entity (not an FSM, not a resource).
///
/// ```ignore
/// #[quent_model::entity]
/// pub struct Operator {
///     pub plan_id: Ref<Plan>,
///     pub type_name: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn entity(_attr: TokenStream, item: TokenStream) -> TokenStream {
    entity::expand(item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Declares an additional one-shot event for an entity.
///
/// ```ignore
/// #[quent_model::event(entity = Operator)]
/// pub struct OperatorStatistics {
///     pub rows_processed: u64,
/// }
/// ```
#[proc_macro_attribute]
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
    event::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Marks an entity as a resource group.
///
/// ```ignore
/// #[quent_model::resource_group]
/// pub struct Engine { pub name: String }
///
/// #[quent_model::resource_group(parent = Engine)]
/// pub struct Query { pub query_group_id: Ref<QueryGroup> }
/// ```
#[proc_macro_attribute]
pub fn resource_group(attr: TokenStream, item: TokenStream) -> TokenStream {
    resource_group::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
