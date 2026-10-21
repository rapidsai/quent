// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint;
use quent_ref_target::RefTargetConstraint;
use quent_schema::builder::{AnnotationsBuilder, EntityBuilder, SchemaBuilder};
use quent_schema::test_utils::{entity, event, field, path};
use quent_schema::{Annotations, DataType, Schema};

use crate::{EventOptions, GenerateError, Options, generate_str};

fn schema(binding: Option<&str>, entity_path: &str) -> Schema {
    SchemaBuilder::try_new("Demo")
        .unwrap()
        .with_entity(
            EntityBuilder::new(path(entity_path))
                .with_annotations(
                    AnnotationsBuilder::new()
                        .with_metadata("quent.codegen.rust.type", binding.map(str::to_owned))
                        .build()
                        .unwrap(),
                )
                .with_event(event("mark", [field("message", DataType::String)]))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

#[test]
fn uses_external_paths_throughout_instrumentation_and_combined_events() {
    let schema = schema(
        Some("::shared_types::SharedEvent"),
        "Jobs::Nested::Annotations",
    );
    let original = schema.clone();
    let source = generate_str(
        &schema,
        &Options {
            events: EventOptions {
                combined_event: true,
                serde: true,
                ..EventOptions::default()
            },
            collector_sink: true,
            ..Options::default()
        },
    )
    .unwrap();

    assert_eq!(schema, original);
    let compact = source.split_whitespace().collect::<String>();
    for expected in [
        "pubstructAnnotations;",
        "pubuse::shared_types::SharedEventasAnnotationsEvent;",
        "typePayload=::shared_types::SharedEvent;",
        "pubfnmark(",
        "message:String",
        "::shared_types::SharedEvent::Mark{message",
        "ExporterProvider<::shared_types::SharedEvent>",
        ".observer::<::shared_types::SharedEvent>(provider)",
        "deserialize_event::<::shared_types::SharedEvent",
        "Annotations(::shared_types::SharedEvent)",
        "From<::shared_types::SharedEvent>forDemoEvent",
        "From<::shared_types::SharedEvent>forJobsEvent",
        "From<::shared_types::SharedEvent>forNestedEvent",
        "<::shared_types::SharedEventas::quent_instrumentation::EventPayload>",
        "Jobs::Nested::Annotations",
    ] {
        assert!(compact.contains(expected), "missing {expected}:\n{source}");
    }
    assert!(!source.contains("impl ::quent_instrumentation::EventPayload for"));
}

#[test]
fn event_only_uses_extern_type_without_applying_event_derives() {
    let source = quent_events_build::generate_str(
        &schema(Some("::shared_types::AnnotationsEvent"), "Annotations"),
        &EventOptions {
            event_derives: &["not a valid derive"],
            ..EventOptions::default()
        },
        &events_runtime(),
    )
    .unwrap();

    assert!(source.contains("type Payload = ::shared_types::AnnotationsEvent;"));
    assert!(source.contains("::quent_events::EventPayload"));
    assert!(!source.contains("quent_instrumentation"));
    assert!(!source.contains("pub enum"));
    assert!(source.contains("pub use ::shared_types::AnnotationsEvent as AnnotationsEvent;"));
    assert!(!source.contains("impl ::quent_events::EventPayload for"));
}

#[test]
fn invalid_bindings_are_generation_errors() {
    for value in [
        None,
        Some(""),
        Some(" "),
        Some("::"),
        Some("::shared_types::"),
        Some("::shared_types::Event; fn injected() {}"),
        Some("Vec<Event>"),
        Some("::shared_types::Event::<u8>"),
        Some("&::shared_types::Event"),
        Some("<T as Trait>::Event"),
    ] {
        for instrumentation in [false, true] {
            let schema = schema(value, "Jobs::Annotations");
            let error = if instrumentation {
                generate_str(&schema, &Options::default())
            } else {
                quent_events_build::generate_str(
                    &schema,
                    &EventOptions::default(),
                    &events_runtime(),
                )
                .map_err(GenerateError::from)
            }
            .unwrap_err();
            assert!(
                matches!(
                    &error,
                    GenerateError::Events(quent_events_build::Error::InvalidExternType { path: entity, value: invalid, .. })
                        if entity == &path("Jobs::Annotations") && invalid.as_deref() == value
                ),
                "{error}"
            );
            assert!(error.to_string().contains("quent.codegen.rust.type"));
        }
    }
}

#[test]
fn external_reference_fields_keep_existing_mapping() {
    let bound = schema(Some("::shared_types::SharedEvent"), "Annotations");
    let annotations = bound.entities().next().unwrap().annotations().clone();
    let typed = AnnotationsBuilder::new()
        .with_constraint(RefTargetConstraint::NAME, Some("Target".to_owned()))
        .build()
        .unwrap();
    let schema = SchemaBuilder::try_new("Demo")
        .unwrap()
        .with_entity(entity("Target", [event("created", [])]))
        .with_entity(
            EntityBuilder::new(path("Annotations"))
                .with_annotations(annotations)
                .with_event(event(
                    "link",
                    [
                        field(
                            "typed",
                            DataType::EntityRef {
                                data: None,
                                annotations: typed,
                            },
                        ),
                        field(
                            "untyped",
                            DataType::Option(Box::new(DataType::List(Box::new(
                                DataType::EntityRef {
                                    data: Some(Box::new(DataType::U64)),
                                    annotations: Annotations::default(),
                                },
                            )))),
                        ),
                    ],
                ))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let source = generate_str(&schema, &Options::default()).unwrap();

    assert!(source.contains("typed: ::quent_instrumentation::EntityRef<Target>"));
    assert!(
        source.contains("untyped: Option<Vec<::quent_instrumentation::EntityRef<AnyEntity, u64>>>")
    );
    assert!(source.contains("pub enum TargetEvent"));
    assert!(!source.contains("pub enum AnnotationsEvent"));
}

fn events_runtime() -> quent_events_build::Runtime {
    quent_events_build::Runtime {
        events: syn::parse_quote!(::quent_events),
        entity_marker: syn::parse_quote!(::quent_events::EntityMarker),
        serde: None,
    }
}
