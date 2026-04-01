# Modeling Concepts

Quent models application performance through a small set of primitives. You
compose these to describe the stateful things and resources in your
application; the resulting model drives both the instrumentation API and the
analysis tooling.

The basic concepts are:

- [Attributes][attributes]
- [Timestamp, Span, and Duration][time]
- [Entity][entity]
- [FSM][fsm], [State][state] and [Transition][transition]
- [Resource][resource] and [Usage][usage]
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

Fields listed under **"must have"** must have a non-null value. Fields listed
under **"may have"** are optional: the field exists but its value may be null.

### Mutual exclusion

Where a value could be one of several mutually exclusive alternatives, each
alternative is listed in a **"mutually exclusive"** section. Exactly one must
be non-null; the rest must be null. This avoids sum types, which are not
well-supported across all transport and serialization formats.

In all cases, optional means the value may be null — the field itself is
always present in the schema.

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
[usage]: ./resource.md#usage
