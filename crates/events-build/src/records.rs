// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of record structs.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::Record;
use quote::quote;

use crate::{
    Error, Options, Runtime,
    common::{derive_attr, doc_attr, doc_attr_or, path_name_pascal, raw_ident, to_case},
    data_type::map_data_type,
};

/// Generates a Rust struct with one public Rust struct field per schema record
/// field.
pub(crate) fn record_struct(
    record: &Record,
    opts: &Options,
    event_runtime: &Runtime,
) -> Result<TokenStream, Error> {
    let record_pascal = path_name_pascal(record.path());
    let ident = raw_ident(record_pascal.clone());
    let docs = doc_attr_or(
        record.annotations().docs(),
        &format!("The `{}` record.", record.path()),
    );
    let derives = derive_attr(
        opts.record_derives,
        opts.debug,
        opts.serde,
        opts.serde,
        event_runtime.serde.as_ref(),
    )?;
    let fields = record
        .fields()
        .map(|field| {
            let name = raw_ident(to_case(field.name(), Case::Snake));
            let ty = map_data_type(field.ty(), 0, record.path().namespace(), event_runtime)?;
            let field_docs = doc_attr(field.annotations().docs());
            Ok::<_, Error>(quote! { #field_docs pub #name: #ty })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fields.is_empty() {
        Ok(quote! { #docs #derives pub struct #ident; })
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
    use quent_schema::{
        DataType,
        test_utils::{field, record, record_type},
    };

    #[test]
    fn generates_record_struct() {
        let record = record(
            "Nested",
            [
                field("inner", record_type("OnePrim")),
                field("list", DataType::List(Box::new(DataType::String))),
            ],
        );
        let expected = quote! {
            #[doc = "The `Nested` record."]
            pub struct Nested {
                pub inner: OnePrim,
                pub list: Vec<String>
            }
        };
        assert_eq!(
            pretty(
                record_struct(
                    &record,
                    &Options {
                        debug: false,
                        ..Options::default()
                    },
                    &crate::tests::runtime(),
                )
                .unwrap(),
            ),
            pretty(expected)
        );
    }
}
