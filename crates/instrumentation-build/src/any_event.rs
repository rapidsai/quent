// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of `AnyEvent`: a decoder from a type-erased event to the concrete
//! `Event<T>` for whichever entity produced it.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::Schema;
use quote::quote;
use syn::Ident;

use crate::common::{derive_attr, raw_ident, to_case};
use crate::{GenerateError, Options};

/// Generate `AnyEvent` and its `from_any` decoder, carrying the event enums'
/// derives ([`Options::event_derives`]).
///
/// # Errors
///
/// Returns [`GenerateError`] if a derive entry is not a parseable Rust path.
pub(crate) fn generate_any_event(
    schema: &Schema,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let variants: Vec<(Ident, Ident)> = schema
        .entities()
        .map(|entity| {
            let pascal = to_case(entity.path().name(), Case::Pascal);
            (
                raw_ident(pascal.clone()),
                raw_ident(format!("{pascal}Event")),
            )
        })
        .collect();
    if variants.is_empty() {
        return Ok(quote! {});
    }

    let derives = derive_attr(opts.event_derives)?;
    let decls = variants.iter().map(|(variant, event)| {
        quote! { #variant(&'a ::quent_instrumentation::Event<#event>) }
    });
    let arms = variants.iter().map(|(variant, event)| {
        quote! {
            if let Some(event) = any.downcast_ref::<::quent_instrumentation::Event<#event>>() {
                return Some(AnyEvent::#variant(event));
            }
        }
    });

    Ok(quote! {
        #derives
        pub enum AnyEvent<'a> {
            #(#decls),*
        }
        impl<'a> AnyEvent<'a> {
            pub fn from_any(any: &'a (dyn ::core::any::Any)) -> Option<AnyEvent<'a>> {
                #(#arms)*
                None
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::pretty;
    use quent_schema::builder::{EntityBuilder, EventBuilder, SchemaBuilder};
    use quent_schema::{Cardinality, test_utils::ident};

    fn entity(name: &str, event: &str) -> quent_schema::Entity {
        EntityBuilder::new(ident(name))
            .with_event(
                EventBuilder::new(ident(event), Cardinality::Once)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    }

    #[test]
    fn emits_a_variant_and_arm_per_entity() {
        let schema = SchemaBuilder::new(ident("Demo"))
            .with_entity(entity("Query", "submitted"))
            .with_entity(entity("Server", "booted"))
            .build()
            .unwrap();
        let opts = Options {
            event_derives: &["Debug"],
            ..Options::default()
        };
        let expected = quote! {
            #[derive(Debug)]
            pub enum AnyEvent<'a> {
                Query(&'a ::quent_instrumentation::Event<QueryEvent>),
                Server(&'a ::quent_instrumentation::Event<ServerEvent>)
            }

            impl<'a> AnyEvent<'a> {
                pub fn from_any(any: &'a (dyn ::core::any::Any)) -> Option<AnyEvent<'a>> {
                    if let Some(event) =
                        any.downcast_ref::<::quent_instrumentation::Event<QueryEvent>>()
                    {
                        return Some(AnyEvent::Query(event));
                    }
                    if let Some(event) =
                        any.downcast_ref::<::quent_instrumentation::Event<ServerEvent>>()
                    {
                        return Some(AnyEvent::Server(event));
                    }
                    None
                }
            }
        };
        assert_eq!(
            pretty(generate_any_event(&schema, &opts).unwrap()),
            pretty(expected)
        );
    }

    #[test]
    fn emits_nothing_without_entities() {
        let schema = SchemaBuilder::new(ident("Demo")).build().unwrap();

        assert!(
            generate_any_event(&schema, &Options::default())
                .unwrap()
                .is_empty()
        );
    }
}
