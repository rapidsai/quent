# Push/Pop Ranges

## NVTX Push/Pop semantics

Push/Pop ranges are **thread-scoped** and **stack-based**. Each thread
maintains its own implicit stack:

```
nvtxRangePushA("outer")     // depth 0
  nvtxRangePushA("inner")   // depth 1
  nvtxRangePop()             // depth 1
nvtxRangePop()               // depth 0
```

This naturally forms a **tree of spans per thread**. The nesting is implicit
— determined by call order within a single thread.

## Capture (injection library)

The injection library is a stateless forwarder. It does **not** maintain
push/pop stacks or track parent-child relationships.

Each NVTX call produces one raw event:

- `Push { thread_id, timestamp, domain_handle_id?, attributes? }`
- `Pop { thread_id, timestamp, domain_handle_id? }`

That's it. No span IDs, no parent pointers, no tree construction.

## Analysis (analyzer)

The analyzer reconstructs the span tree from the raw event stream:

1. Group `Push`/`Pop` events by `(thread_id, domain_handle_id)`.
2. Replay in timestamp order — pushes open a span, pops close the most
   recent open span.
3. Build parent-child relationships from the implied stack order.
4. Map the result into Quent Trace entities (one per thread+domain pair).

### Mapping to Quent Traces

```
RtTrace (one per thread+domain)
├── roots: Vec<SpanId>          // top-level pushes
└── spans: HashMap<SpanId, RtSpan>
    └── RtSpan
        ├── name (from Push message)
        ├── parent_id (derived from stack replay)
        ├── intervals: [push_timestamp, pop_timestamp)
        └── attributes (from Push attributes)
```

## Attributes from nvtxEventAttributes_t

When `nvtxDomainRangePushEx` is called with full attributes:

| nvtxEventAttributes_t field | Quent mapping |
|---|---|
| `message` (ascii/unicode/registered) | Span `name` (registered handles resolved via prior `RegisterString` events) |
| `category` | `nvtx.category` (resolved via prior `NameCategory` events) |
| `color` (if colorType != UNKNOWN) | `nvtx.color` (u32, ARGB) |
| `payload` (if payloadType != 0) | `nvtx.payload` (typed: u64/i64/f64/u32/i32/f32) |

For the simple `nvtxRangePushA(const char*)` variant, only the message
(span name) is available.
