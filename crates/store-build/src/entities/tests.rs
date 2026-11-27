// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::{
    Cardinality, DataType,
    test_utils::{entity, event, event_with, field, record, schema},
};

use crate::{EntityStoreImplementations, GenerateError, Options, generate_str};

#[test]
fn generation_defaults_to_native_and_is_independent_of_loading() {
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
        events: crate::EventOptions {
            combined_event: false,
            ..crate::Options::default().events
        },
        filesystem: false,
        ..Options::default()
    };
    let source = generate_str(&schema, &options).unwrap();
    assert!(!source.contains("pub type TaskStore"));
    assert!(source.contains("pub mod task"));
    assert!(source.contains("pub struct DemoStore"));
    assert!(!source.contains("pub mod entity_events"));
    assert!(!source.contains("pub use self::entity_events"));
    let source = generate_str(
        &schema,
        &Options {
            events: crate::EventOptions {
                combined_event: false,
                ..crate::Options::default().events
            },
            entity_store: EntityStoreImplementations { native: false },
            filesystem: false,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(!source.contains("pub mod task"));
    assert!(!source.contains("pub struct DemoStore"));
    assert!(!source.contains("pub trait TaskEvents"));
    assert!(!source.contains("pub struct CreatedHandle"));
    assert!(!source.contains("pub type TaskStore"));
    let source = generate_str(
        &schema,
        &Options {
            events: crate::EventOptions {
                debug: false,
                ..options.events
            },
            entity_store: EntityStoreImplementations { native: true },
            ..options
        },
    )
    .unwrap();
    assert!(source.contains("pub trait TaskEvents"));
    assert!(!source.contains("pub type NativeTaskEvents"));
    assert!(source.contains("pub struct CreatedHandle"));
    assert!(source.contains("pub struct UpdatedHandle"));
    assert!(!source.contains("filesystem::Model"));
    assert!(!source.contains("pub enum DemoEvent"));
    assert!(!source.contains("derive(Debug)"));
    assert!(!source.contains("CreatedPayload"));
    assert!(!source.contains("UpdatedPayload"));
}

fn assert_name_conflict(schema: quent_schema::Schema) {
    let result = generate_str(
        &schema,
        &Options {
            events: crate::EventOptions {
                combined_event: false,
                ..crate::Options::default().events
            },
            filesystem: false,
            ..Options::default()
        },
    );
    assert!(
        matches!(result, Err(GenerateError::EntityEventsNameConflict { .. })),
        "expected a name conflict, got {result:?}"
    );
}

#[test]
fn rejects_namespace_conflicts() {
    for names in [["Worker", "Worker::Child"], ["Foo::Task", "foo::Worker"]] {
        assert_name_conflict(schema(
            "Demo",
            names.map(|name| entity(name, [event("created", [])])),
            [],
        ));
    }
}

#[test]
fn rejects_normalized_event_name_conflicts() {
    assert_name_conflict(schema(
        "Demo",
        [entity(
            "Task",
            [event("first_name", []), event("FirstName", [])],
        )],
        [],
    ));
}

#[test]
fn rejects_field_accessor_name_conflicts() {
    for (names, data_type) in [
        (["field_name", "FieldName"], DataType::String),
        (["timestamp", "field_timestamp"], DataType::U64),
    ] {
        assert_name_conflict(schema(
            "Demo",
            [entity(
                "Task",
                [event(
                    "created",
                    names.map(|name| field(name, data_type.clone())),
                )],
            )],
            [],
        ));
    }
}

#[test]
fn rejects_reserved_event_names() {
    for name in [
        "id",
        "earliest_timestamp",
        "latest_timestamp",
        "num_events",
        "event_sequence",
        "event_index",
    ] {
        assert_name_conflict(schema("Demo", [entity("Task", [event(name, [])])], []));
    }
}

#[test]
fn entity_modules_keep_store_types_separate_from_schema_types() {
    let schema = schema(
        "Demo",
        [
            entity("Task", [event("created", [])]),
            entity("NativeTask", [event("created", [])]),
            entity("TaskEvents", [event("created", [])]),
            entity("Nested::Task", [event("created", [])]),
            entity("Nested::TaskStore", [event("created", [])]),
            entity("EntityEvents::Task", [event("created", [])]),
        ],
        [record("TaskEventIndex", [])],
    );
    generate_str(&schema, &Options::default()).unwrap();
}

#[test]
fn ref_tree_support_requires_the_schema_constraint() {
    use quent_schema::builder::AnnotationsBuilder;
    for ref_tree in [false, true] {
        let mut annotations = AnnotationsBuilder::new()
            .with_constraint("quent.ref-target.v0.1.0", Some("Root".to_owned()));
        if ref_tree {
            annotations = annotations.with_constraint("quent.ref-tree.v0.1.0", None);
        }
        let parent = DataType::EntityRef {
            data: None,
            annotations: annotations.build().unwrap(),
        };
        let schema = schema(
            "Demo",
            [
                entity("Root", [event("created", [])]),
                entity(
                    "Nested::Child",
                    [event(
                        "created",
                        [field(
                            "parent",
                            DataType::Option(Box::new(quent_schema::test_utils::record_type(
                                "Parent",
                            ))),
                        )],
                    )],
                ),
            ],
            [record("Parent", [field("reference", parent)])],
        );
        let source = generate_str(
            &schema,
            &Options {
                filesystem: false,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(
            source
                .matches("::quent_store::RefTreeEntityMarker for")
                .count(),
            if ref_tree { 2 } else { 0 }
        );
        assert_eq!(source.contains("::quent_store::RefTreeIndex"), ref_tree);
        assert_eq!(source.contains("pub fn build_ref_tree_index"), ref_tree);
    }
}

#[test]
fn parent_id_event_is_available_without_ref_tree_support() {
    generate_str(
        &schema("Demo", [entity("Task", [event("parent_id", [])])], []),
        &Options::default(),
    )
    .unwrap();
}

#[test]
fn rejects_cyclic_parent_type_graphs_during_generation() {
    use quent_schema::builder::AnnotationsBuilder;
    let reference = |target: &str| DataType::EntityRef {
        data: None,
        annotations: AnnotationsBuilder::new()
            .with_constraint("quent.ref-target.v0.1.0", Some(target.to_owned()))
            .with_constraint("quent.ref-tree.v0.1.0", None)
            .build()
            .unwrap(),
    };
    for entities in [
        vec![entity(
            "A",
            [event("created", [field("parent", reference("A"))])],
        )],
        vec![
            entity("A", [event("created", [field("parent", reference("B"))])]),
            entity("B", [event("created", [field("parent", reference("A"))])]),
        ],
    ] {
        assert!(matches!(
            generate_str(&schema("Demo", entities, []), &Options::default()),
            Err(GenerateError::RefTree(_))
        ));
    }
}
