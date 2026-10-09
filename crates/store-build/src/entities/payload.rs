// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of event-specific payload types and conversions.

use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quent_instrumentation_build::{
    Options, generated_field_type, generated_module_ident, generated_type_ident,
};
use quent_schema::{Entity, Event, Identifier};
use quote::quote;

use crate::GenerateError;

/// Rust fragments declaring an event-specific payload and converting an enum
/// variant into it.
pub(super) struct EventPayloadCode {
    /// The payload struct definition, including derives and documentation.
    ///
    /// For example, `pub struct InitPayload { pub name: String }`.
    pub(super) payload_struct_def: TokenStream,
    /// An enum variant pattern binding its payload fields by value.
    ///
    /// For example, `TaskEvent::Init { name: field_0 }`.
    pub(super) variant_pattern: TokenStream,
    /// An expression constructing a typed event from the bound fields and
    /// recorded metadata.
    ///
    /// For example,
    /// `Event::new(id, ts, InitPayload { name: field_0 })`.
    pub(super) event_constructor_expr: TokenStream,
}

pub(super) fn ident(event: &Event) -> syn::Ident {
    let variant = generated_type_ident(event.name());
    quote::format_ident!("{}Payload", variant.to_string().trim_start_matches("r#"))
}

pub(super) fn generate(
    entity: &Entity,
    event: &Event,
    namespace: &[Identifier],
    opts: &Options,
    original: &TokenStream,
) -> Result<EventPayloadCode, GenerateError> {
    let variant = generated_type_ident(event.name());
    let payload = ident(event);
    let mut names = BTreeMap::new();
    let mut fields = Vec::new();
    let mut field_bindings = Vec::new();
    for (index, field) in event.fields().enumerate() {
        let ident = generated_module_ident(field.name());
        if let Some(first) = names.insert(ident.to_string(), field.name().to_string()) {
            return Err(GenerateError::EntityEventsNameConflict {
                name: ident.to_string(),
                first,
                second: field.name().to_string(),
            });
        }
        let ty = generated_field_type(field, namespace, opts)?;
        fields.push(quote! { pub #ident: #ty });
        let binding = quote::format_ident!("field_{}", index);
        field_bindings.push(quote! { #ident: #binding });
    }
    let debug = opts.debug.then(|| quote! { #[derive(Debug)] });
    let docs = format!(
        "Payload of the `{}` event for `{}`.",
        event.name(),
        entity.path()
    );
    let payload_struct_def = quote! {
        #[doc = #docs]
        #debug
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        pub struct #payload { #(#fields,)* }
    };
    let variant_pattern = if field_bindings.is_empty() {
        quote! { #original::#variant }
    } else {
        quote! { #original::#variant { #(#field_bindings,)* } }
    };
    let event_constructor_expr = quote! { ::quent_events::Event::new(event_id, event_timestamp, #payload { #(#field_bindings,)* }) };
    Ok(EventPayloadCode {
        payload_struct_def,
        variant_pattern,
        event_constructor_expr,
    })
}
