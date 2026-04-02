# Remaining Work

Items identified during implementation that are not yet addressed.

## 1. Deferred field state handles

The `&mut` borrow handle for setting deferred attributes after an FSM
transition. The design (decision: deferred-attributes.md) specifies that
`transition()` returns a handle with typed setters, and the borrow prevents
calling `transition()` again until the handle is dropped. The FSM handle
struct has the `emit_deferred()` method but the state handle type and its
integration with `cxx_build` (for C++) are not yet implemented.

## 2. YAML model input

Parsing YAML/JSON into `ModelBuilder` for teams that don't want to define
models in Rust. The `ModelBuilder` types are plain data structs — adding
`Serialize`/`Deserialize` derives enables this. A `quent-codegen` CLI
binary would read the YAML and call `emit_cxx()`. Not yet implemented.

## 3. Codegen CLI binary

A standalone `quent-codegen` command-line tool that reads a model definition
(YAML or Rust crate) and emits target-language code. Currently `quent-codegen`
is a library called from `build.rs`. The CLI would wrap this for use in
CMake `add_custom_command` or standalone invocations.

## 4. CMake module (`quent_add_model`)

A user-facing CMake function that wraps the codegen CLI + Corrosion setup.
Currently the C++ example has a manual `CMakeLists.txt`. The module would be
installed alongside `quent-codegen` and found via `find_package`.

See decision: cpp-cmake-integration.md.

## 5. Entity handles

Per-instance handles for entities (like FSM handles) with at-most-once event
enforcement. Currently entities use the observer pattern (stateless, entity ID
passed per call). Entity handles would own the ID and track which events have
been sent, preventing duplicate emissions. The observer pattern remains for
the CXX bridge (C++ side), but the Rust API could offer handles.

## 6. Model metadata refinement

The `State` derive macro uses `ValueType::String` as a placeholder for field
types it doesn't recognize (anything other than fields annotated with
`#[usage]`, `#[deferred]`, `#[capacity]`, `#[instance_name]`). Proper type
resolution would map `Uuid` → `ValueType::Uuid`, `u64` → `ValueType::U64`,
`Ref<T>` → `ValueType::Ref(name)`, etc. The `Entity` derive has the same
gap for event struct fields — it records event names but not their field
schemas. The codegen `build.rs` currently patches metadata manually to
compensate.

## ~~7. SimulatorEvent via define_model!~~ (resolved)

Resolved: `define_model!` now supports an `extra { Variant: Type }` section
for non-model event types. `SimulatorEvent` is generated via `define_model!`.

## ~~8. Resource observer generation~~ (resolved)

Resolved: resources migrated to model-generated events via
`#[derive(Resource)]` and `#[derive(ResizableResource)]`.

## ~~9. Transitive From impls~~ (resolved)

Resolved: the `extra {}` section in `define_model!` and model composition
eliminate the need for manual transitive `From` impls.
