# NVTX to Quent Event Mapping

## Summary

| NVTX concept | Quent concept | Notes |
|---|---|---|
| Push/Pop ranges (per-thread) | Trace entity | One Trace per (thread, domain). Spans form a tree. See [push-pop-ranges.md](./push-pop-ranges.md). |
| Start/End ranges (cross-thread) | FSM entity (`NvtxStartEndRange`) | Two states: `active` and exit. See [start-end-ranges.md](./start-end-ranges.md). |
| Marks | Quent Event | Instant event with attributes. No span/duration. |
| Domain | String attribute (`nvtx.domain`) | See [domains-and-categories.md](./domains-and-categories.md). |
| Category | String attribute (`nvtx.category`) | Resolved via domain's category name table. See [domains-and-categories.md](./domains-and-categories.md). |
| Message | Span name (push/pop) or FSM instance name (start/end) | |
| Color | Attribute (`nvtx.color`, ARGB u32) | |
| Payload | Attribute (`nvtx.payload`, typed) | Preserves original type (u64, i64, f64, u32, i32, f32). |
| Registered string | Forwarded as raw handle ID | The injection emits `RegisterString` events with the handle ID and value. Subsequent events reference the raw handle ID; the analyzer resolves it. |
| Domain resource (create/destroy) | TBD | NVTX resource naming — may map to an attribute or entity. Not yet decided. |
| OS thread naming | Attribute on the thread's Trace entity | |

## Design rationale

### Why Traces for Push/Pop

Push/Pop ranges are thread-scoped and stack-based. They form a tree of spans
per thread, which is exactly what Quent's Trace entity models. The mapping is:

- `nvtxRangePush*` → `SpanInit` + `SpanEnter` (declares and enters a new child span)
- `nvtxRangePop` → `SpanExit` + `SpanClose` (exits and closes the top-of-stack span)

Parent-child relationships are derived from the push/pop stack order.

### Why FSMs for Start/End

Start/End ranges are process-scoped, identified by an explicit
`nvtxRangeId_t`, and can overlap arbitrarily across threads. They do not form
a tree. Quent's FSM model is flat and independent — each FSM instance has its
own lifecycle — which matches this behavior.

### Why attributes for domains

NVTX domains are a logical namespace for scoping categories and registered
strings. They do not imply hierarchy or capacity. Quent Resource Groups carry
structural meaning (parent/child containment), which would be a semantic
mismatch. A plain string attribute is the simplest correct representation.
