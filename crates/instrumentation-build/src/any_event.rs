// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of `AnyEvent`: a decoder from a type-erased event to the concrete
//! `Event<T>` for whichever entity produced it.

use convert_case::Case;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::common::{
    derive_attr, module_ident, path_name_pascal, raw_ident, relative_type_path, to_case,
};
use crate::namespace::Namespace;
use crate::{GenerateError, Options};

/// Generate `AnyEvent` and its `from_any` decoder.
///
/// # Errors
///
/// Returns [`GenerateError`] if a derive entry is not a parseable Rust path.
pub(crate) fn generate_any_event(
    namespace: &Namespace<'_>,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let variants: Vec<(Ident, TokenStream)> = namespace
        .entities()
        .iter()
        .map(|entity| {
            let variant = raw_ident(path_name_pascal(entity.path()));
            let event = relative_type_path(entity.path(), namespace.path(), "Event");
            (variant, event)
        })
        .collect();
    let children: Vec<(Ident, Ident)> = namespace
        .children_with_entities()
        .map(|child| {
            let segment = child
                .path()
                .last()
                .expect("child namespaces extend their parent");
            (
                raw_ident(to_case(segment, Case::Pascal)),
                module_ident(segment),
            )
        })
        .collect();
    if variants.is_empty() && children.is_empty() {
        return Ok(quote! {});
    }

    let derives = derive_attr(opts.event_derives, opts.debug, opts.serde, false)?;
    let runtime = opts.event_runtime();
    let decls = variants.iter().map(|(variant, event)| {
        quote! { #variant(&'a #runtime::Event<#event>) }
    });
    let child_decls = children.iter().map(|(variant, module)| {
        quote! { #variant(#module::AnyEvent<'a>) }
    });
    let direct_arms = variants.iter().map(|(variant, event)| {
        quote! {
            if let Some(event) = any.downcast_ref::<#runtime::Event<#event>>() {
                return Some(Self::#variant(event));
            }
        }
    });
    let child_arms = children.iter().map(|(variant, module)| {
        quote! {
            if let Some(event) = #module::AnyEvent::from_any(any) {
                return Some(Self::#variant(event));
            }
        }
    });

    Ok(quote! {
        #derives
        pub enum AnyEvent<'a> {
            #(#decls,)*
            #(#child_decls,)*
        }
        impl<'a> AnyEvent<'a> {
            pub fn from_any(any: &'a dyn ::core::any::Any) -> Option<Self> {
                #(#direct_arms)*
                #(#child_arms)*
                None
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::pretty;
    use quent_schema::builder::SchemaBuilder;
    use quent_schema::test_utils::{entity, event, ident};

    #[test]
    fn emits_a_variant_and_arm_per_entity() {
        let schema = SchemaBuilder::new(ident("Demo"))
            .with_entity(entity("Query", [event("submitted", [])]))
            .with_entity(entity("Server", [event("booted", [])]))
            .build()
            .unwrap();
        let opts = Options::default();
        let namespaces = Namespace::root(&schema);
        let expected = quote! {
            #[derive(Debug)]
            pub enum AnyEvent<'a> {
                Query(&'a ::quent_instrumentation::Event<QueryEvent>),
                Server(&'a ::quent_instrumentation::Event<ServerEvent>)
            }

            impl<'a> AnyEvent<'a> {
                pub fn from_any(any: &'a dyn ::core::any::Any) -> Option<Self> {
                    if let Some(event) =
                        any.downcast_ref::<::quent_instrumentation::Event<QueryEvent>>()
                    {
                        return Some(Self::Query(event));
                    }
                    if let Some(event) =
                        any.downcast_ref::<::quent_instrumentation::Event<ServerEvent>>()
                    {
                        return Some(Self::Server(event));
                    }
                    None
                }
            }
        };
        assert_eq!(
            pretty(generate_any_event(&namespaces, &opts).unwrap()),
            pretty(expected)
        );
    }

    #[test]
    fn emits_nothing_without_entities() {
        let schema = SchemaBuilder::new(ident("Demo")).build().unwrap();
        let namespaces = Namespace::root(&schema);

        assert!(
            generate_any_event(&namespaces, &Options::default())
                .unwrap()
                .is_empty()
        );
    }
}
