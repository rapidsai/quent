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
`FsmHandle<Entity, entity_state::State>` for every state. Their
`&&`-qualified transition methods consume the current handle and return the
target-state specialization, so transitions that are not present in the
schema do not compile. Transition sequence numbers are assigned by the
instrumentation runtime and are not part of the C++ payload types.

Observers and handles retain their scoped telemetry runtime independently of
`Context`. Destroying a context prevents obtaining new observers, but exporter
shutdown waits until the last observer or handle is destroyed. Once-cardinality
events on non-FSM entities expose `<event>_emitted()` predicates on their
handles.

Schemas with an `nvtx: true` process expose source capture on `Context` while
keeping the generated NVTX stream private. The Linux 64-bit integration test
drives the public C++ process API and real NVTX calls through the generated
NDJSON exporter:

```sh
pixi run cargo test \
  --manifest-path experimental/vibe/codegen/Cargo.toml \
  -p quent-codegen-cpp-nvtx-test
```
