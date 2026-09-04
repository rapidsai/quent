// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use convert_case::{Boundary, Case, Casing};
use proc_macro2::TokenStream;
use quent_schema::{Identifier, Path};
use quote::quote;

pub(crate) fn to_case(value: impl ToString, case: Case) -> String {
    value
        .to_string()
        .remove_boundaries(&Boundary::digits())
        .to_case(case)
}

pub(crate) fn raw_ident(name: impl Into<String>) -> syn::Ident {
    let name = name.into();
    const NON_RAW: &[&str] = &["crate", "self", "super", "Self"];
    if NON_RAW.contains(&name.as_str()) {
        quote::format_ident!("{name}_")
    } else if syn::parse_str::<syn::Ident>(&name).is_ok() {
        quote::format_ident!("{name}")
    } else {
        syn::Ident::new_raw(&name, proc_macro2::Span::call_site())
    }
}

pub(crate) fn rust_path(root: &syn::Path, path: &Path, suffix: &str) -> TokenStream {
    let modules = path
        .namespace()
        .iter()
        .map(|part| raw_ident(to_case(part, Case::Snake)));
    let ty = raw_ident(format!("{}{}", to_case(path.name(), Case::Pascal), suffix));
    quote! { #root::#(#modules::)*#ty }
}

pub(crate) fn model_path(root: &syn::Path, name: &Identifier) -> TokenStream {
    let model = raw_ident(to_case(name, Case::Pascal));
    quote! { #root::#model }
}

pub(crate) fn path_pascal(path: &Path) -> String {
    path.namespace()
        .iter()
        .chain(std::iter::once(path.name()))
        .map(|part| to_case(part, Case::Pascal))
        .collect()
}

pub(crate) fn path_snake(path: &Path) -> String {
    path.namespace()
        .iter()
        .chain(std::iter::once(path.name()))
        .map(|part| to_case(part, Case::Snake))
        .collect::<Vec<_>>()
        .join("_")
}

pub(crate) fn pretty(tokens: TokenStream) -> Result<String, syn::Error> {
    syn::parse2::<syn::File>(tokens).map(|file| prettyplease::unparse(&file))
}

pub(crate) fn cxx_namespace(base: &str, path: &Path) -> String {
    std::iter::once(base.to_owned())
        .chain(
            path.namespace()
                .iter()
                .chain(std::iter::once(path.name()))
                .map(|part| cxx_safe(&to_case(part, Case::Snake))),
        )
        .collect::<Vec<_>>()
        .join("::")
}

pub(crate) fn cxx_safe(name: &str) -> String {
    if CXX_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_owned()
    }
}

const CXX_KEYWORDS: &[&str] = &[
    "alignas",
    "alignof",
    "and",
    "and_eq",
    "asm",
    "auto",
    "bitand",
    "bitor",
    "bool",
    "break",
    "case",
    "catch",
    "char",
    "char8_t",
    "char16_t",
    "char32_t",
    "class",
    "compl",
    "concept",
    "const",
    "consteval",
    "constexpr",
    "constinit",
    "const_cast",
    "continue",
    "co_await",
    "co_return",
    "co_yield",
    "decltype",
    "default",
    "delete",
    "do",
    "double",
    "dynamic_cast",
    "else",
    "enum",
    "explicit",
    "export",
    "extern",
    "false",
    "float",
    "for",
    "friend",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "mutable",
    "namespace",
    "new",
    "noexcept",
    "not",
    "not_eq",
    "nullptr",
    "operator",
    "or",
    "or_eq",
    "private",
    "protected",
    "public",
    "register",
    "reinterpret_cast",
    "requires",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "static_assert",
    "static_cast",
    "struct",
    "switch",
    "template",
    "this",
    "thread_local",
    "throw",
    "true",
    "try",
    "typedef",
    "typeid",
    "typename",
    "union",
    "unsigned",
    "using",
    "virtual",
    "void",
    "volatile",
    "wchar_t",
    "while",
    "xor",
    "xor_eq",
];
