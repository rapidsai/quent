# Channel

A Channel is a [Resource][resource] responsible for transferring data between
two [Entities][entity].

Must have:

- `capacity_bytes: option<T>`: where T is some unsigned integer.
- `source_id: uuid`: the ID of the [Entity][entity] the Channel receives from.
- `target_id: uuid`: the ID of the [Entity][entity] the Channel sends to.

> TODO: how to specify we're dealing with a bandwidth type of capacity, if
> known? It can simply be unbounded for now. Perhaps its best to specify
> throughput-type capacities as a separate thing.
>
> TODO: for now a channel is unidirectional and bidi can be constructed as a
> resource group for example, but perhaps a more elaborate common model for
> channels could be concieved

## Transfer

A Transfer is a [Usage][usage] of a Channel.

Must have:

- `used_bytes: u64`: the number of bytes sent over the channel

[entity]: ../entity.md
[resource]: ../resource.md
[usage]: ../resource.md#use
