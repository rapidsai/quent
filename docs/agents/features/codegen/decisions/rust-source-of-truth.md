# Decision: Rust as Source of Truth for Model Definitions

## Context

The model definition (FSMs, Resources, Entities and their relationships) must
live somewhere. The instrumentation API for each target language is generated
from this definition. The options considered were:

1. **Language-native definitions**: each consumer language defines the model in
   its own syntax (C++ macros/templates, Rust proc macros, etc.).
2. **External schema file**: a language-agnostic format (JSON, YAML, protobuf)
   that all code generators consume.
3. **Rust as single source of truth**: the model is defined in Rust, and code
   for other languages is generated from the Rust definitions.

## Decision

Rust is the single source of truth.

## Rationale

### Against language-native definitions per consumer language

Defining the model in each consumer language means keeping N definitions in
sync. Attribute structs (the data each FSM transition carries) must match across
languages. Any divergence is a silent bug in event serialization/deserialization.

The moment two languages need to share struct definitions, something must sit
between them. Having each language define its own model just distributes the sync
problem without solving it.

### Against an external schema file

An external schema (JSON, protobuf) solves the sync problem but creates a
different one: attribute structs live in the schema, not in the application
language. Application developers who iterate frequently on their model must
edit the schema, re-run codegen, then use the generated types. They cannot
reuse existing application types directly as transition attributes.

For Rust specifically, an external schema discards the language's type system.
Rust's traits and generics can enforce at compile time that resource usages
reference valid resources with correct capacity types. An external schema would
require a separate validation step that reimplements these checks.

### Why Rust

- The project is Rust-native: the analyzer, exporters, and core libraries are
  all Rust.
- Proc macros can validate the model at compile time (FSM reachability, exit
  convergence, state existence) and generate the instrumentation API in one step.
- Rust's trait system handles cross-model validation naturally. `Usage<T>`
  requires `T: Resource`, enforced by the compiler across crate boundaries.
- The codegen binary is Rust, so it can import the model crate directly and
  read metadata from trait impls without serialization.
- Application models compose via standard crate dependencies (`use domain_model::*`).

### Tradeoff

Non-Rust developers must read Rust to understand the model definition. In
practice the definitions are just structs with annotations, readable without
deep Rust knowledge. The application-facing API they interact with is in their
own language (generated C++ headers, etc.).
