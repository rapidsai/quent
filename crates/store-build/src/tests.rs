// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::{
    builder::{AnnotationsBuilder, EntityBuilder, SchemaBuilder},
    test_utils::{entity, event},
};

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
            events: crate::EventOptions {
                combined_event: false,
                ..crate::Options::default().events
            },
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
            events: crate::EventOptions {
                combined_event: true,
                ..crate::Options::default().events
            },
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
                events: crate::EventOptions {
                    combined_event: false,
                    ..crate::Options::default().events
                },
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

#[test]
fn filesystem_loading_requires_serde_but_native_stores_do_not() {
    let schema = SchemaBuilder::try_new("Demo")
        .unwrap()
        .with_entity(entity("Query", [event("created", [])]))
        .build()
        .unwrap();
    let mut opts = Options::default();
    opts.events.serde = false;
    assert!(matches!(
        generate_str(&schema, &opts),
        Err(GenerateError::FilesystemRequiresSerde)
    ));
    opts.filesystem = false;
    let source = generate_str(&schema, &opts).unwrap();
    assert!(source.contains("pub struct DemoStore"));
    assert!(!source.contains("::serde::Serialize"));
}

#[test]
fn invalid_fsm_is_a_generation_error_even_without_native_storage() {
    use quent_constraints::Constraint;
    use quent_schema::{DataType, test_utils::field};

    for data in [
        None,
        Some("invalid JSON".to_owned()),
        Some(r#"{"initial_state":"missing","transitions":[]}"#.to_owned()),
    ] {
        let entity = EntityBuilder::try_new("Task")
            .unwrap()
            .with_event(event("ready", [field("seq", DataType::U16)]))
            .with_annotations(
                AnnotationsBuilder::new()
                    .with_constraint(quent_fsm::FsmConstraint::NAME, data)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_entity(entity)
            .build()
            .unwrap();
        for native in [false, true] {
            assert!(matches!(
                generate_str(
                    &schema,
                    &Options {
                        entity_store: EntityStoreImplementations { native },
                        ..Options::default()
                    }
                ),
                Err(GenerateError::InvalidFsm(_))
            ));
        }
    }
}

#[test]
fn registers_fsm_validation_and_preserves_unknown_constraint_warnings() {
    let entity = quent_fsm::FsmEntityBuilder::new("Task".parse::<quent_schema::Path>().unwrap())
        .with_annotations(
            AnnotationsBuilder::new()
                .with_constraint("example.unknown.v1", None)
                .build()
                .unwrap(),
        )
        .with_states([
            quent_fsm::StateDecl {
                name: "ready".parse().unwrap(),
                attributes: vec![],
                to: vec!["done".parse().unwrap()],
                initial: true,
            },
            quent_fsm::StateDecl {
                name: "done".parse().unwrap(),
                attributes: vec![],
                to: vec![],
                initial: false,
            },
        ])
        .build()
        .unwrap();
    let schema = SchemaBuilder::try_new("Demo")
        .unwrap()
        .with_entity(entity)
        .build()
        .unwrap();
    let output = tempfile::tempdir().unwrap();
    let generated = generate(
        &schema,
        &Options {
            out_dir: output.path().to_owned(),
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(generated.warnings, ["example.unknown.v1"]);
}
