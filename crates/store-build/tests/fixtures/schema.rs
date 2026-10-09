// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::builder::AnnotationsBuilder;
use quent_schema::test_utils::{entity, event, event_with, field, record, record_type};
use quent_schema::{Cardinality, DataType, Schema};

fn reference(data: Option<DataType>) -> DataType {
    DataType::EntityRef {
        data: data.map(Box::new),
        annotations: AnnotationsBuilder::new()
            .with_constraint("quent.ref-target.v0.1.0", Some("Parent".to_owned()))
            .build()
            .unwrap(),
    }
}

pub(super) fn schema() -> Schema {
    quent_schema::test_utils::schema(
        "ScopedEvents",
        [
            entity("Parent", [event("created", [])]),
            entity("Stream", [event_with("tick", Cardinality::Multi, [])]),
            entity(
                "Nested::type::Worker",
                [
                    event(
                        "type",
                        [
                            field("string", DataType::String),
                            field("option", DataType::Option(Box::new(DataType::String))),
                            field("vec", DataType::List(Box::new(DataType::String))),
                            field("some", DataType::Bool),
                            field("type", DataType::String),
                            field("http2_code", DataType::U16),
                            field("shared", record_type("Shared")),
                            field("local", record_type("Nested::Record")),
                            field("parent", reference(None)),
                            field(
                                "peers",
                                DataType::List(Box::new(reference(Some(record_type("Shared"))))),
                            ),
                        ],
                    ),
                    event("renamed", [field("string", DataType::String)]),
                    event(
                        "metadata",
                        [
                            field("id", DataType::U64),
                            field("timestamp", DataType::U64),
                            field("data", DataType::String),
                            field("event_id", DataType::String),
                            field("event_timestamp", DataType::String),
                            field("field_0", DataType::String),
                        ],
                    ),
                    event_with(
                        "update",
                        Cardinality::Multi,
                        [field("string", DataType::String)],
                    ),
                ],
            ),
        ],
        [
            record("Shared", [field("label", DataType::String)]),
            record("Nested::Record", [field("count", DataType::U64)]),
        ],
    )
}
