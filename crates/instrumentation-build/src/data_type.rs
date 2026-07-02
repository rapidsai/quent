// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Mapping from schema [`DataType`]s to Rust type tokens.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::DataType;
use quote::quote;

use crate::common::{raw_ident, to_case};

/// Maximum nesting depth of `Option`/`List`/`EntityRef` wrappers a single field
/// type may have, far above any realistic schema. Self-referential records are
/// already ruled out by base validation, but even if somehow schemas are
/// produced with great nesting depth, this will produce a friendlier panic
/// instead of a stack overflow.
pub(crate) const MAX_TYPE_DEPTH: usize = 64;

/// Map a [`DataType`] to its Rust type tokens.
///
/// # Panics
///
/// Panics if `ty` nests deeper than [`MAX_TYPE_DEPTH`].
pub(crate) fn map_data_type(ty: &DataType, depth: usize) -> TokenStream {
    assert!(
        depth <= MAX_TYPE_DEPTH,
        "field type nesting exceeds the maximum depth of {MAX_TYPE_DEPTH}"
    );
    match ty {
        DataType::Bool => quote! { bool },
        DataType::Uuid => quote! { ::uuid::Uuid },
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
            let inner = map_data_type(inner, depth + 1);
            quote! { Option<#inner> }
        }
        DataType::List(inner) => {
            let inner = map_data_type(inner, depth + 1);
            quote! { Vec<#inner> }
        }
        DataType::Record(name) => {
            let ident = raw_ident(to_case(name, Case::Pascal));
            quote! { #ident }
        }
        DataType::DynamicRecord => quote! { ::quent_attributes::CustomAttributes },
        DataType::EntityRef { data, .. } => match data {
            Some(inner) => {
                let inner = map_data_type(inner, depth + 1);
                quote! { ::quent_instrumentation_runtime::EntityRef<#inner> }
            }
            None => quote! { ::quent_instrumentation_runtime::EntityRef },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quent_schema::DataType;

    #[test]
    #[should_panic(expected = "maximum depth")]
    fn excessive_type_nesting_panics() {
        let mut ty = DataType::U8;
        for _ in 0..(MAX_TYPE_DEPTH + 5) {
            ty = DataType::Option(Box::new(ty));
        }
        let _ = map_data_type(&ty, 0);
    }
}
