# Architecture Research

**Domain:** NVTX ingestion pipeline integrated into the Quent model-driven telemetry stack
**Researched:** 2026-07-08
**Confidence:** MEDIUM-HIGH (Quent layering HIGH from codebase map; NVTX injection internals MEDIUM — grounded in NVTX headers/CUPTI docs + training, not exercised against live headers here)

## Executive Framing

This is an **integration** architecture, not a greenfield domain. Quent already owns the
whole "model → instrument → export → collect → analyze → serve → UI" spine. The NVTX work
adds **one new event *source*** (the injection library) at the top of that spine and **one new
domain** (`domains/nvtx/{model,analyzer,server,ui}`) mirroring `domains/query_engine/`. The
only genuinely novel machinery — with no analogue elsewhere in Quent — is the **fan-out
mediator** that lets Quent be the single NVTX injection while still cooperating with Nsight
Systems / AON.

Two hard constraints shape every boundary decision:

1. **NVTX callbacks fire synchronously on the application's own threads, carry no timestamp,
   and sit inside a strict latency budget.** The capture layer must stamp-and-hand-off cheaply;
   any real work (interpretation, serialization, I/O) happens off the hot path.
2. **NVTX allows exactly one injection per process** (an invariant, not a bug). Multi-consumer
   coexistence is therefore *the* structural problem, solved by the mediator owning the single
   slot and walking a sink list.

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│                    Instrumented Application (libcudf, cuCascade, test app) │
│                    emits NVTX: RangePush/Pop, RangeStart/End, Mark,        │
│                    Domain*, RegisterString, NameCategory, Resource*,       │
│                    Payload{SchemaRegister, EnumRegister, *Payload}         │
└───────────────────────────────┬──────────────────────────────────────────┘
                                 │ NVTX runtime dispatch through installed
                                 │ function tables (CORE / CORE2 / PAYLOAD ext)
                                 ▼   [app thread · no timestamp · latency budget]
┌──────────────────────────────────────────────────────────────────────────┐
│  FAN-OUT MEDIATOR  (owns the single InitializeInjectionNvtx2 slot)         │
│  - installs mediator callbacks into NVTX's real function tables            │
│  - sink registry (lock-free read on hot path)                              │
│  - per-sink SHADOW function tables                                         │
│  - dlopen passthrough of NVTX_INJECTION64_PATH (nsys) as one more sink     │
│      walks sinks → [ Quent sink ] [ external tool sink ] ...               │
└───────────────┬───────────────────────────────────────┬──────────────────┘
                │ (Quent sink callback)                  │ (verbatim forward)
                ▼  stamp Event::new_now, build NvtxEvent  ▼  nsys / AON
┌──────────────────────────────────────────────────────┐  (unmodified)
│  QUENT NVTX SINK  (quent-nvtx-injection)              │
│  callback args → raw NvtxEvent (no interpretation);   │
│  cheap hand-off (queue / direct emit)                 │
└───────────────┬──────────────────────────────────────┘
                │ NvtxEvent
                ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  INSTRUMENTATION BRIDGE  (quent-nvtx-instrumentation)                      │
│  install<T: From<NvtxEvent>>(sender, session_id) → From<NvtxEvent>         │
│  → EventSender::emit → unbounded tokio mpsc                                │
└───────────────────────────────┬──────────────────────────────────────────┘
                                 │ Event<T>  ── EXISTING QUENT SPINE BELOW ──
                                 ▼
        exporters (ndjson/msgpack/postcard) │ collector gRPC (:7836)
                                 ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  NVTX ANALYZER  (domains/nvtx/analyzer)                                    │
│  - resolves handles (domains, registered strings, categories) from stream │
│  - reconstructs ranges as single-state FSMs (open → exit)                 │
│  - TOLERATES unclosed ranges + cross-thread disorder (synthesize exits)   │
└───────────────────────────────┬──────────────────────────────────────────┘
                                 │ UiAnalyzer view types (ts-rs)
                                 ▼ HTTP JSON (:8080)
