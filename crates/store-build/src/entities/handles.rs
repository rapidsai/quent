// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of borrowing event handles.

use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quent_instrumentation_build::{
    Options, generated_field_type, generated_module_ident, generated_type_ident,
};
use quent_schema::{Entity, Event, Identifier};
use quote::quote;

use crate::GenerateError;

/// Handle declarations and the enum variant they borrow.
pub(super) struct EventHandleCode {
    /// The handle type and its timestamp and field accessors.
    ///
    /// For example, `pub struct CreatedHandle<'a> { event: &'a Event<TaskEvent> }`.
    pub(super) handle_type_def: TokenStream,
    /// A variant pattern that leaves the original event intact.
    ///
    /// For example, `TaskEvent::Created { .. }`.
    pub(super) variant_pattern: TokenStream,
}

pub(super) fn ident(event: &Event) -> syn::Ident {
    let variant = generated_type_ident(event.name());
    quote::format_ident!("{}Handle", variant.to_string().trim_start_matches("r#"))
}

pub(super) fn generate(
    entity: &Entity,
    event: &Event,
    namespace: &[Identifier],
    opts: &Options,
    original: &TokenStream,
) -> Result<EventHandleCode, GenerateError> {
    let variant = generated_type_ident(event.name());
    let handle = ident(event);
    let variant_pattern = if event.fields().count() == 0 {
        quote! { #original::#variant }
    } else {
        quote! { #original::#variant { .. } }
    };
    let mut names = BTreeMap::from([("timestamp".to_owned(), "event timestamp".to_owned())]);
    let mut methods = Vec::new();
    for field in event.fields() {
        let field_ident = generated_module_ident(field.name());
        let method = if field_ident == "timestamp" {
            quote::format_ident!("field_timestamp")
        } else {
            field_ident.clone()
        };
        if let Some(first) = names.insert(method.to_string(), field.name().to_string()) {
            return Err(GenerateError::EntityEventsNameConflict {
                name: method.to_string(),
                first,
                second: field.name().to_string(),
            });
        }
        let ty = generated_field_type(field, namespace, opts)?;
        let docs = format!("Borrows the `{}` field.", field.name());
        let mismatch = (entity.events().count() > 1).then(|| {
            quote! {
                _ => unreachable!("event handle must match its event type"),
            }
        });
        methods.push(quote! {
            #[doc = #docs]
            pub fn #method(&self) -> &'a #ty {
                match &self.event.data {
                    #original::#variant { #field_ident, .. } => #field_ident,
                    #mismatch
                }
            }
        });
    }
    let debug = opts.debug.then(|| quote! { #[derive(Debug)] });
    let docs = format!(
        "Borrows the `{}` event of `{}`.",
        event.name(),
        entity.path()
    );
    let handle_type_def = quote! {
        #[doc = #docs]
        #[derive(Clone, Copy)]
        #debug
        pub struct #handle<'a> {
            event: &'a ::quent_events::Event<#original>,
        }

        impl<'a> #handle<'a> {
            /// Returns the recorded event timestamp.
            pub fn timestamp(&self) -> ::quent_store::TimeUnixNanoSec {
                self.event.timestamp
            }
            #(#methods)*
        }

        /// # Errors
        ///
        /// Returns the borrowed event when its variant does not match.
        impl<'a> ::core::convert::TryFrom<&'a ::quent_events::Event<#original>> for #handle<'a> {
            type Error = &'a ::quent_events::Event<#original>;

            fn try_from(event: &'a ::quent_events::Event<#original>) -> ::core::result::Result<Self, Self::Error> {
                if matches!(&event.data, #variant_pattern) {
                    Ok(Self { event })
                } else {
                    Err(event)
                }
            }
        }
    };
    Ok(EventHandleCode {
        handle_type_def,
        variant_pattern,
    })
}
