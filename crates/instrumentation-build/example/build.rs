// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_instrumentation_build::{GenerateInfo, Options, generate};
use quent_schema::builder::{
    AnnotationsBuilder, EntityBuilder, EventBuilder, RecordBuilder, SchemaBuilder,
};
use quent_schema::test_utils::{field, ident};
use quent_schema::{Annotations, Cardinality, DataType, Schema};

/// Annotations carrying only a documentation string.
fn docs(text: &str) -> Annotations {
    let mut builder = AnnotationsBuilder::new();
    builder.set_docs(text);
    builder.build()
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");

    let schema = demo_schema()?;

    let opts = Options {
        event_derives: &["Debug"],
        record_derives: &["Debug"],
        ..Default::default()
    };

    let GenerateInfo { path, warnings } = generate(&schema, &opts)?;

    if !warnings.is_empty() {
        println!("cargo:warning= {}", warnings.join("\n"));
    }
    println!(
        "cargo:warning=instrumentation library written to {}",
        path.display()
    );

    Ok(())
}

fn demo_schema() -> std::result::Result<Schema, Box<dyn std::error::Error>> {
    let endpoint = RecordBuilder::new(ident("Endpoint"))
        .with_annotations(docs("A network endpoint."))
        .try_with_fields([
            field("host", DataType::String),
            field("port", DataType::U16),
        ])?
        .build();

    let meta = RecordBuilder::new(ident("Meta"))
        .try_with_fields([
            field("tags", DataType::List(Box::new(DataType::String))),
            field("extra", DataType::DynamicRecord),
        ])?
        .build();

    let connection = EntityBuilder::new(ident("Connection"))
        .with_annotations(docs("A client connection."))
        .try_with_events([
            EventBuilder::new(ident("opened"), Cardinality::Once)
                .try_with_fields([
                    field("peer", DataType::Record(ident("Endpoint"))),
                    field("session", DataType::Uuid),
                ])?
                .build(),
            EventBuilder::new(ident("data"), Cardinality::Multi)
                .try_with_fields([
                    field("bytes", DataType::U64),
                    field(
                        "meta",
                        DataType::Option(Box::new(DataType::Record(ident("Meta")))),
                    ),
                ])?
                .build(),
            EventBuilder::new(ident("closed"), Cardinality::Once).build(),
        ])?
        .build();

    let schema = SchemaBuilder::new(ident("Demo"))
        .try_with_records([endpoint, meta])?
        .try_with_entity(connection)?
        .build();

    Ok(schema)
}
