# Resource

A Resource is an [Entity][entity] and an [FSM][finite-state-machine] with at
least one or more associated [Capacities][capacity].

Because a Resource is an [Entity][entity], it inherits the `id`, `type_name`,
and `instance_name` attributes.

Must have:

- `resource_group_id: uuid`: the ID of the [Resource Group][resource-group] that
  contains this Resource.

## Capacity

A Capacity of a [Resource][resource] is a named quantity that can be claimed
during some uninterrupted period of time via a [Usage][usage].

A Capacity may or may not have some non-negative integer maximum bound. Bounds
can be fixed or change during the lifetime of the [Resource][resource].

A Capacity is declared with a name and a type. A Capacity can be of two types:

- `Occupancy`: A [Usage][usage] value represents the amount of Resource Capacity
  held/occupied during a [Span][span].
- `Rate`: A [Usage][usage] value represents the total quantity processed over
  the [Span][span]. The rate is derived by dividing the value by the duration.

### Unbounded capacity

If the Capacity is unbounded, it has no declared maximum value.

### Bounded capacity

If the Capacity is bounded, its declaration includes an upper bound as an
unsigned integer.

## Unit Resource

A Unit Resource is a [Resource][resource] with one exceptional unnamed
dimensionless [Capacity][capacity] whose bounds are `[0, 1]`.

In other words, there can only be one [Usage][usage] of the entire
[Resource][resource] during some period of time.

If a [Resource][resource] does not declare any [Capacity][capacity], it is a
Unit Resource. This default exists as a convenience for resources where only
mutual exclusion matters (e.g. a single thread).

## Fixed-Bounds Resource

If a [Resource][resource] provides [Capacities][capacity] whose
bounds do not change during its lifetime, it is a Fixed-Bounds
[Resource][resource].

## FSM of Unit and Fixed-Bounds Resource

If the bounds of any [Capacity][capacity], including unbounded capacities,
never change during the lifetime of the [Resource][resource], the FSM is:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

For the [Fixed-Bounds Resource][fixed-bounds-resource], the
[Transition][transition] into the `operating` state must declare the
[Capacities][capacity].

## Dynamic-Bounds Resource

If a [Resource][resource] provides at least one bounded [Capacity][capacity]
whose bounds may change during its lifetime, it is a Dynamic-Bounds
[Resource][resource], for which the FSM is:

```text
⊙            -> initializing
initializing -> operating
operating    -> resizing
resizing     -> operating
operating    -> finalizing
finalizing   -> ⊗
```

The [Transition][transition] from the `initializing` state into the `operating`
state must declare all [Capacities][capacity]. The [Transition][transition]
from the `resizing` state into the `operating` state must only declare the
[Capacities][capacity] for which the bounds changed.

## Usage

A Usage represents a claim on a portion of [Capacities][capacity] of a
[Resource][resource]. Multiple Usages of the same Resource may coexist, as
long as the sum of their claimed capacities does not exceed the Resource's
bounds.

Must have:

- `resource_id: uuid`: the ID of the [Resource][resource] being used
- For each capacity of the resource: a value of the same unsigned integer type
  as the [Capacity][capacity], representing the amount of assigned capacity.

May have:

- An effective usage value per capacity: the usage minus any overhead.

Any Usage must be combined with [Timestamps][timestamp] such that
exactly one [Span][span] of time may be derived representing the duration of
the Usage.

## Notes

### Resources and Capacities during model construction

When constructing the model of an application, it may be that there is a desire
to obtain telemetry from some resource which isn't represented in the
implementation by some abstraction with an explicit capacity.

For example, when using libraries that perform computations on e.g. a CPU or a
GPU across many threads, it may not be clear or trivial to obtain knowledge on
how many threads are actively being used during the computation. If this
information is unavailable, but there is a desire to still capture that this
computation happened related to some resource, then it is recommended to
introduce a resource with an unbounded capacity in the model for which the
computation takes up some capacity of one. This way, FSMs that perform
this type of computation can be grouped under this "resource" in post-processing
and visualization. A model can only include things the implementation already
knows; if it doesn't have numbers on capacities, neither can its telemetry
produce them.

### Obtaining the Span of time of a Usage from an FSM

A Usage can be tied to one or more consecutive [FSM][finite-state-machine]
[States][state]. The [Transition][transition] into the first such
[State][state] must carry the Usage's [Attributes][attributes] (e.g.
`resource_id`, capacity values).

If a Usage spans multiple [States][state], those [States][state] must be
consecutive; there must be no intermediate [State][state] in which the claimed
[Capacity][capacity] is released.

The [Span][span] of the Usage is then derived from the entry
[Transition][transition] of the first [State][state] to the exit
[Transition][transition] of the last [State][state].

### Effective usage

Examples of effective usage include: sizes of tables in a memory resource
without padding, or goodput bytes of a message over a network interface
resource.

[attributes]: ./attributes.md
[capacity]: #capacity
[entity]: ./entity.md
[finite-state-machine]: ./fsm.md
[fixed-bounds-resource]: #fixed-bounds-resource
[resource]: #resource
[resource-group]: ./resource_group.md
[span]: ./time.md#span
[state]: ./fsm.md#state
[timestamp]: ./time.md#timestamp
[transition]: ./fsm.md#transition
[usage]: #usage
