// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::builder::SchemaBuilder;
use quent_schema::test_utils::{entity, event};
use quent_yaml::parse_from_str;

use crate::{
    Exporters, GenerateError, GeneratedFile, NvtxSupport, Options, bridge_module_declaration, emit,
    write_bridge_files_to,
};

const DEMO: &str = include_str!("../../../../../examples/readme/model.yaml");

#[test]
fn nvtx_support_is_opt_in_and_preserves_disabled_contract() {
    let schema = parse_from_str(DEMO, None).unwrap().schema;
    assert_eq!(Options::default().nvtx, NvtxSupport::Disabled);
    let files = emit(
        &schema,
        &Options {
            exporters: Exporters::all(),
            ..Options::default()
        },
    )
    .unwrap();
    let context = files.iter().find(|file| file.name == "context.rs").unwrap();
    let facade = files.iter().find(|file| file.name == "quent.hpp").unwrap();
    assert!(
        context
            .content
            .contains("fn create_context(options: Box<ExporterOptions>) -> Result<Box<Context>>")
    );
    assert!(!context.content.contains("NvtxCapture"));
    assert!(!context.content.contains("nvtx_injection"));
    assert!(!context.content.contains("nvtx_bridge"));
    assert!(!context.content.contains("quent_nvtx_events"));
    assert!(!facade.content.contains("NvtxCapture"));
}

#[test]
fn generates_nvtx_capture_for_every_exporter() {
    let schema = parse_from_str(DEMO, None).unwrap().schema;
    for exporters in [Exporters::default(), Exporters::all()] {
        let files = emit(
            &schema,
            &Options {
                nvtx: NvtxSupport::Enabled,
                exporters,
                ..Options::default()
            },
        )
        .unwrap();
        let context = files.iter().find(|file| file.name == "context.rs").unwrap();
        let facade = files.iter().find(|file| file.name == "quent.hpp").unwrap();
        assert!(context.content.contains("enum NvtxCapture"));
        assert!(context.content.contains("nvtx_capture: NvtxCapture"));
        assert_eq!(
            context
                .content
                .contains("quent_nvtx_bridge::Capture::install("),
            exporters.any(),
        );
        assert_eq!(
            context
                .content
                .contains("observer::<quent_nvtx_events::NvtxEvent>"),
            exporters.any(),
        );
        assert!(!context.content.contains("nvtx_injection"));
        assert!(!context.content.contains("_nvtx_observer"));
        let fields = &context.content[context.content.find("pub struct Context {").unwrap()..];
        assert!(fields.contains("_nvtx_capture: Option<quent_nvtx_bridge::Capture>"));
        let capture = fields.find("_nvtx_capture:").unwrap();
        let inner = fields.find("pub(crate) inner:").unwrap();
        assert!(capture < inner);
        assert!(facade.content.contains("NvtxCapture::Disabled"));
        for (enabled, name) in [
            (exporters.ndjson, "ndjson"),
            (exporters.msgpack, "msgpack"),
            (exporters.postcard, "postcard"),
            (exporters.collector, "collector"),
        ] {
            assert_eq!(
                facade.content.contains(&format!("static Context {name}(")),
                enabled,
            );
        }
        for file in files.iter().filter(|file| file.name.ends_with(".rs")) {
            syn::parse_file(&file.content).unwrap_or_else(|error| panic!("{}: {error}", file.name));
        }
    }
}

