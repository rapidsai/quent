# Quent (working title)

> 🚧 WORK IN PROGRESS 🚧
>
> This documentation is work in progress, incomplete, and probably contains
> various inconsistencies. If things that are already laid out imply
> consistency but are not consistent, please create an issue or reach out
> otherwise.

The goal of this document is to specify basic concepts used to model
applications from which a telemetry-emitting instrumentation API can be derived.
The concepts are designed such that it is easy to store, analyze and visualize
the telemetry. The concepts target resource-constrained, distributed, and highly
asynchronous applications.

This document furthermore provides a set of domain-specific models with common
building blocks to model applications within that domain. This allows for more
advanced domain-specific post-processing, analysis, and visualization of the
telemetry.

This project is currently in a PoC state, and focuses mainly on data processing
/ query engines in data analytics.
