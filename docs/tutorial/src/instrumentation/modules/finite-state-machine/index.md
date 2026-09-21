# Finite-State Machine

The Finite-State Machine semantic module defines an entity lifecycle as states
and allowed transitions. The parser validates the topology and derives event
cardinality from it. The generated instrumentation APIs use that topology to
enforce transition order.

For example, a task might move from `queued` to `running`, then to either
`completed` or `failed`. Making that lifecycle part of the schema gives analysis
tools enough meaning to detect unexpected transitions, find work that never
reached a final state, and measure how long entities remained in each state. A
user interface can also present the declared lifecycle and the observed path
through it.

The [Resource](../resource/index.md) lessons later show how an FSM state can
declare the resources an entity uses while it remains in that state.
