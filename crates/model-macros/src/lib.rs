// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Proc macros for defining Quent application models.
//!
//! All model annotations use a single `#[quent_model(...)]` attribute with
//! composable flags:
//!
//! ```ignore
//! #[quent_model(instant, resource_group)]
//! pub struct QueryGroup;
//!
//! #[quent_model(instant, resource_group(root))]
//! pub struct Engine;
//!
//! #[quent_model(state)]
//! pub struct Planning;
//!
//! #[quent_model(fsm(entry -> Init, Init -> exit))]
//! pub struct MyFsm;
//! ```

use proc_macro::TokenStream;

mod entity;
mod event;
mod fsm;
mod resource;
mod resource_group;
mod state;
mod unified;
mod util;

/// Unified model annotation.
///
/// Accepts composable flags:
/// - `state` — declares an FSM state
/// - `instant` — declares a point-in-time entity
/// - `resource_group` — marks as a resource group (composable with instant/fsm)
/// - `resource_group(root)` — marks as the root resource group
/// - `fsm(...)` — declares an FSM with transition table
/// - `event(entity = T)` — declares an event for an entity
///
/// Field-level annotations (`#[usage]`, `#[deferred]`, `#[capacity]`,
/// `#[instance_name]`) are used within state structs.
#[proc_macro_attribute]
pub fn quent_model(attr: TokenStream, item: TokenStream) -> TokenStream {
    unified::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
