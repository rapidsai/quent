// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates code for reading log messages and converting their extra fields.

use proc_macro2::TokenStream;
use quent_events_build::common::{relative_type_path, type_ident};
use quent_log::LogDefinition;
use quent_schema::Schema;
use quote::quote;

#[cfg(feature = "ui")]
use super::attribute_projection;

pub(super) fn generate(schema: &Schema) -> TokenStream {
    let mappings = schema.entities().filter_map(|entity| {
        let definition = LogDefinition::from_entity(entity)?.expect("validated log definition");
        let event_type = relative_type_path(entity.path(), &[], "Event");
        let levels: Vec<_> = definition
            .levels()
            .map(|level| {
                let name = level.name().to_string();
                let rank = level.rank();
                quote! { ::quent_log::analysis::LogLevel { name: #name, rank: #rank } }
            })
            .collect();
        let messages = definition.levels().map(|level| {
            let variant = type_ident(level.name());
            let rank = level.rank();
            let name = level.name().to_string();
            quote! {
                Self::#variant { message, .. } => Some(::quent_log::analysis::LogMessage {
                    level: ::quent_log::analysis::LogLevel { name: #name, rank: #rank },
                    message,
                })
            }
        });
        let fallback = (entity.events().count() != levels.len()).then(|| quote! { _ => None, });
        #[cfg(feature = "ui")]
        let ui = ui_impl(schema, entity, &definition, &event_type);
        #[cfg(not(feature = "ui"))]
        let ui = TokenStream::new();
        Some(quote! {
            impl ::quent_log::analysis::LogEventPayload for #event_type {
                const LEVELS: &'static [::quent_log::analysis::LogLevel] = &[#(#levels),*];
                fn log_message(&self) -> Option<::quent_log::analysis::LogMessage<'_>> {
                    match self { #(#messages,)* #fallback }
                }
            }
            #ui
        })
    });
    quote! { #(#mappings)* }
}

#[cfg(feature = "ui")]
fn ui_impl(
    schema: &Schema,
    entity: &quent_schema::Entity,
    definition: &LogDefinition,
    event_type: &TokenStream,
) -> TokenStream {
    let attributes = definition.levels().map(|level| {
        let variant = type_ident(level.name());
        let event = entity.event(level.name()).expect("validated level event");
        let projection = attribute_projection::fields(
            schema,
            event.fields().filter(|field| field.name() != "message"),
        );
        let bindings = event
            .fields()
            .filter(|field| field.name() != "message")
            .map(|field| quent_events_build::common::module_ident(field.name()));
        quote! { Self::#variant { #(#bindings,)* .. } => vec![#(#projection),*] }
    });
    let fallback =
        (entity.events().count() != definition.levels().len()).then(|| quote! { _ => Vec::new(), });
    quote! {
        impl ::quent_log::ui::LogAttributes for #event_type {
            fn log_attributes(&self) -> Vec<::quent_log::ui::DynamicAttribute> {
                match self { #(#attributes,)* #fallback }
            }
        }
    }
}
