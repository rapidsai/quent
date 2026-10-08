# Schema CXX generator

`quent-schema-codegen-cpp` emits internal CXX bridges plus one public
`quent.hpp` façade. The façade exposes scoped, copyable observer pointers from
`Context`, entity-typed IDs and handles, shared schema records, standard C++
containers and options, and dynamic-attribute builder methods.

```rust
let schema = quent_yaml::parse_from_file("model.yaml")?.schema;
let options = quent_schema_codegen_cpp::Options {
    crate_name: env!("CARGO_PKG_NAME").to_owned(),
    instrumentation_path: "application_instrumentation::model".to_owned(),
    exporters: quent_schema_codegen_cpp::Exporters::all(),
    ..Default::default()
};
let files = quent_schema_codegen_cpp::emit(&schema, &options)?;
let bridges = quent_schema_codegen_cpp::write_bridge_files(&files, &options)?;
let mut build = cxx_build::bridges(bridges);
let include_dir = quent_schema_codegen_cpp::stage_cxx_headers(&options)?;
build.std("c++20").compile("application_bridge");
println!("cargo:include={}", include_dir.display());
```

Include the generated sibling modules once in the bridge crate. The containing
module name is not fixed:

```rust
mod application_bridge {
    include!(concat!(env!("OUT_DIR"), "/bridge_mod.rs"));
}
```

Generated Rust and C++ files are written below `OUT_DIR`. Public headers are
staged under `OUT_DIR/cxxbridge/include`; export or install that directory for
the target C++ build. `stage_cxx_headers` must run after `cxx_build::bridges`
and before `compile`.

Client code includes only the façade:

```cpp
#include "application-bridge/gen/quent.hpp"

auto context = quent::Context::ndjson("./events");
auto request_observer = context.request_observer();
quent::Handle<quent::Request> request = request_observer->handle();
```

The public API uses `std::shared_ptr`, `std::optional`, `std::vector`, and
`std::string`. Targeted entity references use generated entity-specific ID
types; untargeted references use UUIDs. References carrying data use a shared
schema-derived struct in `quent::refs`; an existing target prefix in the data
record name is not repeated. CXX-specific representations remain under
`quent::detail`.

`DynamicAttributes::add` is overloaded for every numeric scalar and list
variant, plus strings, booleans, structures, and lists of structures. Pass
`nullptr` to add a null value. `DynamicList` factories construct recursive
lists accepted by the same `add` overload.
Attributes and nested structure members retain insertion order; list elements
retain their input order.

Plain entities specialize `Handle<Entity>` with their event methods. FSMs
specialize `FsmHandle<Entity>` for a new instance and
`FsmHandle<Entity, entity_state::State>` for every state. Their `&&`-qualified
transition methods consume the current handle and return the target-state
specialization, so transitions that are not present in the schema do not
compile. Transition sequence numbers are assigned by the instrumentation runtime
and are not part of the C++ payload types. Consuming any FSM handle with
`into_dynamic()` produces a `DynamicFsmHandle<Entity>`. It exposes every
transition and reports transitions that are invalid from its current dynamic
state. Once the current state is known, `try_into<State>() &&` returns an
`std::optional<FsmHandle<Entity, State>>`. A state mismatch returns an empty
optional without moving from the dynamic handle, so the same handle can be
checked again. A successful conversion consumes it. FSM entities may declare up
to 255 states; generation rejects 256 or more.

Observers and handles retain their scoped telemetry runtime independently of
`Context`. Destroying a context prevents obtaining new observers, but exporter
shutdown waits until the last observer or handle is destroyed. Once-cardinality
events on non-FSM entities expose `<event>_emitted()` predicates on their
handles.

## Optional NVTX capture

NVTX support is disabled by default. To generate an NVTX-capable bridge, set
the generation option:

```rust
let options = quent_schema_codegen_cpp::Options {
    nvtx: quent_schema_codegen_cpp::NvtxSupport::Enabled,
    // Set crate_name, instrumentation_path, and exporters for your application.
    ..Default::default()
};
```

The consuming bridge crate then needs `quent-nvtx-bridge`, `quent-nvtx-events`,
and `nvtx-injection` from the same Quent revision as its other Quent
dependencies. The generator itself does not depend on these crates. The
current `nvtx-injection` crate supports Linux 64-bit targets only, so
NVTX-enabled consumer builds require that platform. Enable `nvtx-injection`'s
`static-injection` feature for in-process capture.

Generation support does not start capture. Enable capture explicitly when
constructing an active exporter:

```cpp
auto context = quent::Context::ndjson(
    "./events", quent::NvtxCapture::Enabled);
// Run annotated work while context is alive.
// Events emitted during or after context destruction may not be exported.
```

`msgpack`, `postcard`, and `collector` accept the same optional capture
parameter. It defaults to `NvtxCapture::Disabled`; `Context::none()` never
captures, including the raw bridge's no-op exporter branch. When generation
support is disabled, the existing factory signatures are unchanged.

Collector capture also requires the receiving collector to route the
`NvtxEvent` stream. A schema-only generated collector does not provide that
route automatically; enabling capture on the sender does not add receiver
support.

The context owns a `quent_nvtx_bridge::Capture` using its context ID and
exporter configuration. Capture queues owned injection records and converts
them to `quent_nvtx_events::NvtxEvent` on a worker that owns the observer.
NVTX remains a separate event stream; no schema entities need to be added.
Dropping the context closes and drains the capture queue, joins the worker,
and waits for the observer to flush before releasing the schema context.
Moving the context preserves this ownership.

Hook registration is one-shot per process. Constructing another capture-enabled
context returns an error without ending the first context's capture. Destroying
the capturing context does not permit capture to restart. Disabled contexts do
not claim the hook. Events emitted during context destruction may be discarded;
events emitted after destruction are not exported. Stop and join NVTX producers
before destroying the context if complete capture is required. Do not destroy
the context on its exporter or Quent runtime worker: joining the capture worker
there can deadlock.

The readme example bridge offers a Cargo `nvtx` feature to demonstrate opt-in
generation:

```sh
cargo build --manifest-path experimental/vibe/codegen/Cargo.toml \
    -p quent-demo-cpp-bridge --features nvtx
```

Its existing example calls still default to capture disabled. The enabled
compiled regression fixture exercises runtime selection and capture separately.
