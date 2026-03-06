# Quent

Quent is a framework for instrumenting and analyzing applications. It provides
a set of modeling concepts (especially Finite State Machines, Resources, and
their relationships) from which a statically typed instrumentation API is
derived. Applications instrumented with this API emit structured telemetry
that can be stored, analyzed, and visualized.

The current focus is on data processing / query engines, but the concepts are
domain-agnostic and may be applied to other domains in the future.

## How

A developer constructs a **model** of their application: FSMs to track entity
lifecycles, Resources to represent things with limited capacity, and Usages to
tie the two together. The model produces a type-safe instrumentation API.

Analysis tools consume the emitted events and, using the structural information
from the model, automatically derive timelines and utilization graphs. For
query engines, this includes plan DAG visualizations and per-operator
breakdowns. Any application-specific analysis is easier to build on top of the
structured model the framework provides.

## Provided by this repository

- **Specification**: this document, defining the modeling concepts and a
  domain-specific model for query engines.
- Rust implementations of:
  - **Instrumentation**: a domain-agnostic library for emitting
    type-safe telemetry from a model.
  - **Exporters**: pluggable telemetry transports.
  - **Analyzers**: domain-agnostic and domain-specific libraries that
    reconstruct in-memory models from collected events, with traits for
    querying FSM states, resource usage, and entity relationships.
  - **Web UI**: a React-based frontend for interactive visualization of query
    plans (DAGs), resource timelines, and operator statistics.
  - **Simulator**: an example application that emits telemetry for a simulated
    query engine, useful for development and demonstration.

The core of the project is the modeling approach: Entities, FSMs, Resources,
Capacities, and Usages, and the logic that connects them (resource utilization
tracking, hierarchical aggregation, and model reconstruction from events).
Everything else (storage, transport, instrumentation, and visualization) is an
opinionated but replaceable implementation based on the modeling approach.

## Status

This project is experimental and under heavy development. The modeling concepts,
APIs, and tooling are heavily opinionated and subject to change. There are no
official releases yet.
