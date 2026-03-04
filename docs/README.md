# Quent

Quent specifies a set of modeling concepts — Entities, Finite State Machines,
Resources, and their relationships — from which a statically typed
instrumentation API can be generated. Applications instrumented with this API
emit structured telemetry that can be stored, analyzed, and visualized.

The modeling concepts target resource-constrained, distributed, and highly
asynchronous applications. Examples of things that can be modeled as FSMs
include queries, operators, tasks, and data movement operations. Examples of
Resources include memory pools, thread pools, network links, and storage
devices. The current focus is on data processing / query engines in data
analytics, but the concepts are domain-agnostic and may be applied to other
domains in the future.

## How it works

A developer constructs a **model** of their application using the primitives
defined in this specification: FSMs to track entity lifecycles, Resources to
represent things with limited capacity, and Usages to tie the two together.

The model dictates the instrumentation: each FSM transition becomes a function
call that emits a timestamped event, and each Resource Usage is captured as
an attribute on those events. Because the model is statically typed, the
instrumentation API is type-safe — invalid transitions or mismatched capacity
types are caught at compile time.

Analysis tools consume the emitted events and, using only the structural
information from the model (states, transitions, capacities, resource
hierarchies), can automatically derive timelines, utilization graphs, and
DAG visualizations without application-specific code.

## What this repository provides

- **Specification** — this document, defining the modeling concepts and a
  domain-specific model for query engines.
- **Rust instrumentation crate** — a domain-agnostic library for emitting
  type-safe telemetry from a model, with minimal runtime overhead.
- **Exporters** — pluggable telemetry transports including gRPC (Protobuf),
  MessagePack, postcard, and NDJSON.
- **Analyzer** — a domain-agnostic library that reconstructs in-memory models
  from collected events, with traits for querying FSM states, resource usage,
  and entity relationships.
- **Web UI** — a React-based frontend for interactive visualization of query
  plans (DAGs), resource timelines, and operator statistics.
- **Simulator** — an example application that emits telemetry for a simulated
  query engine, useful for development and demonstration.

## Status

This project is experimental. The modeling concepts, APIs, and tooling are
heavily opinionated and subject to change.
