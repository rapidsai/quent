// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::builder::SchemaBuilder;
use quent_schema::test_utils::{entity, event};
use quent_yaml::parse_from_str;

use crate::{Exporters, GenerateError, Options, emit, emit_stubs};

const DEMO: &str = include_str!("../../../../../examples/readme/model.yaml");

const NVTX_PROCESS: &str = r#"
quent: alpha
model: NvtxBridge
entities:
  Process:
    nvtx: true
    events:
      started:
        attributes:
          process: { os: process }
"#;

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
    assert!(bridge.content.contains("pub fn handle("));
    assert!(bridge.content.contains("pub struct PyThreadIdleHandle"));
    assert!(bridge.content.contains("pub struct PyThreadActiveHandle"));
    assert!(bridge.content.contains("-> PyResult<PyThreadActiveHandle>"));
    assert!(!bridge.content.contains("enum PyThreadState"));
    assert!(!bridge.content.contains("struct PyUuid"));
    assert!(!bridge.content.contains("pub fn active(&mut self, seq:"));
    assert!(bridge.content.contains(".declaration("));
    assert!(bridge.content.contains("PyClusterHandle"));
    assert!(
        bridge
            .content
            .contains("#[pyo3(signature = (options = None))]")
    );
    assert!(!bridge.content.contains("pub enum PySourceCapture"));
    assert!(bridge.content.contains("cast::<PyMapping>()"));
    assert!(!bridge.content.contains("cast::<PyDict>()"));
    for constructor in [
        "pub fn u8(",
        "pub fn u16(",
        "pub fn u32(",
        "pub fn u64(",
        "pub fn i8(",
        "pub fn i16(",
        "pub fn i32(",
        "pub fn i64(",
        "pub fn f32(",
        "pub fn f64(",
        "pub fn string(",
        "pub fn structure(",
        "pub fn u8_list(",
        "pub fn u16_list(",
        "pub fn u32_list(",
        "pub fn u64_list(",
        "pub fn i8_list(",
        "pub fn i16_list(",
        "pub fn i32_list(",
        "pub fn i64_list(",
        "pub fn f32_list(",
        "pub fn f64_list(",
        "pub fn string_list(",
        "pub fn struct_list(",
        "pub fn list(",
    ] {
        assert!(bridge.content.contains(constructor));
    }

    let stubs = emit_stubs(&schema, &options).unwrap();
    assert!(stubs[0].content.contains("def now_v7() -> uuid.UUID"));
    assert!(!stubs[0].content.contains("class Uuid:"));
    assert!(stubs[0].content.contains("class QuentError(Exception):"));
    assert!(
        stubs[0]
            .content
            .contains("class EventAlreadyEmittedError(QuentError):")
    );
    assert!(
        stubs[0]
            .content
            .contains("def ndjson(output_dir: str | PathLike[str]) -> ExporterOptions")
    );
    assert!(stubs[0].content.contains("class WorkerHandle:"));
    assert!(stubs[0].content.contains("class WorkerObserver:"));
    assert!(
        stubs[0]
            .content
            .contains("def handle(self, id: uuid.UUID | None = None) -> WorkerHandle")
    );
    assert!(stubs[0].content.contains("class ThreadIdleHandle:"));
    assert!(
        stubs[0]
            .content
            .contains("def active(self) -> ThreadActiveHandle")
    );
    assert!(!stubs[0].content.contains("def active(self, *, seq:"));
    assert!(
        stubs[0]
            .content
            .contains("def worker_observer(self) -> WorkerObserver")
    );
    assert!(
        stubs[0]
            .content
            .contains("cluster: ClusterHandle | uuid.UUID")
    );
    assert!(
        stubs[0]
            .content
            .contains("class QueueUsageRefDict(TypedDict):")
    );
    assert!(
        stubs[0]
            .content
            .contains("use_queue: QueueUsageRefInput | None")
    );
    assert!(
        stubs[0]
            .content
            .contains("QueueUsageRefInput: TypeAlias = QueueUsageRefDict | Mapping[str, object]")
    );
    assert!(stubs[0].content.contains("class DynamicValue:"));
    assert!(
        stubs[0]
            .content
            .contains("def list(values: Sequence[DynamicValue]) -> DynamicValue")
    );
    assert!(stubs[0].content.contains("custom: DynamicAttributes"));
    assert!(
        stubs[0]
            .content
            .contains("options: ExporterOptions | None = None")
    );
    assert!(stubs[0].content.contains("def closed(self) -> bool"));
    assert!(!stubs[0].content.contains("class SourceCapture:"));
    assert!(!stubs[0].content.contains("def none()"));

    let initial_thread = class_body(&stubs[0].content, "ThreadHandle");
    assert!(initial_thread.contains("def idle(self"));
    assert!(!initial_thread.contains("def active(self"));
    let idle_thread = class_body(&stubs[0].content, "ThreadIdleHandle");
    assert!(idle_thread.contains("def active(self) -> ThreadActiveHandle"));
    assert!(idle_thread.contains("def exit(self) -> ThreadExitHandle"));
    assert!(!idle_thread.contains("def idle(self"));
}

