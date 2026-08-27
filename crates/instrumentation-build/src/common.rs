// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared codegen helpers: identifier casing/escaping and attribute emission.

use convert_case::{Boundary, Case, Casing};
use proc_macro2::{Span, TokenStream};
use quent_schema::{Identifier, Path};
use quote::{ToTokens, quote};
use std::collections::HashSet;
use syn::Ident;

use crate::GenerateError;

/// Build a deduplicated `#[derive(..)]` attribute.
pub(crate) fn derive_attr(
    derives: &[&str],
    debug: bool,
    serialize: bool,
    deserialize: bool,
) -> Result<TokenStream, GenerateError> {
    let mut paths = Vec::new();
    if debug {
        paths.push(syn::parse_quote!(Debug));
    }
    if serialize {
        paths.push(syn::parse_quote!(::serde::Serialize));
    }
    if deserialize {
        paths.push(syn::parse_quote!(::serde::Deserialize));
    }
    paths.extend(
        derives
            .iter()
            .copied()
            .map(|derive| {
                syn::parse_str::<syn::Path>(derive).map_err(|source| GenerateError::InvalidDerive {
                    derive: derive.to_owned(),
                    source,
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    );
    for path in &mut paths {
        canonicalize_known_derive_path(path);
    }
    let mut seen = HashSet::new();
    paths.retain(|path| seen.insert(path.to_token_stream().to_string()));
    if paths.is_empty() {
        return Ok(quote! {});
    }
    Ok(quote! { #[derive(#(#paths),*)] })
}

fn canonicalize_known_derive_path(path: &mut syn::Path) {
    canonicalize_external_derive_path(path, "serde", &["Serialize", "Deserialize"]);
    if path_has_segments(path, &["std", "fmt", "Debug"])
        || path_has_segments(path, &["core", "fmt", "Debug"])
    {
        *path = syn::parse_quote!(Debug);
    }
}

fn path_has_segments(path: &syn::Path, names: &[&str]) -> bool {
    path.segments.len() == names.len()
        && path.segments.iter().zip(names).all(|(segment, name)| {
            segment.ident == *name && matches!(segment.arguments, syn::PathArguments::None)
        })
}

fn canonicalize_external_derive_path(
    path: &mut syn::Path,
    crate_name: &str,
    derive_names: &[&str],
) {
    if path.leading_colon.is_none()
        && path.segments.len() == 2
        && path.segments[0].ident == crate_name
        && derive_names
            .iter()
            .any(|derive| path.segments[1].ident == derive)
    {
        path.leading_colon = Some(Default::default());
    }
}

/// Build a `#[doc = ..]` attribute from `docs`.
pub(crate) fn doc_attr(docs: Option<&str>) -> TokenStream {
    match docs {
        Some(text) => quote! { #[doc = #text] },
        None => quote! {},
    }
}

/// Build a `#[doc = ..]` attribute from `docs`, falling back to `fallback` when
/// `docs` is `None`, so the item is always documented.
pub(crate) fn doc_attr_or(docs: Option<&str>, fallback: &str) -> TokenStream {
    let text = docs.unwrap_or(fallback);
    quote! { #[doc = #text] }
}

/// Case-convert a schema identifier without splitting letter/digit boundaries,
/// so names such as `u8` or `http2` are preserved rather than mangled.
pub(crate) fn to_case(id: &Identifier, case: Case) -> String {
    id.to_string()
        .remove_boundaries(&Boundary::digits())
        .to_case(case)
}

/// Return the Pascal-case type name for the final path segment.
pub(crate) fn path_name_pascal(path: &Path) -> String {
    to_case(path.name(), Case::Pascal)
}

/// Return the Rust module name for a path segment.
pub(crate) fn module_ident(segment: &Identifier) -> Ident {
    raw_ident(to_case(segment, Case::Snake))
}

/// Return a generated type path relative to `source_namespace`.
pub(crate) fn relative_type_path(
    path: &Path,
    source_namespace: &[Identifier],
    suffix: &str,
) -> TokenStream {
    let common = path
        .namespace()
        .iter()
        .zip(source_namespace)
        .take_while(|(left, right)| left == right)
        .count();
    let mut segments = Vec::new();
    segments.extend((common..source_namespace.len()).map(|_| quote! { super }));
    for segment in &path.namespace()[common..] {
        let module = module_ident(segment);
        segments.push(quote! { #module });
    }
    let ty = raw_ident(format!("{}{}", path_name_pascal(path), suffix));
    segments.push(quote! { #ty });
    quote! { #(#segments)::* }
}

/// Return a root type path relative to `source_namespace`.
pub(crate) fn relative_root_type(name: &str, source_namespace: &[Identifier]) -> TokenStream {
    let parents = source_namespace.iter().map(|_| quote! { super });
    let ty = raw_ident(name.to_owned());
    quote! { #(#parents::)* #ty }
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
