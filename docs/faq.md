# Frequently Asked Questions

## Why not use a sampling-based profiler?

Sampling profilers (e.g. `perf`, Intel VTune) periodically capture stack
traces, hardware counters, or CPU cycles. They give broad, low-overhead
coverage but can only tell you *where* time is spent, not *why*. They have no
knowledge of your application's domain concepts.

Quent is instrumentation-based: you explicitly emit events at points that
matter to your application model. This means Quent can reconstruct causal
relationships (which task, on behalf of which operator, caused this memory
allocation) that sampling cannot infer.

The two approaches are complementary. A sampling profiler can reveal
unexpected hotspots; Quent can explain what your application was doing and
why.

## Why not use NSight Systems?

NSight Systems is a system-wide performance analysis tool that combines
OS-level tracing (CPU scheduling, GPU activity, memory transfers) with
NVIDIA SDK instrumentation (CUDA, NVTX). It excels at low-level visibility
across the hardware/software stack.

Quent operates at a different level of abstraction. It models
application-specific concepts (queries, plans, operators, tasks) and their
resource usage. NSight Systems can show you that a CUDA kernel ran for 2 ms;
Quent can show you which operator of which query plan scheduled that work,
how much memory it allocated, and what it was waiting on.

As with sampling profilers, the two tools are complementary.

## Why not use OpenTelemetry?

OpenTelemetry (OTel) is a widely adopted and valuable observability
framework. However, Quent has specific requirements that benefit from a
different approach:

- **Partial model recovery.** OTel tracing implementations typically export
  spans only after they are closed. If a program crashes or a query fails
  mid-flight, in-progress spans are lost. FSM transitions are emitted
  individually as they occur. If a failure happens, the model can be partially
  reconstructed from whatever transitions were already emitted.
- **Static typing.** OTel's data model relies on string-keyed attribute bags
  with runtime type information. Quent uses statically typed, schema-driven
  events. This enables compile-time guarantees on the instrumentation API and
  avoids the overhead of runtime type dispatch in both the instrumentation
  and analysis paths.
- **Full control over transport and encoding.** Quent's concepts are defined
  independently of any particular telemetry framework. OTel Logs, for
  instance, could serve as an underlying layer to carry FSM transition events,
  but that is an implementation choice, not a requirement. By not coupling to
  a specific data model, the project is free to choose encodings and
  transports (e.g. Protobuf over gRPC, MessagePack, postcard, Arrow IPC) that
  best fit the performance and deployment constraints of the target
  application.

## Why FSMs instead of tracing spans?

Tracing spans require explicit begin/end pairs and are organized into trees
via context propagation. FSMs offer two advantages over this model:

1. **Implicit spans of time.** Each FSM state spans from its entry transition
   to the next transition. Developers instrument only transitions; the
   durations of states are derived automatically without requiring explicit
   begin/end pairs.
2. **Structural constraints.** The declared set of states and allowed
   transitions forms a schema. Invalid transitions can be detected, and
   analysis tools can reason about what states an entity can be in without
   application-specific code.

FSM entities are flat, independent state machines that relate to each other
through explicit attributes (e.g. a Task FSM references an Operator by ID).
This avoids the need for implicit context propagation and lets developers
define the relationships that matter to them directly.

## Why explicit Resource modeling?

Traditional metrics (e.g. "memory usage at time T") are snapshots: they
capture a value but not the events that produced it. Because resource
utilization in Quent is derived from FSM transitions and Usage events, any
aggregate value (e.g. total bytes allocated) can be traced back to the
individual state transitions and entity lifecycles that contributed to it.

Resources with declared capacities enable analysis tools to automatically
compute utilization, detect saturation, and visualize resource pressure over
time, without application-specific logic.

This design targets systems that must balance resource utilization across
complex distributed topologies with heterogeneous hardware (CPUs,
accelerators, networked storage). The model captures how stateful things use
resources and where run-time trade-offs are made (e.g. spilling to disk when
memory is saturated, or re-scheduling work across nodes).
