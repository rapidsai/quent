# Analyzer Integration

## Overview

The NVTX analyzer is a reusable builder (`NvtxModelBuilder`) provided by
`integrations/nvtx/analyzer/`. Any application's model builder can plug
it in to handle `NvtxEvent` variants.

## Event routing

The application's model builder routes NVTX events to the `NvtxModelBuilder`:

```rust
// In SimulatorModelBuilder::try_push
SimulatorEvent::Nvtx(nvtx) => {
    self.nvtx_builder.try_push(Event::new(id, timestamp, nvtx))
}
```

This follows the existing pattern — `SimulatorModelBuilder` already
delegates to `InMemoryQueryEngineModelBuilder`, `InMemoryResourcesBuilder`,
`TaskBuilder`, and `RtTraceBuilder` by event variant.

## NvtxModelBuilder responsibilities

### During ingest (try_push)

Accumulates raw events. No interpretation beyond collecting them in order.

### During build (try_build)

Reconstructs Quent entities from the raw event stream:

1. **Handle resolution**: Builds lookup tables from registration events:
   - `DomainCreate` → `domain_handle_id → name`
   - `RegisterString` → `(domain_handle_id, string_handle_id) → value`
   - `NameCategory` → `(domain_handle_id, category_id) → name`
   - `SchemaRegister` → `schema_id → schema definition`

2. **Push/Pop → Traces**: Groups push/pop events by `(thread_id,
   domain_handle_id)`. Replays in timestamp order to reconstruct span
   trees. Produces one `RtTrace` per group.

3. **Start/End → FSMs**: Pairs `RangeStart`/`RangeEnd` by
   `range_handle_id`. Produces one `NvtxStartEndRange` FSM per pair.

4. **Marks → point events**: Stored as instant events on the timeline.

5. **Payload resolution**: For events with payloads, uses the schema
   registry to interpret raw bytes. For the well-known Quent schema ID,
   extracts entity UUIDs for FSM correlation.

6. **Thread naming**: Applies `NameThread` events to traces.

## FSM correlation

When the analyzer finds a Quent payload (well-known schema ID) on an
NVTX range, it links the resulting trace span or FSM to the referenced
Quent entity UUID. This allows the UI to show NVTX activity within the
context of FSM state transitions.

Library NVTX ranges (without Quent payloads) that are nested inside
an application's annotated range on the same thread inherit the
correlation via the span tree structure.

## Output

The `NvtxModelBuilder::try_build()` produces an `NvtxModel` containing:

- `traces: Vec<RtTrace>` — reconstructed span trees
- `fsms: Vec<NvtxStartEndRange>` — start/end range FSMs
- `marks: Vec<NvtxMark>` — instant events
- `correlations: HashMap<Uuid, Vec<...>>` — NVTX entity → Quent FSM entity links

The exact output types will be refined during implementation.

## Reusability

`NvtxModelBuilder` is application-agnostic. It only depends on `NvtxEvent`
and core Quent types (`RtTrace`, FSM types, etc.). Any application that
includes `Nvtx(NvtxEvent)` in its event enum can use it.
