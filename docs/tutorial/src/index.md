# Quent Tutorial

Welcome to the Quent tutorial!

## What is Quent?

Quent is a framework that helps build application-specific profiling tools in
order to reduce the time to arrive to a conclusion about how an application is
performing.

Quent is typically used by developers, but the things you can build with it can
also be leveraged by users.

This tutorial mainly focuses on how software developers can integrate it into
their system.

## How does Quent work?

<div class="architecture-overview" role="img" aria-label="Quent architecture">
{{#include ../../figures/overview.svg}}
</div>

At a very high level, using Quent work as follows:

1. **Model the application events:** Write an _Application Event Schema_ that
   expresses what events your application emits and what the related
   semantics are.
2. **Generate typed libraries.** From the schema, a code generation step
   produces a statically-typed _application-specific instrumentation library_
   and an (WIP) analysis library.
3. **Capture events.** The application emits events through the generated
   instrumentation API at run-time. The instrumentation library exports the
   events through any of the provided exporters into their associated storage.
4. **Analyze behavior.** An application-specific analysis service imports the
   events later and digests them into useful insights, e.g. to feed a UI + human
   or an agent in the loop of a performance optimization effort.
5. **Semantic modules.** At any level of the stack, _semantic modules_ can
   influence how things work. They can contribute to generating a more robust
   instrumentation API for certain types of events, add easy-to-use analysis
   functionality to an analysis library, or add UI components and more. They
   represent well-curated, opt-in vertical slices of Quent's entire stack.

## What are the key features of Quent?

- **Instrumentation-based.** You explicitly place instrumentation in your code.
- Schema-driven. You define your application's entities, events, and attributes
  once, then generate instrumentation APIs from that model.
- **Application-specific.** Your events can describe the abstractions and
  behavior that matter to your application instead of fitting a fixed set of
  general-purpose event semantics.
- **Statically typed, end-to-end.** Generated instrumentation APIs, the event
  export path, and analysis all rely on the schema. This way compilers can catch
  mismatches early and avoid unnecessary runtime work.
- **Composable.** Semantic modules add curated vertical slices to your
  application event model. These modules can affect every layer, including code
  generation, instrumentation API, analysis support, and visualizations.
- **Cross-language.** Quent generates Rust instrumentation APIs, with
  experimental C++ and Python bindings for instrumenting mixed-language systems.

## Why use Quent?

- You want to build a profiling tool that speaks in the same abstractions as
  your application.
- You think answer questions about performance is best done through
  domain-specific or application-specific relationships found in your event
  data.
- You need more flexibility than general-purpose logs, metrics, traces, or
  call-stack profiles provide on their own.
- You often dig through general-purpose telemetry/profiling data in which it
  takes you a lot of time to properly correlate and understand everything, and
  you're looking for a rigid solution to do more of this automatically.
- You want to ensure event producers and consumers stay in sync as the
  application evolves.
- You are willing to add explicit instrumentation in exchange for precise,
  structured event data.
- You want to build on Quent's
  [in-tree interactive user interface components](https://rapidsai.github.io/quent/simulator/#/profile/engine/01a07b4c-86ab-7971-97c1-24879c41910e/query/01a07b4c-86ab-7971-97c1-28ffb10dde0d/timeline)
  🤩.

## Why not use Quent?

- You cannot or do not want to modify the source code of your application.
- Existing tools that help produce and analyze logs, metrics, traces, or
  profiles already answer the questions you care about quickly enough.
- A few log statements and analysis scripts provide all the structure you need.
- You need a mature, stable profiling platform today. Quent is currently an
  experimental alpha-stage project.

## What this tutorial covers

In this tutorial, we will explore how to model application behavior by defining
an Application Event Schema through the use of Quent's YAML-based DSL.

You will learn how to:

- Define entities, events, and typed attributes.
- Leverage semantic modules, including those to define
  FSMs, special types of references between entities, and resources.
- Use instrumentation APIs in Rust, C++, and/or Python.

Every lesson shows the complete YAML model beside the generated instrumentation
API. Instrumentation examples are available in Rust, C++, and Python. Use the
tabs above each example to select a language.

To keep the lessons focused, the displayed snippets omit license headers and
language-specific build wiring. The complete buildable sources remain available
on GitHub in the [Rust examples], [C++ examples], and [Python examples] and are
linked for each lesson at the bottom as well.

<div class="badger-note">
  <div class="badger-mark" role="img" aria-label="Quent honey badger">
{{#include ../../../ui/public/logo.svg}}
  </div>
  <div>
    <p>The key takeaway for each newly introduced concept is placed in this kind of box.</p>
  </div>
</div>

Quent has an experimental [Schema
Explorer](https://rapidsai.github.io/quent/schema/) to inspect an Application
Event Schema interactively.

Use the arrow on the right or the sidebar to begin.

[Rust examples]: https://github.com/rapidsai/quent/tree/main/crates/yaml/examples
[C++ examples]: https://github.com/rapidsai/quent/tree/main/experimental/vibe/codegen/cpp/example/tutorial
[Python examples]: https://github.com/rapidsai/quent/tree/main/experimental/vibe/codegen/python/example/tutorial