┌──────────────────────────────────────────────────────────────────────────┐
│  NVTX SERVER (domains/nvtx/server)  →  UI (ui/, reuse Timeline + Gantt)    │
└──────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| `quent-nvtx-sys` | Raw FFI: bindgen over NVTX C headers; C shim exporting the strong `InitializeInjectionNvtx2` symbol overriding NVTX's weak one | bindgen + minimal `cc` shim; Linux/macOS 64-bit only (compile-error elsewhere) |
| `quent-nvtx-events` | Serde types mirroring raw NVTX calls **verbatim** — handles left as opaque ints/pointers; payload-extension types | plain `#[derive(Serialize,Deserialize)]` structs/enums; **no dependency on Quent domain crates** (upstreamable) |
| `quent-nvtx-mediator` | Owns the single injection slot; installs mediator callbacks; maintains sink registry + per-sink shadow tables; dlopen passthrough | lock-free sink list (`arc-swap`/atomic snapshot); generic over sink trait; **zero Quent deps** (most upstreamable piece) |
| `quent-nvtx-injection` | The Quent *sink*: callback args → `NvtxEvent`, timestamp on receipt, cheap hand-off; one-shot `install_hook()` | implements mediator's sink trait; stamps via monotonic clock / `Event::new_now` |
| `quent-nvtx-instrumentation` | Thin bridge: forward `NvtxEvent` into Quent's `EventSender` as domain events | `install<T: From<NvtxEvent>>(sender, session_id)` (PR #87 shape) |
| `domains/nvtx/model` | Quent `model!` defining the NVTX range as a **single-"open"-state FSM** entity (+ mark/resource entities) | proc-macro model definition, mirrors `domains/query_engine/model` |
| `domains/nvtx/analyzer` | Reconstruct ranges → spans; resolve handles from the stream; **close dangling ranges**; tolerate disorder | implements `UiAnalyzer`; reuses `crates/analyzer` FSM/span runtimes |
| `domains/nvtx/server` | axum HTTP routes + tonic collector composition + analyzer/timeline caches | mirrors `domains/query_engine/server` |
| `domains/nvtx/ui` + `ui/` | ts-rs view types + React route reusing `TimelineController`/`OperatorGanttChart` | spans-per-row on shared time axis |
| in-repo NVTX test app | Deterministic NVTX emitter for CI (no GPU) | small Rust/C app emitting a fixed NVTX script |

**Boundary rule of thumb:** everything **above** `EventSender::emit` is NVTX-specific and
application-agnostic (belongs in `integrations/nvtx/` crates, cleanly separable/upstreamable);
everything **below** reuses the existing Quent spine unchanged; the only new *domain* code is
the `domains/nvtx/` mirror.

## Recommended Project Structure

```
integrations/nvtx/                 # application-agnostic capture layer (separable, upstreamable)
├── sys/                           # quent-nvtx-sys: bindgen + C shim (strong injection symbol)
├── events/                        # quent-nvtx-events: verbatim raw NVTX serde types (NvtxEvent)
├── mediator/                      # quent-nvtx-mediator: fan-out, shadow tables, passthrough
├── injection/                     # quent-nvtx-injection: Quent sink (callback→NvtxEvent, install_hook)
└── instrumentation/              # quent-nvtx-instrumentation: NvtxEvent → EventSender bridge

domains/nvtx/                      # domain layer (mirror of domains/query_engine/)
├── model/                         # single-state FSM range entity via model!
├── analyzer/                      # handle resolution + range reconstruction + tolerance; UiAnalyzer
├── server/                        # axum + tonic composition, caches
└── ui/                            # ts-rs view/timeline types

examples/nvtx/                     # deterministic in-repo NVTX test app + end-to-end wiring
ui/src/routes/                     # new NVTX trace route (reuse Timeline/Gantt components)
```

### Structure Rationale

