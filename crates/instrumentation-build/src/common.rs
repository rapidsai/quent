// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared codegen helpers: identifier casing/escaping and attribute emission.

use convert_case::{Boundary, Case, Casing};
use proc_macro2::{Span, TokenStream};
use quent_schema::Identifier;
use quote::quote;
use syn::Ident;

use crate::GenerateError;

/// Build a `#[derive(..)]` attribute from `derives`.
pub(crate) fn derive_attr(derives: &[&str]) -> Result<TokenStream, GenerateError> {
    if derives.is_empty() {
        return Ok(quote! {});
    }
    let paths = derives
        .iter()
        .copied()
        .map(|d| {
            syn::parse_str::<syn::Path>(d).map_err(|source| GenerateError::InvalidDerive {
                derive: d.to_owned(),
                source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! { #[derive(#(#paths),*)] })
}

/// Build a `#[doc = ..]` attribute from `docs`.
pub(crate) fn doc_attr(docs: Option<&str>) -> TokenStream {
    match docs {
        Some(text) => quote! { #[doc = #text] },
        None => quote! {},
    }
}

/// Case-convert a schema identifier without splitting letter/digit boundaries,
/// so names such as `u8` or `http2` are preserved rather than mangled.
pub(crate) fn to_case(id: &Identifier, case: Case) -> String {
    const KEEP_DIGITS: &[Boundary] = &[
        Boundary::LOWER_DIGIT,
        Boundary::UPPER_DIGIT,
        Boundary::DIGIT_LOWER,
        Boundary::DIGIT_UPPER,
    ];
    id.to_string().without_boundaries(KEEP_DIGITS).to_case(case)
}

/// Build an identifier from an already-cased name, raw-escaping Rust keywords.
/// The keywords that cannot be raw (`crate`, `self`, `super`, `Self`) instead
/// receive a trailing underscore.
pub(crate) fn raw_ident(name: String) -> Ident {
    const NON_RAW: &[&str] = &["crate", "self", "super", "Self"];
    if NON_RAW.contains(&name.as_str()) {
        Ident::new(&format!("{name}_"), Span::call_site())
    } else if syn::parse_str::<Ident>(&name).is_ok() {
        Ident::new(&name, Span::call_site())
    } else {
        Ident::new_raw(&name, Span::call_site())
    }
}

/// Pretty-print tokens the same way the generators do, for comparing generated
/// source against `quote!`-built expectations in tests.
#[cfg(test)]
pub(crate) fn pretty(tokens: TokenStream) -> String {
    prettyplease::unparse(&syn::parse2::<syn::File>(tokens).expect("tokens form a valid file"))
}
