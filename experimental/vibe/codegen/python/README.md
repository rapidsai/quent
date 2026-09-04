# Schema PyO3 generator

`quent-schema-codegen-python` emits a PyO3 module and matching PEP 561 type
stubs. `Context` exposes scoped observers that construct UUID-owning handles.
Event fields are keyword-only and events carrying entity-reference data accept
typed mappings.

```rust
let schema = quent_yaml::parse_from_file("model.yaml")?.schema;
let options = quent_schema_codegen_python::Options {
    module_name: "application_events".to_owned(),
    instrumentation_path: "application_instrumentation::model".to_owned(),
    exporters: quent_schema_codegen_python::Exporters::all(),
    ..Default::default()
};
quent_schema_codegen_python::write_generated_files(
    &quent_schema_codegen_python::emit(&schema, &options)?,
    std::env::var("OUT_DIR")?,
)?;
```

Records are accepted as mappings. Targeted entity references accept either a
`Uuid` or the matching generated handle; references carrying data accept a
mapping with `target` and `data` fields.

`Context()` creates a no-op context. Observers and handles retain their scoped
telemetry runtime after `Context.close()`; exporter shutdown waits until the
last observer or handle is released. Once-cardinality events expose
`<event>_emitted()` predicates. Nested options are rejected during generation
because Python cannot distinguish `None` from `Some(None)`.
