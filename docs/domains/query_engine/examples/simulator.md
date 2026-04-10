# Simulator

The simulator (`examples/simulator/`) is a simulated distributed query engine
used for rapid development and prototyping, especially the UI, without
requiring integration with a real engine. The source code also serves as a
reference for how to apply the modeling concepts.

The simulator has multiple [Workers][worker], each with a logical and physical
[Plan][plan]. For each physical [Operator][operator], Tasks are enqueued to a
ThreadPool. During execution, a Task may allocate [Memory][memory], spill to
or load from a Filesystem, compute results, or send data to another Worker
over the Network.

## Resources

### Worker-scoped

- Memory: [Memory][memory]
- Filesystem: [Memory][memory]
- FsToMem: [Channel][channel] (Filesystem → Memory)
- MemToFs: [Channel][channel] (Memory → Filesystem)
- Thread: [Processor][processor]
- ThreadPool: [Resource Group][resource-group] of Threads

### Engine-scoped

- Link: [Channel][channel] between the Memory of two Workers
- Network: [Resource Group][resource-group] of Links

## Task

A Task is an FSM that performs work on behalf of an [Operator][operator]
(referenced via `operator_id`).

Resource usage per state:

| State        | Thread      | Memory     | Channel            |
| ------------ | ----------- | ---------- | ------------------ |
| `queueing`   |             |            |                    |
| `allocating` | Computation |            |                    |
| `spilling`   | Computation |            | Transfer (MemToFs) |
| `loading`    | Computation | Allocation | Transfer (FsToMem) |
| `computing`  | Computation | Allocation |                    |
| `sending`    | Computation | Allocation | Transfer (Link)    |

State transitions:

```text
⊙          -> queueing
queueing   -> allocating
allocating -> spilling -> allocating
allocating -> loading -> computing
allocating -> computing
computing  -> sending -> ⊗
computing  -> ⊗
```

## Entity and resource relations

Every Task traces back to an [Engine][engine] through entity references:

```text
Task -> Operator -> Plan (physical) -> Plan (logical) -> Query -> Query Group -> Engine
```

Every [Resource][resource] [Usage][usage] traces back to an Engine through
resource groups:

```text
Task -> Computation -> Thread -> ThreadPool -> Worker -> Engine
Task -> Transfer    -> FsToMem / MemToFs  -> Worker -> Engine
Task -> Transfer    -> Link -> Network -> Engine
Task -> Allocation  -> Memory -> Worker -> Engine
```

## Example analyses

Given a query, an analysis tool can derive various things from this model, e.g.:

- A DAG visualization of logical and physical plans with per-port row/byte
  counts and per-operator time breakdowns
- A timeline of Tasks on ThreadPool Threads, colored by state
- A Memory usage timeline derived from Allocations
- Network throughput per operator, colored by bytes transferred

[channel]: ../../../modeling/common/channel.md
[engine]: ../README.md#engine
[memory]: ../../../modeling/common/memory.md
[operator]: ../README.md#operator
[plan]: ../README.md#plan
[processor]: ../../../modeling/common/processor.md
[resource]: ../../../modeling/resource.md
[resource-group]: ../../../modeling/resource_group.md
[usage]: ../../../modeling/resource.md#usage
[worker]: ../README.md#worker
