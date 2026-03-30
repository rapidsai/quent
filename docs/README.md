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

## What Quent is not

- **Not a sampling-based profiler.** Quent does not sample stack traces,
  hardware counters, or CPU cycles. It captures the events you explicitly
  instrument.
- **Not a replacement for NSight Systems, perf, or similar tools.** Those
  tools give you low-level, system-wide visibility. Quent gives you
  high-level, application-specific visibility based on your domain model.

## Documentation overview

- [Telemetry](./telemetry.md) — event model and rationale
- [Modeling Concepts](./modeling/README.md) — the core primitives (Entity,
  FSM, Resource, etc.)
- [Domain-Specific Models](./domains/README.md) — concrete models for
  specific application domains

The current focus is on data processing / query engines, but the modeling
concepts are domain-agnostic and may be applied to other domains.

For development instructions and repository overview, see the
[root README](../README.md).

[fsm]: ./modeling/fsm.md
[resource]: ./modeling/resource.md