#[test]
fn nvtx_names_are_reserved_only_when_support_is_enabled() {
    for name in ["NvtxCapture", "NvtxEvent"] {
        let schema = parse_from_str(
            format!("quent: alpha\nmodel: Test\nentities:\n  {name}: {{ events: {{ emitted: {{}} }} }}\n"),
            None,
        )
        .unwrap()
        .schema;
        assert!(emit(&schema, &Options::default()).is_ok());
        assert!(matches!(
            emit(
                &schema,
                &Options {
                    nvtx: NvtxSupport::Enabled,
                    ..Options::default()
                }
            ),
            Err(GenerateError::NameCollision { .. })
        ));
    }
}

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
    assert!(worker.content.contains("pub fn handle(&self)"));
    assert!(worker.content.contains("pub fn handle_with_id(&self"));
    assert!(worker.content.contains("pub fn id(&self) -> ffi::UUID"));
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
    assert!(facade.content.contains("WorkerObserver::handle() const"));
    assert!(
        facade
            .content
            .contains("WorkerObserver::handle(WorkerId id) const")
    );
    assert!(facade.content.contains("struct Worker final {}"));
    assert!(
        facade
            .content
            .contains("class Handle<::quent::Worker> final")
    );
    assert!(facade.content.contains("struct Thread final {}"));
    assert!(facade.content.contains("namespace quent::thread_state {"));
    assert!(
        facade
            .content
            .contains("class FsmHandle<::quent::Thread, ::quent::thread_state::Active> final")
    );
    assert!(facade.content.contains(
        "::quent::FsmHandle<::quent::Thread, ::quent::thread_state::Idle> idle(::quent::thread::Idle data) &&"
    ));
    assert!(facade.content.contains(
        "::quent::FsmHandle<::quent::Thread, ::quent::thread_state::Active> active() &&"
    ));
    assert!(
        facade
            .content
            .contains("class DynamicFsmHandle<::quent::Thread> final")
    );
    assert!(facade.content.contains("ThreadId id() const"));
    assert!(
        facade
            .content
            .contains("DynamicFsmHandle<::quent::Thread> into_dynamic() &&")
    );
    assert!(
        facade
            .content
            .contains("std::optional<FsmHandle<::quent::Thread, State>> try_into() &&")
    );
    assert!(
        facade
            .content
            .contains("struct FsmStateIndex<::quent::Thread, ::quent::thread_state::Active>")
    );
    assert!(!facade.content.contains("ThreadActiveHandle"));
    assert!(!facade.content.contains("friend class ThreadHandle"));
    assert!(!facade.content.contains("struct Active {"));
    assert!(!facade.content.contains("std::uint16_t seq"));
    assert!(facade.content.contains("struct QueueUsageRef {"));
    assert!(facade.content.contains("declaration_emitted() const"));
    assert!(facade.content.contains("static Context ndjson"));
    for value_type in [
        "std::nullptr_t",
        "std::uint8_t value",
        "std::uint16_t value",
        "std::uint32_t value",
        "std::uint64_t value",
        "std::int8_t value",
        "std::int16_t value",
        "std::int32_t value",
        "std::int64_t value",
        "float value",
        "double value",
        "std::string value",
        "const char* value",
        "bool value",
        "DynamicAttributes value",
        "std::vector<std::uint8_t> values",
        "std::vector<std::uint16_t> values",
        "std::vector<std::uint32_t> values",
        "std::vector<std::uint64_t> values",
        "std::vector<std::int8_t> values",
        "std::vector<std::int16_t> values",
        "std::vector<std::int32_t> values",
        "std::vector<std::int64_t> values",
        "std::vector<float> values",
        "std::vector<double> values",
        "std::vector<std::string> values",
        "std::vector<DynamicAttributes> values",
        "DynamicList value",
    ] {
        assert!(
            facade
                .content
                .contains(&format!("void add(std::string key, {value_type})"))
        );
    }
    assert_eq!(facade.content.matches("void add(").count(), 28);
    assert!(!facade.content.contains("void add_u8("));
    let dynamic = files
        .iter()
        .find(|file| file.name == "dynamic_attributes.rs")
        .unwrap();
    assert!(
        dynamic
            .content
            .contains("pub struct DynamicAttributesStorage")
    );
    assert!(dynamic.content.contains("pub struct DynamicListStorage"));
    assert!(
        dynamic
            .content
            .contains("pub storage: Vec<Box<DynamicAttributesStorage>>")
    );
    assert!(!dynamic.content.contains("DynamicAttributeKind"));
    assert!(!dynamic.content.contains("pub struct DynamicAttribute {"));
    for function in [
        "dynamic_attributes_add_null",
        "dynamic_attributes_add_u8",
        "dynamic_attributes_add_u16",
        "dynamic_attributes_add_u32",
        "dynamic_attributes_add_u64",
        "dynamic_attributes_add_i8",
        "dynamic_attributes_add_i16",
        "dynamic_attributes_add_i32",
        "dynamic_attributes_add_i64",
        "dynamic_attributes_add_f32",
        "dynamic_attributes_add_f64",
        "dynamic_attributes_add_string",
        "dynamic_attributes_add_bool",
        "dynamic_attributes_add_structure",
        "dynamic_attributes_add_u8_list",
        "dynamic_attributes_add_u16_list",
        "dynamic_attributes_add_u32_list",
        "dynamic_attributes_add_u64_list",
        "dynamic_attributes_add_i8_list",
        "dynamic_attributes_add_i16_list",
        "dynamic_attributes_add_i32_list",
        "dynamic_attributes_add_i64_list",
        "dynamic_attributes_add_f32_list",
        "dynamic_attributes_add_f64_list",
        "dynamic_attributes_add_string_list",
        "dynamic_attributes_add_struct_list",
        "dynamic_attributes_add_list",
        "dynamic_list_list",
    ] {
        assert!(dynamic.content.contains(function));
    }
    let usage = facade.content.find("struct QueueUsage {").unwrap();
    let reference = facade.content.find("struct QueueUsageRef {").unwrap();
    assert!(usage < reference);
    for file in files.iter().filter(|file| file.name.ends_with(".rs")) {
        assert!(!file.content.contains("crate::bridge"));
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
  Server: { events: { id: {} } }
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
        r#"
quent: alpha
model: collision
entities:
  Handle: { events: { emitted: {} } }
"#,
        r#"
quent: alpha
model: collision
entities:
  Worker:
    events:
      worker_id: { attributes: { value: u32 } }
"#,
        r#"
quent: alpha
model: collision
fsms:
  Job:
    states:
      new: { initial: true, to: [done] }
      done: {}
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
fn prunes_stale_generated_bridge_files() {
    let out_dir = std::env::temp_dir().join(format!(
        "quent-cpp-generated-files-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir(&out_dir).unwrap();
    let options = Options::default();
    write_bridge_files_to(
        &out_dir,
        &[
            GeneratedFile {
                name: "old.rs".to_owned(),
                content: String::new(),
            },
            GeneratedFile {
                name: "quent.hpp".to_owned(),
                content: String::new(),
            },
        ],
        &options,
    )
    .unwrap();
    write_bridge_files_to(
        &out_dir,
        &[
            GeneratedFile {
                name: "current.rs".to_owned(),
                content: String::new(),
            },
            GeneratedFile {
                name: "quent.hpp".to_owned(),
                content: String::new(),
            },
        ],
        &options,
    )
    .unwrap();

    assert!(!out_dir.join("gen/old.rs").exists());
    assert!(out_dir.join("gen/current.rs").is_file());
    std::fs::remove_dir_all(out_dir).unwrap();
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
fn supports_boolean_list_conversion() {
    let schema = parse_from_str(
        r#"
quent: alpha
model: lists
entities:
  Batch:
    events:
      recorded:
        attributes:
          flags: { list: bool }
"#,
        None,
    )
    .unwrap()
    .schema;
    let files = emit(&schema, &Options::default()).unwrap();
    let facade = files.iter().find(|file| file.name == "quent.hpp").unwrap();
    assert!(facade.content.contains("std::vector<bool>"));
    assert!(facade.content.contains("for (auto&& item : input)"));
}

#[test]
fn escapes_rust_keyword_bridge_modules() {
    let schema = parse_from_str(
        r#"
quent: alpha
model: keywords
entities:
  Type: { events: { emitted: {} } }
"#,
        None,
    )
    .unwrap()
    .schema;
    let files = emit(&schema, &Options::default()).unwrap();
    assert!(files.iter().any(|file| file.name == "type.rs"));
    assert_eq!(
        bridge_module_declaration("type.rs", "gen"),
        "#[path = \"gen/type.rs\"]\npub mod r#type;\n"
    );
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
    let facade = files.iter().find(|file| file.name == "quent.hpp").unwrap();
    assert!(
        facade
            .content
            .contains("namespace quent::api { struct Request final {}; }")
    );
    assert!(
        facade
            .content
            .contains("class Handle<::quent::api::Request> final")
    );
}
