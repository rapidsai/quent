// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Mapping from schema [`DataType`]s to Rust type tokens.

use proc_macro2::TokenStream;
use quent_ref_target::RefTarget;
use quent_schema::{Annotations, DataType};
use quote::quote;

use crate::common::{relative_root_type, relative_type_path};
use crate::{GenerateError, Options};

/// Maximum nesting depth of `Option`/`List`/`EntityRef` wrappers a single field
/// type may have, far above any realistic schema. Self-referential records are
/// already ruled out by base validation.
pub(crate) const MAX_TYPE_DEPTH: usize = 64;

/// Map a [`DataType`] to its Rust type tokens.
pub(crate) fn map_data_type(
    ty: &DataType,
    depth: usize,
    source_namespace: &[quent_schema::Identifier],
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    if depth > MAX_TYPE_DEPTH {
        return Err(GenerateError::TypeNestingTooDeep {
            max: MAX_TYPE_DEPTH,
        });
    }
    Ok(match ty {
        DataType::Bool => quote! { bool },
        DataType::Uuid => {
            let runtime = opts.event_runtime();
            quote! { #runtime::Uuid }
        }
        DataType::String => quote! { String },
        DataType::U8 => quote! { u8 },
        DataType::U16 => quote! { u16 },
        DataType::U32 => quote! { u32 },
        DataType::U64 => quote! { u64 },
        DataType::I8 => quote! { i8 },
        DataType::I16 => quote! { i16 },
        DataType::I32 => quote! { i32 },
        DataType::I64 => quote! { i64 },
        DataType::F32 => quote! { f32 },
        DataType::F64 => quote! { f64 },
        DataType::Option(inner) => {
            let inner = map_data_type(inner, depth + 1, source_namespace, opts)?;
            quote! { Option<#inner> }
        }
        DataType::List(inner) => {
            let inner = map_data_type(inner, depth + 1, source_namespace, opts)?;
            quote! { Vec<#inner> }
        }
        DataType::Record(path) => relative_type_path(path, source_namespace, ""),
        DataType::DynamicRecord => {
            let runtime = opts.event_runtime();
            quote! { #runtime::DynamicAttributes }
        }
        DataType::EntityRef { data, annotations } => {
            let target = ref_target_marker(annotations, source_namespace);
            let runtime = opts.event_runtime();
            match data {
                Some(inner) => {
                    let inner = map_data_type(inner, depth + 1, source_namespace, opts)?;
                    quote! { #runtime::EntityRef<#target, #inner> }
                }
                None => quote! { #runtime::EntityRef<#target> },
            }
        }
    })
}

/// The target-entity marker type for an entity reference, taken from its
/// ref-target constraint, or the `AnyEntity` marker when it is not restricted
/// to a target entity.
fn ref_target_marker(
    annotations: &Annotations,
    source_namespace: &[quent_schema::Identifier],
) -> TokenStream {
    match RefTarget::from_annotations(annotations) {
        Some(entity) => relative_type_path(entity.as_ref(), source_namespace, ""),
        None => relative_root_type("AnyEntity", source_namespace),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quent_constraints::Constraint;
    use quent_ref_target::RefTargetConstraint;
    use quent_schema::DataType;

    #[test]
    fn excessive_type_nesting_returns_error() {
        let mut ty = DataType::U8;
        for _ in 0..(MAX_TYPE_DEPTH + 5) {
            ty = DataType::Option(Box::new(ty));
        }
        assert!(matches!(
            map_data_type(&ty, 0, &[], &Options::default()),
            Err(GenerateError::TypeNestingTooDeep {
                max: MAX_TYPE_DEPTH
            })
        ));
    }

    #[test]
    fn entity_ref_uses_its_ref_target_marker() {
        use quent_schema::builder::AnnotationsBuilder;

        let annotations = AnnotationsBuilder::new()
            .with_constraint(RefTargetConstraint::NAME, Some("Cluster".to_string()));
        let ty = DataType::EntityRef {
            data: Some(Box::new(DataType::U64)),
            annotations: annotations.build().unwrap(),
        };
        let tokens = map_data_type(&ty, 0, &[], &Options::default())
            .unwrap()
            .to_string();
        assert!(tokens.contains("EntityRef < Cluster , u64 >"), "{tokens}");
    }
}
