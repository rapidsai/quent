// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::builder::{AnnotationsBuilder, EntityBuilder, SchemaBuilder};
use quent_schema::test_utils::{entity, event};

use super::*;

#[test]
fn generates_nested_retrieval_apis_with_optional_model_loading() {
    let schema = SchemaBuilder::try_new("Demo")
        .unwrap()
        .with_entity(entity("Foo::Query", [event("created", [])]))
        .with_entity(entity("Foo::Nested::Task", [event("created", [])]))
        .build()
        .unwrap();

    let default_source = generate_str(&schema, &Options::default()).unwrap();

    assert!(default_source.contains("event::EntityMarkerInModel<Demo> for foo::Query"));
    assert!(default_source.contains("event::EntityMarkerInModel<Demo> for foo::nested::Task"));
    assert!(default_source.contains("pub enum DemoEvent"));
    assert!(default_source.contains("impl ::quent_store::event::filesystem::Model for Demo"));
    assert!(default_source.contains("import_for_entity::<foo::Query>()"));
    assert!(default_source.contains("import_for_entity::<foo::nested::Task>()"));

    let entity_events_source = generate_str(
        &schema,
        &Options {
            combined_event: false,
            filesystem: false,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(!entity_events_source.contains("pub enum DemoEvent"));
    assert!(!entity_events_source.contains("filesystem::Model for Demo"));

    let events_only_source = generate_str(
        &schema,
        &Options {
            combined_event: true,
            filesystem: false,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(events_only_source.contains("pub enum DemoEvent"));
    assert!(!events_only_source.contains("filesystem::Model for Demo"));

    assert!(matches!(
        generate_str(
            &schema,
            &Options {
                combined_event: false,
                filesystem: true,
                ..Options::default()
            }
        ),
        Err(GenerateError::FilesystemRequiresCombinedEvent)
    ));
}

#[test]
fn generate_returns_unregistered_constraint_warnings() {
    let annotations = AnnotationsBuilder::new()
        .with_constraint("example.unknown.v0.1.0", None)
        .build()
        .unwrap();
    let query = EntityBuilder::try_new("Query")
        .unwrap()
        .with_event(event("created", []))
        .with_annotations(annotations)
        .build()
        .unwrap();
    let schema = SchemaBuilder::try_new("Demo")
        .unwrap()
        .with_entity(query)
        .build()
        .unwrap();
    let output = tempfile::tempdir().unwrap();
    let options = Options {
        out_dir: output.path().to_owned(),
        ..Options::default()
    };

    let generated = generate(&schema, &options).unwrap();

    assert_eq!(generated.warnings, ["example.unknown.v0.1.0"]);
}
