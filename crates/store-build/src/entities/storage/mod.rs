// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of event storage across supported storage backends.

use proc_macro2::TokenStream;
use quent_schema::{Entity, Event};

use super::EventCode;

mod native;

/// Storage fragments for each supported backend of one schema event.
pub(super) struct EventStorageCode {
    /// Fragments for the native in-memory backend.
    native: native::NativeEventStorageCode,
}

pub(super) fn generate_event(
    entity: &Entity,
    event: &Event,
    payload: &syn::Ident,
    method: &syn::Ident,
    variant_pattern: &TokenStream,
    event_constructor_expr: &TokenStream,
    accessor_method_signature: &TokenStream,
) -> EventStorageCode {
    EventStorageCode {
        native: native::generate_event(
            entity,
            event,
            payload,
            method,
            variant_pattern,
            event_constructor_expr,
            accessor_method_signature,
        ),
    }
}

pub(super) fn generate_entity(
    entity: &Entity,
    native: &syn::Ident,
    event_storage_ident: &syn::Ident,
    access: &syn::Ident,
    marker: &TokenStream,
    events: &[EventCode],
) -> TokenStream {
    native::generate_entity(entity, native, event_storage_ident, access, marker, events)
}
