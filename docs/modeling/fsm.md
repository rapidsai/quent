# Finite State Machine

A Finite State Machine (FSM) is an [Entity][entity] with a set of
[States][state] and a set of allowed [Transitions][transition] between
[States][state].

## State

Every State must have a name representable as a `string`.
Every State name is unique within the scope of an FSM.

An FSM must have at least two States, including the [Exit][exit]
[State][state]. The first [Transition][transition] event of an FSM defines
its initial State.

### Exit

The Exit [State][state] is a special reserved [State][state] into which a
transition means the [Entity][entity] no longer exists. Its name is `exit`.

Every FSM must have the Exit [State][state].

Every FSM must ultimately reach the Exit [State][state].

No transition may occur out of the Exit [State][state].

## Transition

A Transition is a [Timestamp][timestamp] upon which the FSM entered a new
[State][state].

A Transition may be accompanied by [Attributes][attributes].

## Notation

In the remainder of this document, specifying [States][state] and their
[Transitions][transition] is done as follows:

- `⊙ -> a`: transition into existence, with a initial state named `a`.
- `a -> b`: transition from state `a` to state `b`
- `b -> ⊗`: transition out of existence, with the final meaningful state named
  `b` and `⊗` denoting the special Exit state.

For example, an FSM can be described as follows, where each line denotes a
possible transition:

```text
⊙             -> initializing
initializing  -> operating
operating     -> finalizing
finalizing    -> ⊗
```

Note that in this example, while multiple [Transitions][transition] mention the
same [State][state], [States][state] have unique names. Therefore, these
[Transitions][transition] refer to the same [State][state].

For brevity, when [State][state] [Transitions][transition] must follow a fixed
sequence, this is simplified as:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

[attributes]: ./attributes.md
[entity]: ./entity.md
[exit]: #exit
[state]: #state
[timestamp]: ./time.md#timestamp
[transition]: #transition
