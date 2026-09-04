// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::builder::SchemaBuilder;
use quent_schema::test_utils::{entity, event};
use quent_yaml::parse_from_str;

use crate::{Exporters, GenerateError, Options, emit};

const DEMO: &str = include_str!("../../../../../examples/readme/model.yaml");

#[test]
fn generates_schema_driven_bridge() {
    let schema = parse_from_str(DEMO, None).unwrap().schema;
    let options = Options {
        instrumentation_path: "quent_readme_example".to_owned(),
        crate_name: "demo-bridge".to_owned(),
        exporters: Exporters::all(),
        ..Options::default()
    };
    let files = emit(&schema, &options).unwrap();
    assert_eq!(files.len(), schema.entities().count() + 4);
    let worker = files.iter().find(|file| file.name == "worker.rs").unwrap();
    assert!(worker.content.contains("pub struct WorkerHandle"));
    assert!(worker.content.contains("pub struct BridgeRecordDetails"));
    assert!(worker.content.contains("pub fn declaration"));
    let facade = files.iter().find(|file| file.name == "quent.hpp").unwrap();
    assert!(facade.content.contains("class Context final"));
    assert!(
        facade
            .content
            .contains("WorkerObserver> worker_observer() const")
    );
    assert!(facade.content.contains("class EntityId final"));
    assert!(facade.content.contains("WorkerId id() const"));
    assert!(facade.content.contains("std::optional<"));
    assert!(facade.content.contains("namespace quent::records"));
    assert_eq!(facade.content.matches("struct Details {").count(), 1);
    assert!(facade.content.contains("class WorkerObserver final"));
    assert!(facade.content.contains("struct QueueUsageRef {"));
    assert!(facade.content.contains("declaration_emitted() const"));
    assert!(facade.content.contains("static Context ndjson"));
    assert_eq!(facade.content.matches("value_.values.push_back").count(), 6);
    let usage = facade.content.find("struct QueueUsage {").unwrap();
    let reference = facade.content.find("struct QueueUsageRef {").unwrap();
    assert!(usage < reference);
    for file in files.iter().filter(|file| file.name.ends_with(".rs")) {
        syn::parse_file(&file.content).unwrap_or_else(|error| panic!("{}: {error}", file.name));
    }
}

#[test]
fn defaults_to_noop_exporter_only() {
    let schema = parse_from_str(DEMO, None).unwrap().schema;
    let files = emit(
        &schema,
        &Options {
            instrumentation_path: "quent_readme_example".to_owned(),
            ..Options::default()
        },
    )
    .unwrap();
    let context = files.iter().find(|file| file.name == "context.rs").unwrap();
    let facade = files.iter().find(|file| file.name == "quent.hpp").unwrap();
    assert!(!context.content.contains("quent_io::"));
    assert!(!facade.content.contains("Context::ndjson"));
}

#[test]
fn rejects_generated_name_collisions() {
    for schema in [
        r#"
quent: alpha
model: collision
entities:
  Context: { events: { emitted: {} } }
"#,
        r#"
quent: alpha
model: collision
entities:
  Server: { events: { uuid: {} } }
"#,
        r#"
quent: alpha
model: collision
entities:
  Server:
    events:
      fooBar: {}
      foo_bar: {}
"#,
    ] {
        let schema = parse_from_str(schema, None).unwrap().schema;
        assert!(matches!(
            emit(&schema, &Options::default()),
            Err(GenerateError::NameCollision { .. })
        ));
    }
}

#[test]
fn orders_dependent_public_value_types() {
    let schema = parse_from_str(
        r#"
quent: alpha
model: ordered
records:
  Envelope: { fields: { later: Later } }
  Later: { fields: { label: string } }
entities:
  Server:
    events:
      emitted: { attributes: { envelope: Envelope } }
"#,
        None,
    )
    .unwrap()
    .schema;
    let files = emit(&schema, &Options::default()).unwrap();
    let facade = files.iter().find(|file| file.name == "quent.hpp").unwrap();
    let later = facade.content.find("struct Later {").unwrap();
    let envelope = facade.content.find("struct Envelope {").unwrap();
    assert!(later < envelope);
}

#[test]
fn rejects_invalid_options() {
    let schema = parse_from_str(DEMO, None).unwrap().schema;
    for options in [
        Options {
            namespace: "quent::class".to_owned(),
            ..Options::default()
        },
        Options {
            bridge_path: "../gen".to_owned(),
            ..Options::default()
        },
    ] {
        assert!(matches!(
            emit(&schema, &options),
            Err(GenerateError::InvalidOption { .. })
        ));
    }
}

#[test]
fn has_no_model_crate_in_dependency_source() {
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("quent-model"));
}

#[test]
fn preserves_schema_namespaces() {
    let schema = SchemaBuilder::try_new("Namespaced")
        .unwrap()
        .with_entity(entity("Api::Request", [event("sent", [])]))
        .build()
        .unwrap();
    let files = emit(
        &schema,
        &Options {
            instrumentation_path: "instrumentation::namespaced".to_owned(),
            ..Options::default()
        },
    )
    .unwrap();
    let entity = files
        .iter()
        .find(|file| file.name == "api_request.rs")
        .unwrap();
    assert!(
        entity
            .content
            .contains("namespace = \"quent::detail::api::request\"")
    );
    assert!(
        entity
            .content
            .contains("instrumentation::namespaced::api::Request")
    );
}
