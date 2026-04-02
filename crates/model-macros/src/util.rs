// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use proc_macro2::Ident;

/// Converts a PascalCase identifier to snake_case.
pub fn to_snake_case(ident: &Ident) -> String {
    let s = ident.to_string();
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Converts a snake_case string to PascalCase.
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut result = first.to_uppercase().to_string();
                    result.extend(chars);
                    result
                }
            }
        })
        .collect()
}

/// Check if a field has a specific attribute.
pub fn field_has_attr(field: &syn::Field, attr_name: &str) -> bool {
    field.attrs.iter().any(|a| {
        a.path()
            .segments
            .last()
            .is_some_and(|seg| seg.ident == attr_name)
    })
}

/// Check for `#[resource_group]` or `#[resource_group(root)]` outer attribute.
///
/// Returns `Some(true)` for `#[resource_group(root)]`, `Some(false)` for
/// `#[resource_group]`, and `None` if the attribute is absent.
pub fn parse_resource_group_attr(input: &syn::DeriveInput) -> Option<bool> {
    for attr in &input.attrs {
        if attr
            .path()
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "resource_group")
        {
            // Check if it has (root) argument
            if let syn::Meta::List(list) = &attr.meta {
                if let Ok(ident) = syn::parse2::<Ident>(list.tokens.clone()) {
                    if ident == "root" {
                        return Some(true);
                    }
                }
            }
            return Some(false);
        }
    }
    None
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

    if let syn::Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            let ident_str = seg.ident.to_string();

            // Check for Option<T>
            if ident_str == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        let (inner_vt, _) = resolve_value_type(inner);
                        return (inner_vt, true);
                    }
                }
                return (quote! { quent_model::ValueType::String }, true);
            }

            // Check for Vec<T>
            if ident_str == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        let (inner_vt, _) = resolve_value_type(inner);
                        return (
                            quote! { quent_model::ValueType::List(Box::new(#inner_vt)) },
                            false,
                        );
                    }
                }
                return (
                    quote! { quent_model::ValueType::List(Box::new(quent_model::ValueType::String)) },
                    false,
                );
            }

            // Check for Ref<T>
            if ident_str == "Ref" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        let inner_name = quote! { #inner }.to_string();
                        return (
                            quote! { quent_model::ValueType::Ref(#inner_name.to_string()) },
                            false,
                        );
                    }
                }
                return (
                    quote! { quent_model::ValueType::Ref(String::new()) },
                    false,
                );
            }

            // Primitive and well-known types
            let vt = match ident_str.as_str() {
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
                _ => quote! { quent_model::ValueType::String }, // fallback
            };
            return (vt, false);
        }
    }

    // Fallback for non-path types
    (quote! { quent_model::ValueType::String }, false)
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
        assert_eq!(to_snake_case(&ident), "a_b_c");
    }

    #[test]
    fn snake_case_already_lowercase() {
        let ident = Ident::new("hello", Span::call_site());
        assert_eq!(to_snake_case(&ident), "hello");
    }

    #[test]
    fn pascal_case_basic() {
        assert_eq!(to_pascal_case("foo_bar"), "FooBar");
    }

    #[test]
    fn pascal_case_single_word() {
        assert_eq!(to_pascal_case("hello"), "Hello");
    }

    #[test]
    fn pascal_case_already_pascal() {
        assert_eq!(to_pascal_case("FooBar"), "FooBar");
    }

    #[test]
    fn pascal_case_empty_segments() {
        assert_eq!(to_pascal_case("foo__bar"), "FooBar");
    }
}
