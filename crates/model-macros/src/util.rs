// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Returns serde derive tokens when the `serde` feature is enabled, or empty
/// tokens otherwise. Uses the `quent_model::serde` re-export so callers don't
/// need `serde` as a direct dependency.
pub fn serde_derives() -> TokenStream {
    if cfg!(feature = "serde") {
        quote! { quent_model::serde::Serialize, quent_model::serde::Deserialize }
    } else {
        quote! {}
    }
}

/// Returns `#[serde(crate = "quent_model::serde")]` when the `serde` feature
/// is enabled. Tells the serde derive macros where to find the serde runtime.
pub fn serde_crate_attr() -> TokenStream {
    if cfg!(feature = "serde") {
        quote! { #[serde(crate = "quent_model::serde")] }
    } else {
        quote! {}
    }
}

/// Check if a type is `Capacity<...>`.
pub fn is_capacity_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        type_path
            .path
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "Capacity")
    } else {
        false
    }
}

/// Converts a snake_case string to PascalCase.
pub fn to_pascal_case(s: &str) -> String {
    s.to_case(Case::Pascal)
}

/// Converts a PascalCase identifier to snake_case.
pub fn to_snake_case(ident: &Ident) -> String {
    ident.to_string().to_case(Case::Snake)
}

/// If `ty` is `Option<T>`, returns `(T, true)`. Otherwise `(ty, false)`.
pub fn strip_option(ty: &syn::Type) -> (syn::Type, bool) {
    if let syn::Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
        && seg.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return (inner.clone(), true);
    }
    (ty.clone(), false)
}

/// Resolve a `syn::Type` to a `quent_model::ValueType` token stream.
///
/// Returns a token stream that constructs the appropriate `ValueType` variant.
/// Handles common Rust types: primitives, `String`, `Uuid`, `bool`,
/// `Option<T>` (resolves inner T and sets optional flag), `Vec<T>` (maps to
/// `ValueType::List`), `Ref<T>` (maps to `ValueType::Ref`).
/// Unknown types fall back to `ValueType::String`.
///
/// The second element of the returned tuple is `true` if the type is `Option<T>`.
pub fn resolve_value_type(ty: &syn::Type) -> (proc_macro2::TokenStream, bool) {
    use quote::quote;

    if let syn::Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
    {
        let ident_str = seg.ident.to_string();

        // Check for Option<T>
        if ident_str == "Option" {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments
                && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
            {
                let (inner_vt, _) = resolve_value_type(inner);
                return (inner_vt, true);
            }
            return (quote! { quent_model::ValueType::String }, true);
        }

        // Check for Vec<T>
        if ident_str == "Vec" {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments
                && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
            {
                let (inner_vt, _) = resolve_value_type(inner);
                return (
                    quote! { quent_model::ValueType::List(Box::new(#inner_vt)) },
                    false,
                );
            }
            return (
                quote! { quent_model::ValueType::List(Box::new(quent_model::ValueType::String)) },
                false,
            );
        }

        // Check for Ref<T>
        if ident_str == "Ref" {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments
                && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
            {
                let inner_name = quote! { #inner }.to_string();
                return (
                    quote! { quent_model::ValueType::Ref(#inner_name.to_string()) },
                    false,
                );
            }
            return (quote! { quent_model::ValueType::Ref(String::new()) }, false);
        }

        // Primitive and well-known types
        let vt = match ident_str.as_str() {
            "DynamicAttributes" => quote! { quent_model::ValueType::DynamicAttributes },
            "bool" => quote! { quent_model::ValueType::Bool },
            "u8" => quote! { quent_model::ValueType::U8 },
            "u16" => quote! { quent_model::ValueType::U16 },
            "u32" => quote! { quent_model::ValueType::U32 },
            "u64" => quote! { quent_model::ValueType::U64 },
            "i8" => quote! { quent_model::ValueType::I8 },
            "i16" => quote! { quent_model::ValueType::I16 },
            "i32" => quote! { quent_model::ValueType::I32 },
            "i64" => quote! { quent_model::ValueType::I64 },
            "f32" => quote! { quent_model::ValueType::F32 },
            "f64" => quote! { quent_model::ValueType::F64 },
            "String" => quote! { quent_model::ValueType::String },
            "Uuid" => quote! { quent_model::ValueType::Uuid },
            _ => {
                // Unknown type — try to resolve via EventMetadata
                let type_path_str = quote!(#ty).to_string();
                return (
                    quote! {
                        quent_model::ValueType::Struct(
                            #type_path_str.to_string(),
                            <#ty as quent_model::EventMetadata>::event_def().attributes,
                        )
                    },
                    false,
                );
            }
        };
        return (vt, false);
    }

    // Fallback for non-path types
    (quote! { quent_model::ValueType::String }, false)
}

/// Generate an expression converting a field (given its declared type) into
/// an `Option<quent_model::attributes::DynamicValue>`.
///
/// Most types delegate to the `ToAttributeValue` trait. `Vec<T>` of an
/// attribute struct is special-cased syntactically because the orphan rule
/// prevents implementing `ToAttributeValue for Vec<T>` outside the crate
/// defining the trait.
pub fn attribute_value_expr(
    field_expr: &proc_macro2::TokenStream,
    ty: &syn::Type,
) -> proc_macro2::TokenStream {
    use quote::quote;

    // Inner types of Vec<T> with a dedicated ToAttributeValue impl.
    const KNOWN_VEC_INNER: &[&str] = &[
        "bool", "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "f32", "f64", "String",
        "Uuid", "Ref",
    ];

    if let syn::Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
        && seg.ident == "Vec"
        && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(syn::GenericArgument::Type(syn::Type::Path(inner))) = args.args.first()
        && let Some(inner_seg) = inner.path.segments.last()
        && !KNOWN_VEC_INNER.contains(&inner_seg.ident.to_string().as_str())
    {
        // Vec of an attribute struct — convert element-wise.
        return quote! {
            Some(quent_model::attributes::DynamicValue::List(
                quent_model::attributes::DynamicList::Struct(
                    #field_expr
                        .iter()
                        .map(|v| quent_model::attributes::DynamicStruct(
                            quent_model::analyze::ExtractAttributes::extract_attributes(v),
                        ))
                        .collect(),
                ),
            ))
        };
    }

    quote! {
        quent_model::analyze::ToAttributeValue::to_attribute_value(&#field_expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;

    #[test]
    fn snake_case_basic() {
        let ident = Ident::new("FooBar", Span::call_site());
        assert_eq!(to_snake_case(&ident), "foo_bar");
    }

    #[test]
    fn snake_case_single_char() {
        let ident = Ident::new("A", Span::call_site());
        assert_eq!(to_snake_case(&ident), "a");
    }

    #[test]
    fn snake_case_consecutive_uppercase() {
        let ident = Ident::new("ABC", Span::call_site());
        assert_eq!(to_snake_case(&ident), "abc");
    }

    #[test]
    fn snake_case_already_lowercase() {
        let ident = Ident::new("hello", Span::call_site());
        assert_eq!(to_snake_case(&ident), "hello");
    }
}