#[test]
fn keeps_private_nvtx_stream_out_of_python_api() {
    let schema = parse_from_str(NVTX_PROCESS, None).unwrap().schema;
    let options = Options {
        exporters: Exporters {
            ndjson: true,
            ..Exporters::default()
        },
        ..Options::default()
    };
    let bridge = emit(&schema, &options).unwrap().remove(0);

    assert!(bridge.content.contains("pub struct PyProcessHandle"));
    assert!(bridge.content.contains("pub fn started"));
    assert!(bridge.content.contains("native_id"));
    assert!(
        bridge
            .content
            .contains("HandleError::OnceAlreadyEmitted { .. }")
    );
    assert!(
        bridge
            .content
            .contains("HandleError::SourceActivation { .. }")
    );
    assert!(
        bridge
            .content
            .contains("EventAlreadyEmittedError::new_err(message)")
    );
    assert!(
        bridge
            .content
            .contains("SourceActivationError::new_err(message)")
    );
    assert!(bridge.content.contains(".map_err(__handle_error)"));
    assert!(bridge.content.contains("\"SourceActivationError\""));
    assert!(bridge.content.contains("pub enum PySourceCapture"));
    assert!(bridge.content.contains("PySourceCapture::Enabled"));
    assert!(bridge.content.contains("PySourceCapture::Disabled"));
    assert!(bridge.content.contains("ContextOptions::default()"));
    assert!(
        bridge
            .content
            .contains(".with_source_capture(source_capture)")
    );
    assert!(bridge.content.contains("try_new_with_options"));
    assert!(
        bridge
            .content
            .contains("source_capture = PySourceCapture::Enabled")
    );
    assert!(
        bridge
            .content
            .contains("module.add_class::<PySourceCapture>()?")
    );
    assert!(!bridge.content.contains("PyNvtxEventObserver"));
    assert!(!bridge.content.contains("PyNvtxEventHandle"));
    assert!(!bridge.content.contains("nvtx_event_observer"));

    let stubs = emit_stubs(&schema, &options).unwrap();
    let stubs = &stubs[0].content;
    assert!(stubs.contains("class ProcessObserver:"));
    assert!(stubs.contains("class ProcessHandle:"));
    assert!(stubs.contains("class EventAlreadyEmittedError(QuentError):"));
    assert!(stubs.contains("class SourceActivationError(QuentError):"));
    assert!(stubs.contains("class SourceCapture:"));
    assert!(stubs.contains("Enabled: SourceCapture"));
    assert!(stubs.contains("Disabled: SourceCapture"));
    assert!(stubs.contains("source_capture: SourceCapture = SourceCapture.Enabled"));
    assert!(stubs.contains("def started("));
    assert!(stubs.contains("native_id: int"));
    assert!(!stubs.contains("class NvtxEventObserver:"));
    assert!(!stubs.contains("class NvtxEventHandle:"));
    assert!(!stubs.contains("def nvtx_event_observer("));
}

fn class_body<'a>(stubs: &'a str, name: &str) -> &'a str {
    stubs
        .split_once(&format!("class {name}:"))
        .unwrap()
        .1
        .split("\nclass ")
        .next()
        .unwrap()
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
