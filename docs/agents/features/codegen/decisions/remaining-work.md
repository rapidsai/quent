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

## 7. SimulatorEvent via define_model!

`SimulatorEvent` is hand-written because it includes `ResourceEvent` and
`TraceEvent` from `quent-events`, which are not model components and don't
implement `HasEventType`. The `define_model!` macro only handles model
components. Options:

- Extend `define_model!` with an `extra { Variant: Type }` section for
  non-model event types that get included in the enum and `From` impls
  but don't contribute to the `Model<T>` type alias.
- Implement `HasEventType` for `ResourceEvent` and `TraceEvent` so they
  can be treated as model components.
- Keep `SimulatorEvent` hand-written (acceptable if the boilerplate is small).

## 8. Resource observer generation

Memory, Processor, and Channel resource observers are still provided by
`quent-instrumentation` (hand-written generics), not model-generated.
Since resources are stdlib FSMs, their observers could be generated from
the FSM definitions. This would make the resource observer API consistent
with entity and FSM observers.

## 9. Transitive From impls

The simulator events crate manually implements `From<EntityEvent> for
SimulatorEvent` via `impl_from_via_qe!` macro for each query engine entity
event type. This boilerplate could be eliminated if `define_model!`
generated transitive `From` impls when composing nested models, or if the
`extra {}` section (item 7) handled it.
