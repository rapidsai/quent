// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_instrumentation_build::{GenerateInfo, Options, generate};
use quent_schema::builder::{
    AnnotationsBuilder, EntityBuilder, EventBuilder, RecordBuilder, SchemaBuilder,
};
use quent_schema::test_utils::{field, ident};
use quent_schema::{Cardinality, DataType, Schema};

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
        .annotations(
            AnnotationsBuilder::new()
                .docs("A network endpoint.")
                .build(),
        )
        .fields([
            field("host", DataType::String),
            field("port", DataType::U16),
        ])
        .unwrap()
        .build();

    let meta = RecordBuilder::new(ident("Meta"))
        .fields([
            field("tags", DataType::List(Box::new(DataType::String))),
            field("extra", DataType::DynamicRecord),
        ])
        .unwrap()
        .build();

    let connection = EntityBuilder::new(ident("Connection"))
        .annotations(
            AnnotationsBuilder::new()
                .docs("A client connection.")
                .build(),
        )
        .events([
            EventBuilder::new(ident("opened"), Cardinality::Once)
                .fields([
                    field("peer", DataType::Record(ident("Endpoint"))),
                    field("session", DataType::Uuid),
                ])
                .unwrap()
                .build(),
            EventBuilder::new(ident("data"), Cardinality::Multi)
                .fields([
                    field("bytes", DataType::U64),
                    field(
                        "meta",
                        DataType::Option(Box::new(DataType::Record(ident("Meta")))),
                    ),
                ])
                .unwrap()
                .build(),
            EventBuilder::new(ident("closed"), Cardinality::Once).build(),
        ])
        .unwrap()
        .build();

    let schema = SchemaBuilder::new(ident("Demo"))
        .records([endpoint, meta])
        .unwrap()
        .entities([connection])
        .unwrap()
        .build();

    Ok(schema)
}
