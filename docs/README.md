# Quent

Quent is a framework for instrumenting and analyzing applications. It provides
a set of modeling concepts (especially Finite State Machines, Resources, and
their relationships) from which a statically typed instrumentation API is
derived. Applications instrumented with this API emit structured telemetry
that can be stored, analyzed, and visualized.

The current focus is on data processing / query engines, but the concepts are
domain-agnostic and may be applied to other domains in the future.

This document specifies the modeling concepts and domain-specific models.
For development instructions and repository overview, see the
[root README](../README.md).