- **`integrations/nvtx/` (new top-level, matches PR #87's `integrations/nvtx/`):** keeps the
  capture crates physically separate from `crates/` and `domains/` so the injection+mediator
  layer can be lifted out and offered upstream to NVIDIA/NVTX without dragging Quent domain
  code. This directly serves the "Separability" constraint.
- **Split `sys` / `events` / `mediator` / `injection` / `instrumentation`:** each crate has one
  dependency direction and one reason to change. `events` and `mediator` deliberately carry
  **zero Quent dependencies** so they are the trivially-upstreamable units; `injection` is the
  Quent-flavored sink; `instrumentation` is the only crate that touches `EventSender`.
- **`domains/nvtx/` mirrors `domains/query_engine/`:** the codebase map explicitly says "copy
  the simulator/query_engine layout when building a new tool." NVTX is that new tool.
- Register every crate in **both** `members` and `default-members` of the root `Cargo.toml`;
  pin shared deps via `[workspace.dependencies]` (per STRUCTURE.md conventions).

## Architectural Patterns

### Pattern 1: Mediator-owns-slot, walk-a-sink-list fan-out

**What:** The mediator provides the process's single strong `InitializeInjectionNvtx2`. On
first NVTX call, NVTX hands it a `getExportTable` function; the mediator fetches the real CORE /
CORE2 / payload-extension function tables and writes **its own** callback pointers into every
slot. Each registered sink (Quent, plus any external tool) is handed a *synthetic*
`getExportTable` backed by a **per-sink shadow function table** in mediator memory, and populates
its shadow slots exactly as if it were the sole injection. When a real NVTX call fires a mediator
callback, the mediator iterates the sink list and invokes each sink's corresponding non-null
shadow slot with the same arguments. This is precisely Johan's "populate the global table, keep a
shadow table per sink" and Lawrence's "walked handler list."

**When to use:** Whenever >1 NVTX consumer must coexist in one process — the common case here
(Quent + nsys), not an edge case.

**Trade-offs:**
- (+) External tools stay unmodified; passthrough is "just another sink."
- (+) Sinks are decoupled from each other and from NVTX's real ABI.
- (−) Adds one indirection per NVTX call across N sinks on the app's hot thread — the sink walk
  must be lock-free and allocation-free.
- (−) Init ordering is delicate: whoever wins the injection slot must be resolved before the
  first NVTX call; if `NVTX_INJECTION64_PATH` already points at nsys, Quent must instead be
  loaded such that the *mediator* wins and dlopens nsys itself.

**Example (shape, not literal ABI):**
```rust
// Hot path — runs on the app's thread, no allocation, no lock.
extern "C" fn on_domain_range_push(domain: nvtxDomainHandle_t, attr: *const EventAttributes) -> i32 {
    let sinks = SINKS.load();            // arc-swap snapshot: single atomic load
    for sink in sinks.iter() {
        if let Some(cb) = sink.core2.domain_range_push { cb(domain, attr); }
    }
    0
}
```

### Pattern 2: Stamp-and-hand-off (keep the app thread cheap)

**What:** NVTX callbacks deliver no timestamp and must return fast. The Quent sink stamps a
monotonic timestamp **at callback entry** (the same clock behind `Event::new_now` /
`crates/time`), captures the raw arguments into an owned `NvtxEvent` (copying `A`/`W` string
bytes because NVTX does not guarantee they outlive the call), and hands off. Interpretation
(handle resolution, span pairing) is deferred to the analyzer.

**When to use:** Always, for the capture→bridge boundary.

**Trade-offs / options for the hand-off:**
- **Direct `EventSender::emit`** (simplest, PR #87 shape): `emit` stamps and pushes onto an
  **unbounded** tokio mpsc — non-blocking, no backpressure stall. Cost = one `NvtxEvent`
  allocation + string copies + a channel push per NVTX call, on the app thread. Start here.
- **Lock-free ring / thread-local buffer + drainer thread** (fallback if the above is too heavy
  under high-frequency ranges): callback writes into a per-thread SPSC buffer; a dedicated
  drainer converts and forwards to `EventSender`. Adds complexity; only justify with measurement.
- **Avoid the collector's *bounded* mpsc on the hot path** — the existing collector client uses
  a bounded (1024) channel whose `send` awaits when full (CONCERNS.md), which would stall the
  instrumented app. Keep NVTX capture on the unbounded emit side; let backpressure live at the
  exporter/collector boundary, not at the NVTX callback.

### Pattern 3: Raw events, interpret later (handle resolution in the analyzer)

**What:** The injection crate emits NVTX calls verbatim — `DomainCreate{handle, name}`,
`RegisterString{domain, handle, str}`, `NameCategory{domain, id, name}`, `RangePush{domain,
category, message(handle|literal), payload…}` — with handles as opaque values. The **analyzer**
builds `handle → name` maps from the register/create events (which, per NVTX contract, precede
their use) and resolves references during reconstruction.

**When to use:** For every handle-bearing NVTX concept (domains, registered strings, categories,
resources, payload schemas/enums).

**Trade-offs:**
- (+) Hot path does zero map lookups / zero interpretation — cheapest possible capture.
- (+) Injection crate stays "dumb" and upstreamable; all semantics live in Quent's analyzer.
- (+) Robust to tools that register lazily — resolution is a stream fold, not a live cache.
- (−) The analyzer must handle *forward references* defensively (a handle used before its
  register event is observed, e.g. dropped early events) — resolve to a placeholder, don't panic.

### Pattern 4: NVTX range as a single-"open"-state FSM

**What:** Model a range as a Quent FSM with one meaningful state. A `RangePush`/`RangeStart`
creates the FSM instance and emits an `"open"` transition at `t_start`; the matching
`RangePop`/`RangeEnd` emits an `"exit"` transition at `t_end`. Quent's runtime FSM builder
(`RtFsmBuilder`) requires ≥2 transitions with the **last named `"exit"`**, so a well-formed
range is exactly `["open"@start, "exit"@end]` → one span. Marks are instantaneous (model as a
zero-length range `start==end`, or a distinct `Mark` entity). Push/Pop nest per-thread (a
stack); `RangeStart`/`RangeEnd` correlate by `nvtxRangeId_t` and may cross threads.

**When to use:** The core modeling decision, per Johan's plan (rapidsai/quent#76).

**Trade-offs:**
- (+) Reuses the existing FSM → span → binned-timeline analyzer and the Timeline/Gantt UI wholesale.
- (−) The FSM machinery's completeness guarantees clash with real NVTX streams — see tolerance
  work below.

## Data Flow

### Capture → Emit Flow

```
[App NVTX call]
    ↓  (NVTX runtime dispatch via installed function table)
[Mediator callback]  ── app thread, stamp point ──
    ↓  walk sinks
    ├─→ [External tool sink (nsys)]  (verbatim passthrough, fire-and-forget)
    └─→ [Quent sink]
            ↓  stamp Event::new_now + copy args → NvtxEvent (raw)
            ↓  hand-off (unbounded emit; or ring→drainer)
        [instrumentation bridge]  From<NvtxEvent> → domain Event<T>
            ↓  EventSender::emit → unbounded tokio mpsc → per-entity forwarder
        [exporter files | collector gRPC :7836]
```

### Reconstruction → Serve Flow

```
[event streams on disk / collected]
    ↓  AnalyzerCache lazily imports (mirror analyzer_cache.rs)
[NVTX analyzer]
    ├─ fold register/create/category events → handle→name maps
    ├─ pair Push/Pop (per-thread stack) & Start/End (by RangeId, cross-thread)
    ├─ synthesize "exit" for ranges still open at trace end   ← TOLERANCE
    ├─ drop/bucket malformed (end<start, orphan pop)          ← TOLERANCE
    └─ build single-state FSMs → spans
    ↓  UiAnalyzer view types (ts-rs)
[axum HTTP :8080 /api/…]  →  [UI route: TimelineController + OperatorGanttChart]
```

### Key Data Flows

1. **Handle resolution is a stream fold, not a hot-path cache** — maps are built once during
   reconstruction from the same event stream that carries the ranges.
2. **Timestamps originate at the Quent sink**, never from NVTX. All timestamps then flow through
   `crates/time` exactly like existing Quent telemetry, so binning/timeline reuse is free.
3. **Passthrough is one-directional and lossless from nsys's perspective** — the mediator never
   transforms args for the external sink; it forwards the raw NVTX call so nsys behaves as if it
   owned the injection.

## Tolerance Requirements (must-fix integration risk)

Real NVTX streams routinely contain **unclosed ranges at process exit** and **cross-thread
ordering imperfections**. Quent's analyzer today is hostile to both:

- `RtFsmBuilder::try_build` **errors** if the final transition isn't named `"exit"`, and
  `RtFsmsBuilder::try_build` **bubbles the first error → the entire engine build fails**
  (`crates/analyzer/src/fsm/runtime.rs:122-128, 309-313`; CONCERNS.md "One incomplete FSM fails
  the whole analyzer build").
- Span construction `SpanUnixNanoSec::try_new(start, end).unwrap()` **panics** on out-of-order /
  duplicate-timestamp transitions (`crates/analyzer/src/fsm/runtime.rs:170`,
  `crates/analyzer/src/fsm/mod.rs:125-135`; CONCERNS.md "Analyzer ordering assumptions").

**Recommended handling (do it at the NVTX-analyzer level; do not weaken the shared analyzer's
guarantees for query_engine):**
1. **Pre-close dangling ranges** in the NVTX analyzer before feeding the FSM builder — synthesize
   an `"exit"` transition at the trace's last-observed timestamp (or an explicit
   "still-open / truncated" sentinel), so every range is a well-formed 2-transition FSM.
2. **Reject/bucket malformed pairs** (orphan pop, `end < start`) into a separate diagnostics
   bucket rather than letting them reach `try_new().unwrap()`.
3. **Separately, upstream the shared-analyzer robustness fix** the TODO already anticipates —
   move incomplete FSMs into their own bucket in `RtFsmsBuilder::try_build`, and convert the span
   `unwrap()`s to error propagation. This benefits query_engine too and de-risks the whole
   integration, but should be additive (opt-in bucket), not a semantic change for existing domains.

This is the single highest-risk seam in the integration and warrants a dedicated phase / deeper
research at planning time.

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| Low range rate (test app, light libcudf) | Direct `EventSender::emit` from the callback is fine; unbounded mpsc absorbs bursts |
| High range rate (tight-loop GPU kernels) | Move to ring-buffer + drainer hand-off; batch `NvtxEvent`s; consider `registered strings` to avoid per-call string copies |
| Large traces (long runs, many domains/threads) | Same in-memory-analyzer scaling limits as query_engine (CONCERNS.md): 32-analyzer moka cache, whole-model reconstruction, unpaginated list endpoints — inherit the same Arrow-migration/pagination roadmap |

### Scaling Priorities

1. **First bottleneck: per-call cost on the app thread.** Measure `NvtxEvent` allocation +
   string copy + channel push under a realistic range rate. If it dents app latency, switch
   hand-off to a lock-free ring + drainer (Pattern 2 fallback). Prefer registered-string handles
   over literal `A`/`W` strings to make copies cheap/rare.
2. **Second bottleneck: analyzer memory on large traces.** Same as query_engine — bounded by RAM,
   whole-model rebuild. Reuse (don't re-solve) the existing Arrow/columnar migration plan.

## Anti-Patterns

### Anti-Pattern 1: Interpreting handles on the hot path

**What people do:** Resolve registered-string/domain handles to names inside the NVTX callback
(live handle→name map with a lock).
**Why it's wrong:** Adds a locked map lookup on the app's latency-budgeted thread and couples the
injection crate to Quent semantics, killing upstreamability.
**Do this instead:** Emit raw handle values; resolve in the analyzer via a stream fold
(Pattern 3).

### Anti-Pattern 2: Blocking or awaiting in the callback

**What people do:** Push onto a **bounded** channel (or otherwise block) inside the callback.
**Why it's wrong:** Bounded backpressure stalls the instrumented application's own threads — the
exact failure mode CONCERNS.md flags for the collector client's bounded mpsc.
**Do this instead:** Stamp-and-hand-off via unbounded emit or a lock-free buffer; let backpressure
live downstream at the exporter/collector, never at the NVTX callback.

### Anti-Pattern 3: Letting real NVTX streams hit the analyzer's `unwrap()`/whole-build-fail paths

**What people do:** Feed raw ranges (with unclosed/last-thread-lost pops) straight into the shared
FSM builder.
**Why it's wrong:** One crashed range aborts the entire trace build; one out-of-order pair panics
the analyzer.
**Do this instead:** Pre-close and bucket at the NVTX-analyzer level (Tolerance Requirements);
land the additive incomplete-FSM bucket in the shared analyzer.

### Anti-Pattern 4: Making the mediator a Quent-only injection

**What people do:** Hard-wire Quent's sink as the injection (PR #87's current single-consumer
shape) and skip the sink abstraction.
**Why it's wrong:** Breaks nsys/AON coexistence — an explicit v1 requirement — and re-couples the
upstreamable mediator to Quent.
**Do this instead:** Mediator owns the slot and knows only a generic `Sink` trait; Quent is one
sink, the dlopen'd external tool is another.

### Anti-Pattern 5: Building new capture code on the unused preliminary crates

**What people do:** Reuse `crates/fsm`, `crates/schema`, etc.
**Why it's wrong:** Those seven crates are dead preliminary work for issue #191 (CONCERNS.md /
STRUCTURE.md) and must not be extended.
**Do this instead:** Use `crates/model-macros` + `crates/analyzer` FSM machinery, as query_engine does.

## Integration Points

### External Services / System Interfaces

| Interface | Integration Pattern | Notes |
|-----------|---------------------|-------|
| NVTX runtime (target app) | Own `InitializeInjectionNvtx2` (strong symbol) + install CORE/CORE2/payload function tables | Linux/macOS 64-bit only; weak-symbol mechanism excludes Windows (PR #87 already compile-errors there) |
| `NVTX_INJECTION64_PATH` (nsys/AON) | `dlopen` the pointed library, call its `InitializeInjectionNvtx2` with a mediator-supplied `getExportTable` → it populates a shadow table → forwarded as a sink | Coexistence must keep nsys working unmodified; init-order resolution is the sharp edge |
| Quent collector gRPC (:7836) / exporters | Reuse unchanged via `EventSender` | The NVTX bridge is just another event producer |
| Quent analyzer HTTP (:8080) / UI | Reuse `UiAnalyzer` + axum composition + Timeline/Gantt | New `domains/nvtx` mirror of query_engine |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| mediator ↔ sink | generic `Sink` trait (shadow function tables) | mediator has **no** Quent dep; upstreamable |
| Quent sink ↔ instrumentation bridge | `NvtxEvent` value + `From<NvtxEvent>` | the "raw events" seam; interpretation forbidden here |
| bridge ↔ Quent spine | `EventSender::emit` (unbounded mpsc) | reuse existing; do not add bounded backpressure |
| analyzer ↔ shared `crates/analyzer` | `UiAnalyzer` + FSM/span runtimes | pre-close/bucket before the FSM builder |

## Suggested Build Order

Dependency-driven; the question's order is right, with two refinements: **events precedes
injection** (shared vocabulary), and **the mediator is inserted under a working single-consumer
injection** rather than built first (prove capture end-to-end, then generalize to fan-out).

1. **`quent-nvtx-sys` + `quent-nvtx-events`** — FFI (bindgen + C shim) and verbatim raw
   `NvtxEvent` types. Vocabulary + linkage; nothing works without them. *(Rebase/evaluate PR #87
   here — it already has bindgen, the strong-symbol shim, and these event types.)*
2. **`quent-nvtx-injection` (single-consumer)** — callback → `NvtxEvent`, `install_hook()`,
   stamp-and-hand-off. **Proves capture** against a trivial emitter. (PR #87's shape, minus fan-out.)
3. **in-repo deterministic NVTX test app** — stand up early (used from step 2 onward) so CI can
   validate without GPU hardware.
4. **`quent-nvtx-mediator` (fan-out + passthrough)** — insert under the injection: mediator owns
   the slot, Quent becomes a sink, add `dlopen` passthrough. **Unblocks nsys coexistence.**
5. **`quent-nvtx-instrumentation`** — `install<T: From<NvtxEvent>>` bridge into `EventSender`.
   (Small; can land alongside step 2 for a first end-to-end file dump, then stays stable.)
6. **`domains/nvtx/model`** — single-"open"-state FSM range entity via `model!`.
7. **`domains/nvtx/analyzer`** — handle resolution (stream fold) + range reconstruction +
   **tolerance** (pre-close/bucket). Highest-risk phase; flag for deeper research.
8. **`domains/nvtx/server`** — axum routes + collector composition + caches (mirror query_engine).
9. **`domains/nvtx/ui` + `ui/` route** — ts-rs view types + React route reusing
   `TimelineController`/`TimelineRuler`/`OperatorGanttChart`.
10. **Payload extension** (schemas/enums/binary payloads) — layer on after ranges work end-to-end
    (PR #87 marks payloads "Phase 5 — not yet emitted"); it threads through the same
    events→bridge→analyzer→UI chain and can extend, not block, the vertical slice.

**Parallelizable:** steps 6–9 (model/analyzer/server/ui) can begin against synthetic `NvtxEvent`
fixtures before the mediator (step 4) is finished, since they depend only on `quent-nvtx-events`,
not on live injection.

## Build-Order Implications for the Roadmap

- **A working vertical slice exists after step 9 using the single-consumer injection** — fan-out
  (step 4) can slip a phase without blocking "NVTX visible in the UI." Sequence phases so the
  slice is demonstrable before coexistence hardening.
- **Step 7 (analyzer/tolerance) is the critical-path risk.** It touches shared-analyzer
  invariants and needs the malformed/out-of-order fixtures CONCERNS.md says don't yet exist.
  Budget a dedicated phase and flag for deeper research.
- **Step 4 (mediator) is novel with no Quent analogue** and has the subtlest failure mode
  (injection-slot ordering vs. an already-set `NVTX_INJECTION64_PATH`). Flag for deeper research;
  keep it a standalone phase so it can be prototyped against nsys.
- **Steps 1–2 hinge on the PR #87 rebase decision** (adopt vs. reference). Resolve that early
  since it gates the FFI vocabulary everything else builds on.

## Sources

- Quent codebase map (HIGH): `.planning/codebase/{ARCHITECTURE,STRUCTURE,CONCERNS,INTEGRATIONS}.md`;
  `crates/analyzer/src/fsm/runtime.rs`; `crates/events/src/lib.rs`;
  `domains/query_engine/analyzer/src/ui.rs`
- Project context (HIGH): `.planning/PROJECT.md` (PR #87 crate layout, Johan/Lawrence fan-out
  sketch, tolerance hazard, out-of-scope Windows/device-side)
- NVTX injection mechanism (MEDIUM): NVIDIA/NVTX headers & CUPTI docs —
  https://github.com/NVIDIA/NVTX ,
  https://nvidia.github.io/NVTX/doxygen/index.html ,
  https://docs.nvidia.com/cupti/main/main.html (InitializeInjectionNvtx2 / NVTX_INJECTION64_PATH)
- NVTX payload-extension callback IDs (MEDIUM): `nvToolsExtPayload.h` (`NVTX3EXT_CBID_*` —
  SchemaRegister, EnumRegister, {Mark,RangePush,RangePop,RangeStart,RangeEnd}Payload)

---
*Architecture research for: NVTX ingestion integrated into Quent*
*Researched: 2026-07-08*
