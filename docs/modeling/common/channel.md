# Channel

A Channel is a [Resource][resource] responsible for transferring data between
two [Entities][entity].

A Channel has a `Rate`-type [Capacity][capacity] of bytes.

Must have:

- `source_id: uuid`: the ID of the [Entity][entity] the Channel receives from.
- `target_id: uuid`: the ID of the [Entity][entity] the Channel sends to.

May have:

- `capacity_bytes: u64`: a `Rate`-type capacity. Absent if unbounded.

A Channel is unidirectional. Bidirectional communication can be modeled as two
Channels (or as a [Resource Group][resource-group] containing two Channels).

## Transfer

A Transfer is a [Usage][usage] of a Channel.

Must have:

- `used_bytes: u64`: the number of bytes sent over the channel

[capacity]: ../resource.md#capacity
[entity]: ../entity.md
[resource]: ../resource.md
[resource-group]: ../resource_group.md
[usage]: ../resource.md#usage
