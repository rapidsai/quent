// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of record structs.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::{Record, Schema};
use quote::quote;

use crate::common::{derive_attr, doc_attr, raw_ident, to_case};
use crate::data_type::map_data_type;
use crate::{GenerateError, Options};

/// Record structs, as tokens, in declaration order.
///
/// # Panics
///
/// Panics if a field type nests deeper than [`crate::data_type::MAX_TYPE_DEPTH`].
pub(crate) fn generate_record_types(
    schema: &Schema,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let records: Vec<TokenStream> = schema
        .records()
        .map(|record| record_struct(record, opts))
        .collect::<Result<_, _>>()?;
    Ok(quote! { #(#records)* })
}

fn record_struct(record: &Record, opts: &Options) -> Result<TokenStream, GenerateError> {
    let ident = raw_ident(to_case(record.name(), Case::Pascal));
    let docs = doc_attr(record.annotations().docs());
    let derives = derive_attr(opts.record_derives)?;
    let fields: Vec<TokenStream> = record
        .fields()
        .map(|field| {
            let name = raw_ident(to_case(field.name(), Case::Snake));
            let ty = map_data_type(field.ty(), 0);
            let field_docs = doc_attr(field.annotations().docs());
            quote! { #field_docs pub #name: #ty }
        })
        .collect();
    if fields.is_empty() {
        Ok(quote! { #docs #derives pub struct #ident {} })
    } else {
        Ok(quote! {
            #docs
            #derives
            pub struct #ident {
                #(#fields),*
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::pretty;
    use quent_schema::DataType;
    use quent_schema::test_utils::{field, ident, record, schema};

    #[test]
    fn test_generate_record_types() {
        let s = schema(
            "M",
            [],
            [
                record("OnePrim", [field("a", DataType::U8)]),
                record(
                    "Nested",
                    [
                        field("inner", DataType::Record(ident("OnePrim"))),
                        field("list", DataType::List(Box::new(DataType::String))),
                    ],
                ),
                record("Empty", []),
            ],
        );
        let expected = quote! {
            pub struct OnePrim {
                pub a: u8
            }
            pub struct Nested {
                pub inner: OnePrim,
                pub list: Vec<String>
            }
            pub struct Empty {}
        };
        assert_eq!(
            pretty(generate_record_types(&s, &Options::default()).unwrap()),
            pretty(expected)
        );
    }
}
