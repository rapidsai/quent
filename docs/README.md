# Quent

Quent is an instrumentation-based framework for modeling and analyzing
application performance. You define a model of your application's stateful
things and resources, instrument your code against that model, and Quent's
tooling reconstructs, analyzes, and visualizes application behavior from the
emitted telemetry.

## What Quent is

- **Instrumentation-based.** You add instrumentation calls to your application
  code. Quent does not observe your application from the outside.
- **Model-driven.** You define your application's performance-relevant
  structure up front: what things have state, what resources exist, and how
  they relate to each other. This model drives both the instrumentation API
  and the analysis.
- **Stateful things are [FSMs][fsm].** Anything whose lifecycle you want to
  track becomes a Finite State Machine: a piece of data moving through
  processing stages, a task transitioning between queued/running/blocked, or
  even a simple function call modeled as enter/exit. FSM transitions are
  events; the durations of states are derived automatically.
- **[Resources][resource] are capacity-bound things.** A Resource is something
  that can be occupied or saturated: a memory arena with a byte capacity, a
  thread pool with a fixed number of threads, a network link with bandwidth.
  Resources have declared capacities, and utilization is computed from the
  FSM transitions and usage events that affect them.
- **A set of building blocks.** Quent provides reusable components for
  instrumentation, event transport, analysis, and visualization. You define
  your application's model, wire the components together, and write the
  application-specific glue that connects them. The
  [simulator example](./domains/query_engine/examples/simulator.md) shows
  what a complete pipeline looks like end-to-end.

## What Quent is not

- **Not a sampling-based profiler or a replacement for NSight Systems /
  perf.** Quent adds an application-specific semantic layer on top and aims
  to complement these tools.
- **Not an OpenTelemetry implementation.** Quent shares the idea of
  structured observability but uses a statically typed, schema-driven model
  rather than string-keyed attribute bags.

See the [FAQ](./faq.md) for detailed comparisons.

- **Not a turnkey solution.** Quent does not ship a single library that all
  applications link against or a single tool that analyzes any application.

## Documentation overview

- [Event Model](./event_model.md) — events, FSM events, and design rationale
- [Modeling Concepts](./modeling/README.md) — the core primitives (Entity,
  FSM, Resource, etc.)
- [Domain-Specific Models](./domains/README.md) — concrete models for
  specific application domains
- [FAQ](./faq.md) — common questions and design comparisons

The current focus is on query engines, but the modeling
concepts are domain-agnostic and may be applied to other domains.

For development instructions and repository overview, see the
[root README](../README.md).

[fsm]: ./modeling/fsm.md
[resource]: ./modeling/resource.md
