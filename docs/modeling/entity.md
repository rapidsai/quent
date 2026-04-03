# Entity

An Entity is any discrete run-time construct that can be traced, measured, or in
some other way produce telemetry that is potentially useful to understand the
performance characteristics of an application.

Must have:

- `id: uuid`
- `type_name: string`: the name of the type of this Entity.
- `instance_name: string`: the name of this specific instance of the Entity.

Notes:

- Examples of things that can be modeled as Entity include objects,
  functions, threads, events, data movement abstractions over a PCIe-based
  host-to-device/device-to-host interface of a GPU, etc.

Rationale:

- Using UUIDs practically prevents the need to synchronize between various
  producers of telemetry to produce unique identifiers, especially when they are
  originating from a distributed system.

## Implementation-specific notes on entities

Implementations are recommended to use _UUIDv7_ as `id`. UUIDv7 includes a Unix
timestamp at approximately millisecond granularity, which is useful to build
indexes for fast analysis and search in time ranges.
