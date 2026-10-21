// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of schema-ordered typed tree traversal.

use proc_macro2::TokenStream;
use quent_events_build::common::{relative_type_path, type_ident};
use quent_schema::Schema;
use quote::quote;

use crate::{GenerateError, entities::ref_tree};

/// Generates typed traversal for schemas using the Reference Tree constraint.
pub(super) fn generate(schema: &Schema, events: &syn::File) -> Result<TokenStream, GenerateError> {
    if !ref_tree::uses_constraint(schema) {
        return Ok(TokenStream::new());
    }
    let model = type_ident(schema.name());
    let name = model.to_string();
    let name = name.trim_start_matches("r#");
    let tree = quote::format_ident!("{name}Tree");
    let visitor = quote::format_ident!("{name}TreeVisitor");
    for name in [&tree, &visitor] {
        if events
            .items
            .iter()
            .filter_map(crate::entities::namespaces::ident)
            .any(|existing| existing == name)
        {
            return Err(GenerateError::EntityEventsNameConflict {
                name: name.to_string(),
                first: "existing generated type or namespace".into(),
                second: "model-wide tree traversal".into(),
            });
        }
    }
    let entities = schema.entities().collect::<Vec<_>>();
    let methods = super::entity_names(schema)
        .iter()
        .map(|name| {
            let name = name.to_string();
            quote::format_ident!("visit_{}", name.trim_start_matches("r#"))
        })
        .collect::<Vec<_>>();
    let markers = entities
        .iter()
        .map(|entity| relative_type_path(entity.path(), &[], ""))
        .collect::<Vec<_>>();
    let parents = entities
        .iter()
        .map(|entity| ref_tree::parent_type(schema, entity))
        .collect::<Vec<_>>();
    let callbacks = methods.iter().zip(&markers).map(|(method, marker)| {
        let docs = format!("Visits a `{marker}`; calls [`Self::visit`] by default.");
        quote! {
            #[doc = #docs]
            fn #method(&mut self, node: ::quent_store::RefTreeNode<'_, #marker>) {
                self.visit(node);
            }
        }
    });
    let implementations =
        entities
            .iter()
            .zip(&markers)
            .zip(&methods)
            .map(|((entity, marker), method)| {
                let children = markers
                    .iter()
                    .zip(&parents)
                    .filter_map(|(child, parent)| {
                        (parent.as_ref() == Some(entity.path())).then_some(child)
                    })
                    .collect::<Vec<_>>();
                let visitor_binding = if children.is_empty() {
                    quote!(_)
                } else {
                    quote!(visitor)
                };
                quote! {
                    impl #tree for ::quent_store::RefTreeNode<'_, #marker> {
                        fn walk<V: #visitor + ?Sized>(&self, visitor: &mut V) {
                            visitor.#method(*self);
                        }

                        fn walk_children<V: #visitor + ?Sized>(&self, #visitor_binding: &mut V) {
                            #(
                                for child in self.children::<#children>() {
                                    #tree::walk(&child, visitor);
                                }
                            )*
                        }
                    }
                }
            });
    Ok(quote! {
        /// Receives typed callbacks during parent-first tree traversal.
        pub trait #visitor {
            /// Visits an entity, traversing its descendants by default.
            ///
            /// Overrides can call `node.walk_children(self)` to descend, or omit
            /// the call to skip descendants.
            fn visit<'a, E: ::quent_store::RefTreeEntityMarker>(
                &mut self,
                node: ::quent_store::RefTreeNode<'a, E>,
            )
            where
                ::quent_store::RefTreeNode<'a, E>: #tree,
            {
                node.walk_children(self);
            }

            #(#callbacks)*
        }

        /// Traverses child types in schema declaration order, parent first.
        pub trait #tree {
            /// Visits this subtree; instance order within each child type is unspecified.
            fn walk<V: #visitor + ?Sized>(&self, visitor: &mut V);

            /// Visits direct children and their descendants in schema order.
            fn walk_children<V: #visitor + ?Sized>(&self, visitor: &mut V);
        }

        #(#implementations)*
    })
}
