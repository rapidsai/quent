# Simulator

The simulator (`experimental/vibe/simulator/`) is a simulated distributed query
engine used for rapid development and prototyping, especially the UI, without
requiring integration with a real engine. The source code also serves as a
reference for how to apply the modeling concepts.

The simulator has multiple [Workers][worker], each with a logical and physical
[Plan][plan]. Physical operators model compressed scans, GPU decoding,
partitioned and local joins, aggregations, filters, UDFs, sorting, a query-wide
limit, and output. During execution, a Task may allocate [Memory][memory], load
from or spill to storage, compute on a CPU thread, transfer data between host
and GPU memory, or send shuffled data over the Network.

The generated profile follows query-engine cardinality behavior: decoding and
one early many-to-many join may expand data, sorting preserves it, and later
selective joins, aggregations, filters, and projection-like UDFs reduce it. The
limit emits at most 42 rows across all workers.

The data-flow endpoint reports task and resident-byte series for physical
operators and rolls those series up to their logical parents, so the UI
animation is populated for both logical and physical plan views.

## Resources

### Worker-scoped

- Host Memory: [Memory][memory]
- Storage: [Memory][memory]
- StorageToHost: [Channel][channel] (Storage → Host Memory)
- HostToStorage: [Channel][channel] (Host Memory → Storage)
- GPU: [Resource Group][resource-group] containing GPU Memory, HostToGpu, and
  GpuToHost
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
| `loading`    | Computation | Allocation | Input transfer     |
| `computing`  | Computation | Allocation |                    |
| `sending`    | Computation |            | Transfer (Link)    |

State transitions:

```text
⊙          -> queueing
queueing   -> allocating
allocating -> loading -> computing
allocating -> computing
computing  -> spilling -> allocating
computing  -> sending -> queueing
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
Task -> Computation    -> Thread -> ThreadPool -> Worker -> Engine
Task -> Transfer       -> HostToGpu / GpuToHost -> GPU -> Worker -> Engine
Task -> Transfer       -> StorageToHost / HostToStorage -> Worker -> Engine
Task -> Transfer       -> Link -> Network -> Engine
Task -> Allocation     -> GPU Memory -> GPU -> Worker -> Engine
Task -> Allocation     -> Host Memory -> Worker -> Engine
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
