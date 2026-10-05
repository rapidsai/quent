# Schema PyO3 generator

`quent-schema-codegen-python` emits a PyO3 module and matching PEP 561 type
stubs. `Context` exposes scoped observers that construct UUID-owning handles.
Event fields are keyword-only and events carrying entity-reference data accept
typed mappings. UUID values use the standard-library `uuid.UUID` type.

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
standard-library `uuid.UUID` or the matching generated handle; references
carrying data accept a mapping with `target` and `data` fields.

Dynamic-attribute fields accept mappings. `None`, `bool`, `int`, `float`,
`str`, and nested mappings have natural conversions; `DynamicValue` selects an
exact numeric width or constructs a typed list, structure, structure list, or
recursive list with `DynamicValue.list`.
Mapping iteration order is retained for attributes and nested structure
members; sequence order is retained for lists.

FSM transitions consume the source-state handle and return a generated handle
for the target state. Type checkers and editor completion therefore expose only
legal transitions. Reusing a consumed handle raises `HandleConsumedError`;
transition sequence numbers are assigned by the instrumentation runtime and are
not Python parameters.
Calling `into_dynamic()` on any FSM handle produces an entity-specific
`DynamicFsmHandle`. It exposes every transition and raises
`InvalidFsmTransitionError` when a transition is invalid from its current
dynamic state.
Generated `try_into_<state>()` methods recover a typestate handle once the
current state is known. A successful conversion consumes the dynamic handle. A
mismatch raises `InvalidFsmStateError` without consuming it, so another state
can be checked. FSM entities may declare up to 255 states; generation rejects
256 or more.

`Context()` creates a no-op context. `Context.close()` prevents creation of new
observers; existing observers and handles retain their scoped telemetry runtime.
Exporter shutdown waits until the last observer or handle is released.
Once-cardinality events on non-FSM entities expose `<event>_emitted()`
predicates. Generated exceptions derive from `QuentError`. Nested options are
rejected during generation because Python cannot distinguish `None` from
`Some(None)`.

## Embedded collector server

Set `collector_server: true` in generator options to expose `start_collector()`
and the `Collector` handle. The generated bridge needs a dependency on
`quent-collector-python`, and its instrumentation model needs collector dispatch
generation (`collector_sink: true`) and a `collector` feature enabling
`quent-instrumentation/io-collector`. The collector output accepts the same
filesystem `ExporterOptions` as local contexts.

```python
import application_events as quent

with quent.start_collector(
    quent.ExporterOptions.ndjson("events"),
    bind_address="0.0.0.0:0",
    advertised_host="collector.example",
) as collector:
    address = collector.address
    # The application passes address to its collector clients.
```

The server uses Quent's existing gRPC collector protocol. `close()` stops
accepting connections and waits for active client streams to end and for their
exporter shutdown attempts to finish. Close client contexts and release all
their observers and handles before closing the server so sender-side buffers can
flush and client streams can end. `Context.close()` alone does not release
retained observers or handles. Exporter write failures are logged; `close()`
does not guarantee that data has been synced to disk.

`close()` can block indefinitely if a client leaves a stream open or an exporter
does not finish. `close(timeout=seconds)` bounds shutdown in seconds,
including draining and flushing. It raises `TimeoutError` and stops remaining
connections when the deadline expires; pending events may be lost and unfinished
cleanup may continue in the background. The collector remains closed
afterward, and repeated calls have no effect.
