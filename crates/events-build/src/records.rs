// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of record structs.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::{Record, Schema};
use quote::quote;

use crate::{
    Error, Options, Runtime,
    common::{
        derive_attr, doc_attr, doc_attr_or, parse_extern_type, path_name_pascal, raw_ident, to_case,
    },
    data_type::map_data_type,
};

/// Generates a record struct or re-exports its extern type.
pub(crate) fn record_struct(
    schema: &Schema,
    record: &Record,
    opts: &Options,
    event_runtime: &Runtime,
) -> Result<TokenStream, Error> {
    let record_pascal = path_name_pascal(record.path());
    let ident = raw_ident(record_pascal.clone());
    if let Some(path) = parse_extern_type(record.path(), record.annotations())? {
        return Ok(quote! { pub use #path as #ident; });
    }
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
            let ty = map_data_type(
                schema,
                field.ty(),
                0,
                record.path().namespace(),
                event_runtime,
            )?;
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
                    &quent_schema::test_utils::schema("M", [], [record.clone()]),
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

    #[test]
    fn extern_record_uses_owner_traits() {
        use quent_schema::builder::{AnnotationsBuilder, RecordBuilder};

        let record = RecordBuilder::try_new("Message")
            .unwrap()
            .with_annotations(
                AnnotationsBuilder::new()
                    .with_metadata(
                        "quent.codegen.rust.type",
                        Some("::shared::SharedMessage".to_owned()),
                    )
                    .build()
                    .unwrap(),
            )
            .with_field(field("text", DataType::String))
            .build()
            .unwrap();
        let schema = quent_schema::test_utils::schema("M", [], [record]);
        let source = crate::generate_str(
            &schema,
            &Options {
                record_derives: &["invalid derive"],
                ..Options::default()
            },
            &crate::tests::runtime(),
        )
        .unwrap();

        assert!(source.contains("pub use ::shared::SharedMessage as Message;"));
    }

    #[test]
    fn invalid_extern_record_metadata_is_a_generation_error() {
        use quent_schema::builder::{AnnotationsBuilder, RecordBuilder};

        for value in [None, Some(""), Some("::"), Some("Vec<u8>")] {
            let record = RecordBuilder::try_new("Shared::Message")
                .unwrap()
                .with_annotations(
                    AnnotationsBuilder::new()
                        .with_metadata("quent.codegen.rust.type", value.map(str::to_owned))
                        .build()
                        .unwrap(),
                )
                .with_field(field("text", DataType::String))
                .build()
                .unwrap();
            let path = record.path().clone();
            let schema = quent_schema::test_utils::schema("M", [], [record]);
            let error = crate::generate_str(&schema, &Options::default(), &crate::tests::runtime())
                .unwrap_err();
            assert!(
                matches!(error, Error::InvalidExternType { path: invalid, .. } if invalid == path)
            );
        }
    }
}
