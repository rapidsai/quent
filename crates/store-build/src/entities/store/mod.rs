// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of entity stores across supported backends.

use proc_macro2::TokenStream;
use quent_schema::{Entity, Event};

use crate::entities::EventCode;

mod native;

/// Store fragments for each supported backend of one schema event.
pub(super) struct EventStoreCode {
    /// Fragments for the native in-memory backend.
    native: native::NativeEventStoreCode,
}

/// Generates code to index and access one event type in the supported stores.
pub(super) fn generate_event(
    entity: &Entity,
    event: &Event,
    handle: &TokenStream,
    method: &syn::Ident,
    variant_pattern: &TokenStream,
    accessor_method_signature: &TokenStream,
) -> EventStoreCode {
    EventStoreCode {
        native: native::generate_event(
            entity,
            event,
            handle,
            method,
            variant_pattern,
            accessor_method_signature,
        ),
    }
}

/// Generates entity indices and event access implementations.
pub(super) fn generate_entity(
    entity: &Entity,
    event_index_ident: &syn::Ident,
    access: &syn::Ident,
    marker: &TokenStream,
    events: &[EventCode],
    fsm: Option<&super::fsm::FsmCode>,
) -> TokenStream {
    native::generate_entity(entity, event_index_ident, access, marker, events, fsm)
}

/// Store fragments for an FSM entity.
pub(super) struct FsmStoreCode {
    native: native::NativeFsmStoreCode,
}

pub(super) fn generate_fsm(
    entity: &Entity,
    marker: &TokenStream,
    original: &TokenStream,
    transition: &syn::Ident,
) -> FsmStoreCode {
    FsmStoreCode {
        native: native::generate_fsm(entity, marker, original, transition),
    }
}
