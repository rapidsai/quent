// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of Reference Tree parent access.

use proc_macro2::TokenStream;
use quent_events_build::common::{module_ident, relative_type_path, type_ident};
use quent_schema::{
    DataType, Entity, Path, Schema,
    visitor::{Cursor, Element, Visitor},
};
use quote::quote;

pub(crate) fn uses_constraint(schema: &Schema) -> bool {
    struct UsesRefTree(bool);
    impl Visitor for UsesRefTree {
        type Output = bool;

        fn visit(&mut self, cursor: &Cursor) {
            if let Element::Annotations(annotations) = cursor.current() {
                self.0 |= annotations.has_constraint("quent.ref-tree.v0.1.0");
            }
        }

        fn finish(self) -> bool {
            self.0
        }
    }
    schema.walk(UsesRefTree(false))
}

pub(super) fn generate(
    schema: &Schema,
    entity: &Entity,
    marker: &TokenStream,
    original: &TokenStream,
) -> TokenStream {
    let parent_target = parent_type(schema, entity);
    let is_child = parent_target.is_some();
    let parent = match parent_target {
        Some(target) => {
            let target = relative_type_path(&target, &[], "");
            let parents = vec![quote!(super); entity.path().namespace().len() + 1];
            quote!(#(#parents::)* #target)
        }
        None => quote!(Self),
    };
    let child_impl = is_child.then(|| {
        quote! {
            impl ::quent_store::RefTreeChildOf<#parent> for #marker {}
        }
    });
    let arms = entity.events().map(|event| {
        let variant = type_ident(event.name());
        let mut bindings = Vec::new();
        let mut parents = Vec::new();
        for field in event.fields() {
            let name = module_ident(field.name());
            if let Some(parent) = parent_expr(schema, field.ty(), quote!(#name)) {
                bindings.push(name);
                parents.push(parent);
            }
        }
        let pattern = if event.fields().count() == 0 {
            quote!(#original::#variant)
        } else {
            quote!(#original::#variant { #(#bindings,)* .. })
        };
        let parent = first_parent(parents).unwrap_or_else(|| quote!(None));
        quote!(#pattern => #parent)
    });
    quote! {
        #child_impl

        impl ::quent_store::RefTreeEntityMarker for #marker {
            type Parent = #parent;

            fn parent_id(payload: &Self::Payload) -> Option<::quent_store::Id<Self::Parent>> {
                match payload { #(#arms,)* }
            }
        }
    }
}

fn parent_expr(schema: &Schema, ty: &DataType, value: TokenStream) -> Option<TokenStream> {
    match ty {
        DataType::EntityRef { annotations, .. }
            if annotations.has_constraint("quent.ref-tree.v0.1.0") =>
        {
            Some(quote!(Some(::quent_store::Id::from(#value))))
        }
        DataType::Record(path) => {
            let parents = schema
                .record(path)?
                .fields()
                .filter_map(|field| {
                    let name = module_ident(field.name());
                    parent_expr(
                        schema,
                        field.ty(),
                        borrow_parent_ref(field.ty(), quote!((#value).#name)),
                    )
                })
                .collect::<Vec<_>>();
            first_parent(parents)
        }
        DataType::Option(inner) => {
            if let Some(mapper) = required_parent_mapper(schema, inner) {
                Some(quote!((#value).as_ref().map(#mapper)))
            } else {
                let parent = parent_expr(schema, inner, quote!(value))?;
                Some(quote!((#value).as_ref().and_then(|value| #parent)))
            }
        }
        DataType::List(inner) => {
            if let Some(mapper) = required_parent_mapper(schema, inner) {
                Some(quote!((#value).first().map(#mapper)))
            } else {
                let parent = parent_expr(schema, inner, quote!(value))?;
                Some(quote!((#value).iter().find_map(|value| #parent)))
            }
        }
        DataType::EntityRef {
            data: Some(inner), ..
        } => parent_expr(
            schema,
            inner,
            borrow_parent_ref(inner, quote!((#value).data)),
        ),
        _ => None,
    }
}

fn first_parent(parents: Vec<TokenStream>) -> Option<TokenStream> {
    parents
        .into_iter()
        .reduce(|first, next| quote!((#first).or(#next)))
}

fn borrow_parent_ref(ty: &DataType, value: TokenStream) -> TokenStream {
    match ty {
        DataType::EntityRef { annotations, .. }
            if annotations.has_constraint("quent.ref-tree.v0.1.0") =>
        {
            quote!(&#value)
        }
        _ => value,
    }
}

fn required_parent_mapper(schema: &Schema, ty: &DataType) -> Option<TokenStream> {
    match ty {
        DataType::EntityRef { annotations, .. }
            if annotations.has_constraint("quent.ref-tree.v0.1.0") =>
        {
            Some(quote!(::quent_store::Id::from))
        }
        _ => {
            let parent = required_parent_expr(schema, ty, quote!(value))?;
            Some(quote!(|value| #parent))
        }
    }
}

fn required_parent_expr(schema: &Schema, ty: &DataType, value: TokenStream) -> Option<TokenStream> {
    match ty {
        DataType::EntityRef { annotations, .. }
            if annotations.has_constraint("quent.ref-tree.v0.1.0") =>
        {
            Some(quote!(::quent_store::Id::from(#value)))
        }
        DataType::Record(path) => schema.record(path)?.fields().find_map(|field| {
            let name = module_ident(field.name());
            required_parent_expr(
                schema,
                field.ty(),
                borrow_parent_ref(field.ty(), quote!((#value).#name)),
            )
        }),
        DataType::EntityRef {
            data: Some(inner), ..
        } => required_parent_expr(
            schema,
            inner,
            borrow_parent_ref(inner, quote!((#value).data)),
        ),
        _ => None,
    }
}

/// Returns the declared parent entity path, or `None` for a root type.
pub(crate) fn parent_type(schema: &Schema, entity: &Entity) -> Option<Path> {
    entity
        .events()
        .flat_map(|event| event.fields())
        .find_map(|field| parent_target(schema, field.ty()))
}

fn parent_target(schema: &Schema, ty: &DataType) -> Option<Path> {
    match ty {
        DataType::EntityRef { annotations, .. }
            if annotations.has_constraint("quent.ref-tree.v0.1.0") =>
        {
            Some(
                annotations
                    .constraint("quent.ref-target.v0.1.0")
                    .and_then(|constraint| constraint.data())
                    .and_then(|target| target.parse().ok())
                    .expect("validated tree-forming references have a target entity"),
            )
        }
        DataType::Record(path) => schema
            .record(path)?
            .fields()
            .find_map(|field| parent_target(schema, field.ty())),
        DataType::Option(inner)
        | DataType::List(inner)
        | DataType::EntityRef {
            data: Some(inner), ..
        } => parent_target(schema, inner),
        _ => None,
    }
}
