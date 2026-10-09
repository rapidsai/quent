// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::test_utils::{entity, event, event_with, field, schema};
use quent_schema::{Cardinality, DataType};

use crate::{GenerateError, Options, generate_str};

#[test]
fn generation_is_opt_in_and_independent_of_loading() {
    let schema = schema(
        "Demo",
        [entity(
            "Task",
            [
                event("created", []),
                event_with(
                    "updated",
                    Cardinality::Multi,
                    [field("name", DataType::String)],
                ),
            ],
        )],
        [],
    );
    let options = Options {
        combined_event: false,
        filesystem: false,
        ..Options::default()
    };
    let source = generate_str(&schema, &options).unwrap();
    assert!(!source.contains("pub mod entity_events"));
    let source = generate_str(
        &schema,
        &Options {
            entity_events: true,
            debug: false,
            ..options
        },
    )
    .unwrap();
    assert!(source.contains("pub trait TaskEvents"));
    assert!(source.contains("pub type NativeTaskEvents"));
    assert!(source.contains("pub struct CreatedPayload"));
    assert!(source.contains("pub struct UpdatedPayload"));
    assert!(!source.contains("filesystem::Model"));
    assert!(!source.contains("pub enum DemoEvent"));
    assert!(!source.contains("derive(Debug)"));
}

#[test]
fn rejects_namespace_and_normalized_name_conflicts() {
    let schemas = [
        schema(
            "Demo",
            [entity("EntityEvents::Task", [event("created", [])])],
            [],
        ),
        schema(
            "Demo",
            [
                entity("Worker", [event("created", [])]),
                entity("Worker::Child", [event("created", [])]),
            ],
            [],
        ),
        schema(
            "Demo",
            [
                entity("Foo::Task", [event("created", [])]),
                entity("foo::Worker", [event("created", [])]),
            ],
            [],
        ),
        schema(
            "Demo",
            [entity(
                "Task",
                [event("first_name", []), event("FirstName", [])],
            )],
            [],
        ),
        schema(
            "Demo",
            [entity(
                "Task",
                [event(
                    "created",
                    [
                        field("field_name", DataType::String),
                        field("FieldName", DataType::String),
                    ],
                )],
            )],
            [],
        ),
        schema("Demo", [entity("Task", [event("id", [])])], []),
        schema("Demo", [entity("Task", [event("event_storage", [])])], []),
        schema("Demo", [entity("Task", [event("properties", [])])], []),
    ];
    for schema in schemas {
        assert!(matches!(
            generate_str(
                &schema,
                &Options {
                    entity_events: true,
                    filesystem: false,
                    combined_event: false,
                    ..Options::default()
                }
            ),
            Err(GenerateError::EntityEventsNameConflict { .. })
        ));
    }
}
