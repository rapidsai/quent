// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::builder::SchemaBuilder;
use quent_schema::test_utils::{entity, event};
use quent_yaml::parse_from_str;

use crate::{Exporters, GenerateError, Options, emit, emit_stubs};

const DEMO: &str = include_str!("../../../../../examples/readme/model.yaml");

#[test]
fn generates_schema_driven_bridge_and_stubs() {
    let schema = parse_from_str(DEMO, None).unwrap().schema;
    let options = Options {
        module_name: "quent_demo".to_owned(),
        instrumentation_path: "quent_readme_example".to_owned(),
        exporters: Exporters::all(),
        ..Options::default()
    };
    let bridge = emit(&schema, &options).unwrap().remove(0);
    syn::parse_file(&bridge.content).unwrap();
    assert!(bridge.content.contains("pub struct PyWorkerHandle"));
    assert!(bridge.content.contains(".declaration("));
    assert!(bridge.content.contains("PyClusterHandle"));
    assert!(
        bridge
            .content
            .contains("#[pyo3(signature = (options = None))]")
    );
    assert!(bridge.content.contains("cast::<PyMapping>()"));
    assert!(!bridge.content.contains("cast::<PyDict>()"));

    let stubs = emit_stubs(&schema, &options).unwrap();
    assert!(
        stubs[0]
            .content
            .contains("class Uuid:\n    def __repr__(self) -> str: ...")
    );
    assert!(
        stubs[0]
            .content
            .contains("class ExporterOptions:\n    @staticmethod")
    );
    assert!(stubs[0].content.contains("class WorkerHandle:"));
    assert!(stubs[0].content.contains("class WorkerObserver:"));
    assert!(
        stubs[0]
            .content
            .contains("def worker_observer(self) -> WorkerObserver")
    );
    assert!(stubs[0].content.contains("cluster: ClusterHandle | Uuid"));
    assert!(
        stubs[0]
            .content
            .contains("class QueueUsageRefDict(TypedDict):")
    );
    assert!(
        stubs[0]
            .content
            .contains("use_queue: QueueUsageRefDict | None")
    );
    assert!(stubs[0].content.contains("custom: Mapping[str,"));
    assert!(
        stubs[0]
            .content
            .contains("options: ExporterOptions | None = None")
    );
}

#[test]
fn defaults_to_noop_exporter_only() {
    let schema = parse_from_str(DEMO, None).unwrap().schema;
    let bridge = emit(&schema, &Options::default()).unwrap().remove(0);
    let stubs = emit_stubs(&schema, &Options::default()).unwrap();
    assert!(!bridge.content.contains("quent_io::"));
    assert!(!stubs[0].content.contains("def ndjson"));
}

#[test]
fn rejects_nested_options() {
    let schema = parse_from_str(
        r#"
quent: alpha
model: nested
entities:
  Server:
    events:
      emitted:
        attributes:
          value: { option: { option: u32 } }
"#,
        None,
    )
    .unwrap()
    .schema;
    assert!(matches!(
        emit(&schema, &Options::default()),
        Err(GenerateError::UnsupportedType { .. })
    ));
    assert!(matches!(
        emit_stubs(&schema, &Options::default()),
        Err(GenerateError::UnsupportedType { .. })
    ));
}

#[test]
fn rejects_generated_name_collisions() {
    for schema in [
        r#"
quent: alpha
model: collision
records:
  Typed: { fields: { value: u32 } }
entities:
  Server: { events: { emitted: {} } }
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
  fooBar: { events: { emitted: {} } }
  foo_bar: { events: { emitted: {} } }
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
fn rejects_invalid_module_names() {
    let schema = parse_from_str(DEMO, None).unwrap().schema;
    for module_name in ["", "bad-name", "package.class"] {
        assert!(matches!(
            emit(
                &schema,
                &Options {
                    module_name: module_name.to_owned(),
                    ..Options::default()
                }
            ),
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
fn preserves_schema_namespaces_in_exported_names() {
    let schema = SchemaBuilder::try_new("Namespaced")
        .unwrap()
        .with_entity(entity("Api::Request", [event("sent", [])]))
        .build()
        .unwrap();
    let options = Options {
        instrumentation_path: "instrumentation::namespaced".to_owned(),
        ..Options::default()
    };
    let bridge = emit(&schema, &options).unwrap().remove(0);
    assert!(bridge.content.contains("pub struct PyApiRequestHandle"));
    assert!(bridge.content.contains("pub fn api_request_observer"));
    assert!(
        bridge
            .content
            .contains("instrumentation::namespaced::api::Request")
    );
}
