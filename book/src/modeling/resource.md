# Resource

A Resource is an FSM with at least one or more associated [Capacity][capacity].

Resources have a type defined by a unique type name, such that multiple
instances of Resources with exactly the same abstract behavior can exist in a
model.

Must have:

- `type_name: string`: the name of the type of this Resource.
- `instance_name: string`: the name of the instance of this Resource.
- `resource_group_id: uuid`: the ID of the [Resource Group][resource-group] that
  contains this Resource.

## Capacity

A Capacity of a [Resource][resource] is a named quantity that can be
exclusively claimed during some uninterrupted period of time via a [Use][use].

A Capacity may or may not have some non-negative integer maximum bound. Bounds
can be fixed or change during the lifetime of the [Resource][resource].

A Capacity is declared as a set of [Attributes][attribute].

A Capacity can be of two types:

- An `Occupancy`-type capacity: A [Use][use] value represents the amount of
  Resource Capacity held/occupied during a Span.
- A `Rate`-type capacity: A [Use][use] value represents the total quantity
  processed over the span.

A Capacity type is declared through the following attribute:

- `capacity_<capacity_name>_is_rate: bool`: `true` if it is a `Rate` Capacity,
  `false` otherwise.

### Unbounded capacity

If the Capacity is unbounded, then it is declared by the following
[Attribute][attribute]:

- `capacity_<capacity_name>: option<T>` where:
  - T is any unsigned integer
  - The value of this attribute must be `none`.

### Bounded capacity

If the Capacity is
bounded, then it is declared by the following set of [Attributes][attribute]:

- `capacity_<capacity_name>: T` where:
  - T is an unsigned integer
  - The value of this attribute represents the upper bound of the Capacity.

## Unit Resource

A Unit Resource is a [Resource][resource] with one exceptional unnamed
dimensionless [Capacity][capacity] whose bounds are `[0, 1]`.

In other words, there can only be one [Use][use] of the entire
[Resource][resource] during some period of time.

If a [Resource][resource] does not declare any [Capacity][capacity], it is a
Unit Resource.

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

## Use

A Use represents an exclusive assignment of [Capacities][capacity] of a
[Resource][resource].

A Use must not outlive the `operating` (and `resizing`, if applicable) states of
the associated [Resource][resource].

Must have:

> TODO: figure out what the attributes of multiple resources usages should look
> like on a single event. Each use could simply be wrapped into a `struct`.

- `resource_id: uuid`: the ID of the [Resource][resource] being used

For each of the capacities of the resource, it must have:

- `used_<capacity name>: <capacity value type>`: the amount of assigned capacity
  of the [Resource][resource], where:
  - `<capacity name>` in the field name must be expanded to the specific
    capacity of the [Resource][resource] that is being used
  - `<capacity value type>` must be of the same unsigned integer type as the
    [Capacity][capacity].

May have:

- `used_<capacity name>_effective: <capacity value type>`: the amount usage of
  the [Resource][resource] 's capacity minus any overhead.

Any Use must be combined with [Timestamps][timestamp] such that
exactly one [Span][span] of time may be derived representing the duration of
the Use.

## Notes

### Resources and Capacities during model construction

When constructing the model of an engine, it may be that there is a desire to
obtain telemetry from some resource which isn't represented in the engine
implementation by some abstraction with an explicit capacity.

For example, when using libraries that perform computations on e.g. a CPU or a
GPU across many threads, it may not be clear or trivial to obtain knowledge on
how many threads are actively being used during the computation. If this
information is unavailable, but there is a desire to still capture that this
computation happened related to some resource, then it is recommended to
introduce a resource with an unbounded capacity in the model for which the
computation takes up some capacity of one. This way, spans or FSMs that perform
this type of computation can be grouped under this "resource" in post-processing
and visualization. Colloquially speaking, a model of an engine can only include
things the engine implementation already knows - if it doesn't have numbers on
capacities, neither can its telemetry produce them. Traditional profiling may
need to be applied to uncover the used capacity. Future work (as shown in the
overview figure of README.md in the repository sources) aims to provide the
means to correlate outcomes of traditional profiling tools to the telemetry of
engine models.

### Obtaining the Span of time of a Use from an FSM

One way of deriving the [Span][span] of time of the Use is by by encapsulating
it in one or multiple [FSM][finite-state-machine] [States][state]. In this
case, the required [Attributes][attributes] of the Use must be captured by the
[Transition][transition] into the [State][state] spanning the Use of a
[Resource][resource]. This must be done in at least one [State][state]. This
may be done in multiple [States][state] if the same Use outlives a single
[State][state]. In case the Use spans multiple states, the sequence of
[States][state] must not be interrupted by [States][state] in which the Use's
claim of the [Resource][resource] associated [Capcity][capacity] is released.

### `used_<capacity name>_effective`

Examples of `used_<x>_effective` include: sizes of tables in a memory resource
without padding or goodput bytes of a message over a network interface
resource.

[attribute]: #capacity
[attributes]: ./attributes.md
[capacity]: #capacity
[finite-state-machine]: ./fsm.md
[fixed-bounds-resource]: #fixed-bounds-resource
[resource]: #resource
[resource-group]: ./resource_group.md
[span]: ./time.md#span
[state]: ./fsm.md#state
[timestamp]: ./time.md#timestamp
[transition]: ./fsm.md#transition
[use]: #use
