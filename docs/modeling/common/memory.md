# Memory

A (fixed- or dynamically) bounded [Resource][resource] with an
`Occupancy`-type [Capacity][capacity] of bytes of type `u64`.

Any [Transition][transition] into the `operating` state must have:

- `capacity_bytes: u64`: the maximum amount of bytes that can be stored

## Allocation

A [Usage][usage] of a Memory [Resource][resource].

Must have:

`used_bytes: u64`: the number of bytes used from the Memory.

## Notes on models using Allocation

Allocations are not necessarily meant to capture single allocations (e.g. one
`malloc`). For example, in a columnar query engine working with Arrow, each
underlying Arrow buffer would be a single `malloc`, yet in the model, an
Allocation can be tied to an entire worker-local "Table", capturing the sum of
all Arrow data and metadata buffer capacities. Note that here the effective
part of the Allocation is the bytes of useful information within these buffers,
but the true use is the capacity of the buffers (which includes unused bytes and
padding). Choosing what an Allocation represents, as with all other model
constructs, will be a trade-off between telemetry detail, run-time overhead and
storage.

[capacity]: ../resource.md#capacity
[resource]: ../resource.md
[transition]: ../fsm.md#transition
[usage]: ../resource.md#usage
