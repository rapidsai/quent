# Modeling Concepts

> 🚧 WORK IN PROGRESS 🚧
>
> TODO:
>
> Could haves:
>
> - logs-like specification
> - metrics-like specification
> - traces as FSMs

This section specifies basic concepts used to model applications. The intent of
these concepts is to model applications guided by application developers.

Models of applications constructed with these concepts dictate an
instrumentation API that is used to emit application telemetry.

By modeling applications with these concepts, it becomes easier to store,
analyze and visualize the telemetry emitted through such an instrumentation API.
A basic set of resource-oriented visualizations can be automatically provided
from just applying these concepts.

The basic concepts are:

- [Attributes][attributes]
- [Timestamp, Span, and Duration][time]
- [Entity][entity]
- [FSM][fsm], [State][state] and [Transition][transition]
- [Resource][resource] and [Use][use]
- [Resource Group][resource-group]

These modeling primitives are used to construct some common [Entity][entity]
types that exist solely for convenience of building domain- or
application-specific models. Application models are not required to use them.
These include:

- [Memory][memory]
- [Channel][channel]
- [Processor][processor]

## Conventions

Names of constructs that are defined by this specification are intentionally
capitalized, e.g. [Entity][entity], [Timestamp][timestamp],
[Resource][resource], etc.

[attributes]: ./attributes.md
[channel]: ./common/channel.md
[entity]: ./entity.md
[fsm]: ./fsm.md
[memory]: ./common/memory.md
[processor]: ./common/processor.md
[resource]: ./resource.md
[resource-group]: ./resource_group.md
[state]: ./fsm.md#state
[time]: ./time.md
[timestamp]: ./time.md#timestamp
[transition]: ./fsm.md#transition
[use]: ./resource.md#use
