# Start/End Ranges → Quent FSM

## NVTX Start/End semantics

Start/End ranges are **process-scoped** and matched by an explicit
`nvtxRangeId_t`. Unlike Push/Pop, they:

- Can span across threads (start on thread A, end on thread B).
- Can overlap arbitrarily — no implicit nesting or tree structure.
- Are identified by a handle (`nvtxRangeId_t`, a `uint64_t`) returned by
  `nvtxRangeStartEx` and passed to `nvtxRangeEnd`.

```c
nvtxRangeId_t r1 = nvtxRangeStartA("request A");
nvtxRangeId_t r2 = nvtxRangeStartA("request B");
nvtxRangeEnd(r1);  // can end in any order
nvtxRangeEnd(r2);  // can end on any thread
```

## Quent FSM: NvtxStartEndRange

A two-state FSM:

```
⊙ → active → ⊗
```

- `⊙ → active`: Triggered by `nvtxRangeStart*`. Carries attributes from
  `nvtxEventAttributes_t` (message, color, category, payload).
- `active → ⊗`: Triggered by `nvtxRangeEnd`.

Each Start/End range instance gets its own FSM instance. The analyzer
creates these from paired `RangeStart` / `RangeEnd` events.

## nvtxRangeId_t assignment

The injection library controls the `nvtxRangeId_t` value returned to the
caller. Strategy: atomically incrementing `u64` counter. The value is
included in the serialized `RangeStart` and `RangeEnd` events as
`range_handle_id`.

The injection maintains no mapping. The analyzer pairs `RangeStart` and
`RangeEnd` events by matching `range_handle_id`.

## Attributes

Same mapping as Push/Pop (see [push-pop-ranges.md](./push-pop-ranges.md)):

| nvtxEventAttributes_t field | Quent attribute |
|---|---|
| `message` | FSM instance name |
| `category` | `nvtx.category` |
| `color` | `nvtx.color` (u32) |
| `payload` | `nvtx.payload` (typed) |

## Duration

FSM duration is derived automatically from the two transition timestamps.
This is standard Quent FSM behavior — no special handling needed.
