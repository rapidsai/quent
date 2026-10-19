// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates code for converting schema fields to dynamic attributes.

use proc_macro2::TokenStream;
use quent_events_build::common::module_ident;
use quent_schema::{DataType, Field, Schema};
use quote::{format_ident, quote};

pub(super) fn fields<'a>(
    schema: &Schema,
    fields: impl Iterator<Item = &'a Field>,
) -> Vec<TokenStream> {
    fields
        .map(|field| {
            let binding = module_ident(field.name());
            attribute(schema, field, quote!(#binding))
        })
        .collect()
}

fn attribute(schema: &Schema, field: &Field, value: TokenStream) -> TokenStream {
    let key = field.name().to_string();
    let value = nullable(schema, field.ty(), value);
    quote! { ::quent_log::ui::DynamicAttribute { key: #key.to_owned(), value: #value } }
}

fn nullable(schema: &Schema, ty: &DataType, value: TokenStream) -> TokenStream {
    match ty {
        DataType::Option(inner) if matches!(**inner, DataType::Option(_)) => {
            let projection = nullable(schema, inner, quote!(value));
            quote! { (#value).as_ref().and_then(|value| #projection) }
        }
        DataType::Option(inner) => {
            let projection = dynamic_value(schema, inner, quote!(value));
            quote! { (#value).as_ref().map(|value| #projection) }
        }
        _ => {
            let projection = dynamic_value(schema, ty, value);
            quote! { Some(#projection) }
        }
    }
}

fn dynamic_value(schema: &Schema, ty: &DataType, value: TokenStream) -> TokenStream {
    let (variant, payload) = payload(schema, ty, value);
    let variant = format_ident!("{variant}");
    quote! { ::quent_log::ui::DynamicValue::#variant(#payload) }
}

fn payload(schema: &Schema, ty: &DataType, value: TokenStream) -> (&'static str, TokenStream) {
    match ty {
        DataType::Bool => ("U8", quote!(u8::from(*(#value)))),
        DataType::Uuid => ("String", quote!((#value).to_string())),
        DataType::String => ("String", quote!((#value).clone())),
        DataType::U8 => ("U8", quote!(*(#value))),
        DataType::U16 => ("U16", quote!(*(#value))),
        DataType::U32 => ("U32", quote!(*(#value))),
        DataType::U64 => ("U64", quote!(*(#value))),
        DataType::I8 => ("I8", quote!(*(#value))),
        DataType::I16 => ("I16", quote!(*(#value))),
        DataType::I32 => ("I32", quote!(*(#value))),
        DataType::I64 => ("I64", quote!(*(#value))),
        DataType::F32 => ("F32", quote!(*(#value))),
        DataType::F64 => ("F64", quote!(*(#value))),
        DataType::DynamicRecord => (
            "Struct",
            quote!(::quent_log::ui::DynamicStruct((#value).0.clone())),
        ),
        DataType::Record(path) => {
            let fields = schema
                .record(path)
                .expect("validated record reference")
                .fields()
                .map(|field| {
                    let name = module_ident(field.name());
                    let projection = attribute(schema, field, quote!(value));
                    quote! { { let value = &(#value).#name; #projection } }
                });
            (
                "Struct",
                quote!(::quent_log::ui::DynamicStruct(vec![#(#fields),*])),
            )
        }
        DataType::EntityRef { data, .. } => {
            let data = data.as_ref().map(|ty| {
                let projection = nullable(schema, ty, quote!(value));
                quote! { , ::quent_log::ui::DynamicAttribute {
                    key: "data".to_owned(),
                    value: { let value = &(#value).data; #projection }
                } }
            });
            (
                "Struct",
                quote!(::quent_log::ui::DynamicStruct(vec![
                    ::quent_log::ui::DynamicAttribute::string("target", (#value).target.to_string()) #data
                ])),
            )
        }
        DataType::List(inner) => {
            let (variant, projection) = if matches!(**inner, DataType::Option(_)) {
                let projection = nullable(schema, inner, quote!(value));
                (
                    "Struct",
                    quote!(::quent_log::ui::DynamicStruct(vec![
                        ::quent_log::ui::DynamicAttribute { key: "value".to_owned(), value: #projection }
                    ])),
                )
            } else {
                payload(schema, inner, quote!(value))
            };
            let variant = format_ident!("{variant}");
            let elements = if matches!(
                **inner,
                DataType::String
                    | DataType::U8
                    | DataType::U16
                    | DataType::U32
                    | DataType::U64
                    | DataType::I8
                    | DataType::I16
                    | DataType::I32
                    | DataType::I64
                    | DataType::F32
                    | DataType::F64
            ) {
                quote! { (#value).clone() }
            } else {
                quote! { (#value).iter().map(|value| #projection).collect() }
            };
            (
                "List",
                quote!(::quent_log::ui::DynamicList::#variant(#elements)),
            )
        }
        DataType::Option(_) => unreachable!("options handled by nullable projection"),
    }
}
