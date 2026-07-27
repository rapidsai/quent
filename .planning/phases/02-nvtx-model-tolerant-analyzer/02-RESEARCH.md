# Phase 2: NVTX Model & Tolerant Analyzer - Research

**Researched:** 2026-07-23
**Domain:** Event-stream reconstruction (hand-written, framework-free) over the Phase-1 `NvtxEvent` vocabulary
**Confidence:** HIGH on vocabulary/mechanics/tolerance (grounded in source); MEDIUM on crate placement (discretion); one HIGH-confidence blocking gap (no thread identity on the stream)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** NVTX is a self-standing telemetry stream rendered alongside other Quent telemetry, NOT merged. The NVTX model must stand entirely on its own — it cannot require an `Engine` root, operators, or any query-engine entity.
- **D-02:** "Where the NVTX lane docks" is a Phase 3 UI-layout concern, not Phase 2.
- **D-03:** True semantic correlation (range↔operator) is deferred to v2 (COR-01). Design the Phase 2 model so it does not wall off a later correlation pass — **keep per-range thread/domain/precise timestamps.**
- **D-04:** The current (#191 YAML/schema) design is producer-only — no consumer/analyzer/reconstruction/serving crate.
- **D-05:** "Legacy" conflates the declaration DSL (`model!`/`fsm!`/`entity!`, rejected — NVTX never needs it) and the reconstruction+UI-view framework (`crates/analyzer`+`crates/ui`, not replaced). NVTX never touches the macro DSL.
- **D-06:** Phase 2 = a framework-free, hand-written reconstruction core. Depends only on the shared runtime (`quent-events`) + `nvtx-events`. **No proc-macros, no `schema::Schema`, no legacy `crates/analyzer` dependency.**
- **D-07:** Do NOT declare NVTX as a `schema::Schema`.
- **D-08:** A range is a plain span (start/end interval) — drop the single-state-FSM framing. Define **our own plain span type** (avoids the transitive `quent-model` `RtFsm` trait link). Isomorphic to a single-state FSM so a Phase-3 adapter can map it trivially.
- **D-09:** Model the full explicit NVTX surface — domains, threads, ranges (spans), marks (instants), categories (namespaced by `(domain, categoryId)`), **and resources** (named lifespans: `ResourceCreate → ResourceDestroy` span + resolved name + `identifier_type` label + domain grouping).
- **D-10:** Model NVTX resources as named object lifespans, NOT as Quent capacity-resources. No fabricated capacity/occupancy/utilization semantics.
- **D-11:** Core resource types get a label now; unknown/CUDA-extension `identifier_type`s pass through raw. Same "core-now, extension-deferred" line as payloads.
- **D-12:** Tolerance is handled by construction inside our own reconstruction core. The panic-prone legacy `crates/analyzer/src/fsm/runtime.rs` is simply not on our path. We close ranges open at trace-end, sort by timestamp, and handle duplicate timestamps ourselves.
- **D-13:** Flag synthetically-closed ranges (never popped, closed at trace-end) so Phase 3 can render them distinctly.
- **D-14:** Stable placeholders — keep unresolved things visible, never drop. Distinguish *legitimately unnamed* (default domain `0`, unnamed thread → clean default labels) vs *referenced-but-unresolved* (non-zero handle with no registration → placeholder exposing the raw id).
- **D-15:** Two-pass reconstruction for handle resolution. First pass builds all lookup tables; second pass reconstructs entities. (Handles may also be genuinely unregistered per D-14.)
- **D-16:** The legacy-vs-new tension bites at the serving/UI seam (Phase 3), not Phase 2. Design the Phase 2 core so spans/marks map cleanly onto timeline view types either way. **Not decided yet.**

### Claude's Discretion
- **Crate placement** — reconstruction core is Quent-side; does NOT belong in the upstreamable `integrations/nvtx/{events,injection}` set. Lean: `integrations/nvtx/analyzer` (sibling to `bridge`). Planner finalizes vs. a `domains/nvtx/` placement.
- **Nested-range representation** — flat spans + time-containment vs. explicit parent/child tree. Planner picks based on Phase 3 rendering.
- **Test fixtures** — real capture from `nvtx-example` (happy path) + hand-crafted synthetic `Event<NvtxEventEntity>` streams for malformed cases.
- **Reconstruction strategy** — batch / two-pass (D-15) is the natural shape; planner details.

### Deferred Ideas (OUT OF SCOPE)
- **`analyzer-build`** — schema-driven consumer generator. Separate future initiative; would not serve NVTX.
- **Operator correlation (COR-01)** — v2. Phase 2 keeps the model correlation-ready (D-03) but builds none.
- **Inferring Quent capacity/utilization from NVTX** (D-10) — heuristic. Deferred.
- **Payload extension decode** (PAY-01/02) — v2. Core payload union is carried verbatim but not decoded here.
- **Phase 3 serving foundation (A vs B)** — flagged (D-16), decided at Phase 3.
- HTTP endpoints and UI rendering (Phase 3); fan-out mediator (Phase 4); real-GPU validation (Phase 5).
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MOD-01 | NVTX ranges modeled as Quent FSMs with a single "range open" state, in an NVTX domain | **Reframed by D-08:** a range is a plain span (isomorphic to a single-state FSM), not the legacy FSM machinery. Own span type — see "Plain Span Type." The "single range-open state" intent is preserved in *shape* (one start/end interval). |
| MOD-02 | Marks, domains, threads, categories represented in the NVTX Quent model | Model surface enumerated in "Model Representation." Marks = instants; domains/threads/categories = resolved labels + grouping keys. |
| ANA-01 | Resolve registered-string handles from the event stream | `RegisterString { domain, handle, string }` → table keyed by `(domain, handle)`. See "Two-Pass Handle Resolution." |
| ANA-02 | Resolve domain and category names with `(domain, categoryId)` namespacing | `DomainCreate` → `domain`→name; `NameCategory { domain, category, name }` → `(domain, category)`→name. Never global. |
| ANA-03 | Push/Pop reconstruct as per-thread nested stacks (Pop matches most recent Push on same thread) | **BLOCKED by a vocabulary gap — see Open Question #1.** The stream carries NO thread identity; per-thread reconstruction is not currently possible for multi-threaded input. Resolution required before planning ANA-03. |
| ANA-04 | RangeStart/RangeEnd match process-wide by handle across threads | `RangeStart { domain, range_id, .. }` / `RangeEnd { domain, range_id }` matched by `range_id` in a `HashMap`. Thread-independent by construction — no gap here. |
| ANA-05 | Tolerant: close unclosed at trace-end; out-of-order/duplicate timestamps never panic/abort | Handled by construction in our own core (D-12). Legacy panic paths verified off-path. See "Tolerance by Construction." |
| ANA-06 | Range statistics per name/domain/category: count + total/avg/min/max duration | Aggregate over completed spans grouped by `(name, domain, category)`. See "Range Statistics." |
</phase_requirements>

---

## Summary

Phase 2 builds a **hand-written, framework-free reconstruction core** that replays the flat
`Event<NvtxEventEntity>` stream produced in Phase 1 and materializes an in-memory NVTX model:
resolved domains, threads, categories, registered strings, ranges (spans), marks (instants), and
resources (named lifespans). The Phase-1 vocabulary is complete and verified in source
(`integrations/nvtx/events/src/{lib,attributes,payload}.rs`): 12 `NvtxEvent` variants, all handles
raw integers, nothing resolved at capture. The core depends only on `nvtx-events`, `nvtx-bridge`,
`quent-events`, and `quent-time` — never on `crates/analyzer`, `crates/model`, or the rejected
proc-macro DSL. Because we own reconstruction, the panic-prone legacy `crates/analyzer/src/fsm/runtime.rs`
(zero-transition `unwrap()`, first-incomplete-FSM `?`-abort) is genuinely off our path (D-12,
verified below).

**One blocking gap dominates this research.** The NVTX event stream carries **no thread identity
anywhere** — not on the `NvtxEvent` variants (`RangePush`/`RangePop` carry only `domain: u64`), not
on the `Event<T>` envelope (`id` is the per-process session UUID, shared by every NVTX event;
`timestamp` is a global monotonic clock; there is no thread field), and it is not recoverable from a
captured ndjson/msgpack/postcard file. The injection layer *knows* the calling thread (it maintains a
per-thread `RANGE_DEPTH` thread-local) but does **not** stamp it onto the emitted event. **Therefore
ANA-03 / success-criterion-1 ("Pop matches the most recent Push on the same thread") is not
satisfiable from the current stream for any multi-threaded producer — including the primary targets
libcudf and cuCascade, which are heavily multi-threaded.** `RangeStart/RangeEnd` (ANA-04) is
unaffected because it matches process-wide by `range_id`. This gap must be resolved before ANA-03 can
be planned; the recommended resolution (a small Phase-1 vocabulary addition of `thread_id: u32` on
`RangePush`/`RangePop`) is detailed in Open Question #1.

**Primary recommendation:** Build the core as a new crate `integrations/nvtx/analyzer` (sibling to
`bridge`, workspace `members` only). Two-pass reconstruction (D-15): pass 1 builds handle tables,
pass 2 replays events sorted by timestamp into an own `NvtxSpan` type with tolerance by construction.
**Do not begin ANA-03 until the thread-identity gap (Open Question #1) is decided** — everything else
(ANA-01/02/04/05/06, MOD-01/02, resources) is fully specified by the existing vocabulary and can
proceed against synthetic fixtures immediately.

---

## Architectural Responsibility Map

NVTX Phase 2 is a single-tier backend reconstruction stage; "tiers" map to pipeline stages.

| Capability | Primary Stage | Secondary Stage | Rationale |
|------------|--------------|-----------------|-----------|
| Emit verbatim `NvtxEvent`s | Capture (Phase 1, done) | — | Injection cdylib + bridge; already complete. Phase 2 adds no capture code. |
| Handle resolution (domain/string/category/thread/resource names) | Reconstruction core (pass 1) | — | Foreign handles resolved from the stream, not at capture (project constraint: "capture raw, resolve in analyzer"). |
| Range/resource span reconstruction | Reconstruction core (pass 2) | — | Own `NvtxSpan` type; per-thread Push/Pop stacks + process-wide Start/End matching. |
| Tolerance (unclosed / out-of-order / duplicate ts) | Reconstruction core | — | By construction (D-12); no shared-framework change. |
| Range statistics | Model / query layer | Reconstruction core | Aggregation over completed spans grouped by `(name, domain, category)`. |
| Timeline/view types, HTTP, caching | Serving (Phase 3) | — | Out of scope; core must map cleanly onto timeline views either way (D-16). |
| Thread identity for Push/Pop | **Capture (Phase 1) — MISSING** | — | Not carried today. See Open Question #1. |

---

## Standard Stack

This phase introduces **no new third-party dependencies**. Everything is in-workspace or already in
`[workspace.dependencies]` (root `Cargo.toml`).

### Core
| Crate | Source | Purpose | Why |
|-------|--------|---------|-----|
| `nvtx-events` | `integrations/nvtx/events` (in-repo) | The `NvtxEvent` vocabulary being reconstructed | The input contract. `serde` is a default feature (needed to deserialize captured files). [VERIFIED: `integrations/nvtx/events/src/lib.rs:25`] |
| `nvtx-bridge` | `integrations/nvtx/bridge` (in-repo) | `NvtxEventEntity` newtype (`EntityEvent::NAME = "NvtxEvent"`) | The stream element type the core replays. [VERIFIED: `integrations/nvtx/bridge/src/lib.rs:23-27`] |
| `quent-events` | `crates/events` (in-repo) | `Event<T>` envelope (`id`, `timestamp`, `data`), `EntityEvent` trait | Shared runtime — the only Quent dep D-06 permits besides `nvtx-events`. [VERIFIED: `crates/events/src/lib.rs:12-46`] |
| `quent-time` | `crates/time` (in-repo) | `TimeUnixNanoSec` (= `u64`), `Timestamp` trait, `TimeOrderedCollector` | `quent-events` already depends on it; provides an in-order collector reusable for timestamp sorting (D-12). [VERIFIED: `crates/time/src/lib.rs:34`, and `crates/analyzer/.../TimeOrderedCollector` per NVTX-PHASE2-MAP.md §3] |

### Supporting (already in `[workspace.dependencies]`)
| Crate | Version (workspace) | Purpose | When to Use |
|-------|--------|---------|-------------|
| `uuid` | 1 (v7) | Entity/event ids | Event envelope carries a `Uuid`. [VERIFIED: root `Cargo.toml`] |
| `serde` / `serde_json` | 1 | Deserialize captured event files in tests | Reading back ndjson fixtures. |
| `thiserror` | 2.0.17 | Per-crate error enum (`NvtxModelError` + `Result` alias) | Matches project error convention (CLAUDE.md). [VERIFIED: root `Cargo.toml:141`] |
| `tracing` | 0.1 | `warn!` on tolerated anomalies (orphan pop, unclosed range) | Matches logging convention. |
| `rustc-hash` | 2 | `FxHashMap` for handle tables (perf) | Optional; `std::collections::HashMap` is fine for v1. [VERIFIED: root `Cargo.toml:137`] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Own `NvtxSpan` type (D-08) | `crates/analyzer` `RtFsmBuilder`/`FsmEvents` | REJECTED by D-06/D-08 — drags in `quent-model`'s `RtFsm` trait link and the panic-prone runtime; the whole point is to be off that path. |
| Own timestamp sort | `quent-time::TimeOrderedCollector` | Permitted (quent-time is shared runtime, not legacy). It gives O(1) in-order push, stable duplicate-timestamp ordering, no panic. Recommended over a hand-rolled sort. [VERIFIED: NVTX-PHASE2-MAP.md §3 lines 380-407] |
| `domains/nvtx/` placement | `integrations/nvtx/analyzer` | See "Crate Placement." |

**Installation:** No `cargo add` of external crates. New crate declares path/workspace deps and is
added to root `Cargo.toml` `[workspace] members` **only** (not `default-members`), matching the
existing NVTX crates (root `Cargo.toml:28-31`; `default-members:79-115` excludes them). [VERIFIED]

---

## Package Legitimacy Audit

**No external packages are introduced by this phase.** All dependencies are in-workspace path crates
or already-pinned `[workspace.dependencies]` entries verified present in the repo's root `Cargo.toml`.
slopcheck / registry verification is not applicable — nothing is fetched from crates.io that the
workspace does not already build against.

| Package | Registry | Disposition |
|---------|----------|-------------|
| `nvtx-events`, `nvtx-bridge`, `quent-events`, `quent-time` | in-repo path deps | Approved (workspace members) |
| `uuid`, `serde`, `serde_json`, `thiserror`, `tracing`, `rustc-hash` | already in `[workspace.dependencies]` | Approved (pre-existing) |

**Packages removed due to slopcheck [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

---

## NVTX Event Vocabulary (the input contract)

Ground truth: `integrations/nvtx/events/src/lib.rs` (the enum), `attributes.rs`, `payload.rs`.
[VERIFIED: read in full]

### `NvtxEvent` — 12 variants
| Variant | Fields | Reconstruction role |
|---------|--------|---------------------|
| `RangePush` | `domain: u64`, `attributes: NvtxEventAttributes` | Opens a nested range on the current thread+domain. **Carries no thread id.** |
| `RangePop` | `domain: u64` | Closes the innermost push on this thread+domain. **Carries no thread id, no attributes.** |
| `RangeStart` | `domain: u64`, `range_id: u64`, `attributes` | Opens a process-wide range keyed by `range_id`. |
| `RangeEnd` | `domain: u64`, `range_id: u64` | Closes the matching `RangeStart` by `range_id`. |
| `Mark` | `domain: u64`, `attributes` | Instantaneous marker (no pairing). |
| `DomainCreate` | `domain: u64`, `name: String` | Registers a domain handle → name. |
| `DomainDestroy` | `domain: u64` | Ends a domain's lifespan. |
| `RegisterString` | `domain: u64`, `handle: u64`, `string: String` | Registers a string handle → value, scoped to `domain`. |
| `NameCategory` | `domain: u64`, `category: u32`, `name: String` | Names a `(domain, category)` pair. |
| `NameThread` | `thread_id: u32`, `name: String` | Names an OS thread. **Only place a thread id appears in the vocabulary.** |
| `ResourceCreate` | `domain: u64`, `handle: u64`, `identifier_type: i32`, `identifier: u64`, `message: Option<NvtxMessage>` | Opens a resource lifespan; name may be immediate or a registered handle. |
| `ResourceDestroy` | `handle: u64` | Ends a resource lifespan. **Carries only `handle` — no domain.** |

Default (NULL) domain is `0`. [VERIFIED: `lib.rs:31`, and injection `convert.rs` `mark_a`/`range_push_a` emit `domain: 0`]

### `NvtxEventAttributes` (`attributes.rs:46-57`)
```
category: u32                 // 0 = none; namespaced by (domain, category) in the analyzer
color:    Option<NvtxColor>   // { color_type: i32, value: u32 } raw ARGB
message:  Option<NvtxMessage> // String(String) | RegisteredHandle(u64)
payload:  Option<NvtxPayload> // { payload_type: i32, value: NvtxPayloadValue } — carry verbatim, do not decode (v2)
```
`NvtxMessage` (`attributes.rs:21-28`): `String(String)` (immediate, resolved) or
`RegisteredHandle(u64)` (resolve via `RegisterString` events, keyed by `(domain, handle)`).

### `Event<NvtxEventEntity>` stream shape (`crates/events/src/lib.rs:17-46`)
```
Event { id: Uuid, timestamp: TimeUnixNanoSec (u64), data: NvtxEventEntity(NvtxEvent) }
```
- `id` = the UUID passed to `EventSender::emit(id, event)`. In capture wiring this is the **session
  UUID** (`integrations/nvtx/example/src/lib.rs:34` passes `session` for every event). So **all NVTX
  events in a session share one `id`** — it is a stream id, not a per-range or per-thread id.
  [VERIFIED: `observer.rs:69-71` `emit` → `Event::new_now(id, ..)`; `example/src/lib.rs:34`]
- `timestamp` = capture-time monotonic clock (`quent_time::timestamp()`), global across threads.
- `data` = the verbatim `NvtxEvent`.

---

## Runtime State Inventory

> This is a greenfield reconstruction phase (no rename/migration). The relevant "state" is what the
> input stream does and does **not** carry. Recorded here because it materially bounds what the
> analyzer can reconstruct.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Thread identity on Push/Pop | **NONE.** `RangePush`/`RangePop` carry only `domain`. `Event` envelope carries `id` (=session UUID) + `timestamp` + `data` — no thread field. Injection knows the thread (thread-local `RANGE_DEPTH`, `init.rs:41-51`) but does not stamp it. Not recoverable from captured files (ndjson importer yields only `Event{id,timestamp,data}`, `crates/io/ndjson/src/lib.rs:147`). | **Blocks ANA-03.** Resolve via Open Question #1 (recommend Phase-1 `thread_id` addition) before planning per-thread reconstruction. |
| Range-id space | Process-unique monotonic counter (`init.rs:34-39` `NEXT_HANDLE` starts at 1; domain/string/resource handles + range ids all drawn from it). So `range_id`, domain handles, string handles, resource handles are all globally unique within a session. | Match `RangeStart`/`RangeEnd` by `range_id` alone (safe); match `ResourceCreate`/`ResourceDestroy` by `handle` alone (safe — `ResourceDestroy` lacks domain but handles are globally unique). |
| Domain on `ResourceDestroy` | **NONE** — `ResourceDestroy { handle }` has no domain. | Key resource lifespans by `handle`; recover domain from the paired `ResourceCreate`. |
| Duplicate / out-of-order timestamps | Present in real streams (high-frequency NVTX calls collide at the same ns; `timestamp()` is monotonic but not strictly increasing). | Sort by timestamp with stable tie-breaking (arrival order). See Tolerance. |
| Unclosed ranges at trace end | Present in real streams (project constraint: streams "routinely contain unclosed ranges"). | Close at trace-end timestamp, flag synthetic (D-13). |
| Wide-char / non-ASCII labels | Dropped or lossily captured at Phase 1 (injection `convert.rs` warns once, substitutes U+FFFD; wide-char `*W` stubs drop labels). | Out of scope for Phase 2 — the analyzer sees whatever strings survived capture; no action. |

**The canonical bound:** After replaying every captured `Event<NvtxEventEntity>`, the analyzer knows
each event's `domain`, `timestamp`, and payload — but for Push/Pop it does **not** know which thread
issued it. Everything else needed by MOD-01/02 and ANA-01/02/04/05/06 is present.

---

## Two-Pass Handle Resolution (D-15)

**Pass 1 — build lookup tables** by scanning all events (order-independent; tolerates forward
references per D-15). [VERIFIED against variant fields in `lib.rs`]

| Table | Key | Value | Source event |
|-------|-----|-------|--------------|
| Domain names | `domain: u64` | `String` | `DomainCreate { domain, name }` |
| Domain lifespan | `domain: u64` | `(created_ts, Option<destroyed_ts>)` | `DomainCreate` / `DomainDestroy` |
| Registered strings | `(domain: u64, handle: u64)` | `String` | `RegisterString { domain, handle, string }` |
| Category names | `(domain: u64, category: u32)` | `String` | `NameCategory { domain, category, name }` — **never global** (ANA-02) |
| Thread names | `thread_id: u32` | `String` | `NameThread { thread_id, name }` |
| Resource records | `handle: u64` | `{ domain, identifier_type, identifier, name }` | `ResourceCreate` (+ `ResourceDestroy` closes) |

**Pass 2 — reconstruct entities** using the tables, sorted by timestamp. Resolve each
`NvtxMessage::RegisteredHandle(h)` on an event via the registered-strings table keyed by
`(event.domain, h)`.

### Placeholder policy (D-14) — success criterion 2
Two visible cases, both deterministic and stable across runs:
- **Legitimately unnamed** → clean default label:
  - `domain == 0` and no `DomainCreate` → `"default domain"`.
  - `category == 0` → no category (none), not a placeholder.
  - Thread with no `NameThread` → `"thread {thread_id}"` (once thread id exists).
- **Referenced-but-unresolved** → placeholder that surfaces the raw id:
  - non-zero `domain` with no `DomainCreate` → e.g. `"<domain 0x{domain:X}>"`.
  - `RegisteredHandle(h)` with no matching `RegisterString` → e.g. `"<unregistered string 0x{h:X}>"`.
  - non-zero `category` with no `NameCategory` → e.g. `"<category {category} @ domain 0x{domain:X}>"`.

Placeholders must be pure functions of the raw id (no counters, no timestamps) so success-criterion-2's
"stable placeholders" holds. Test by asserting exact placeholder strings.

---

## Range Reconstruction Mechanics

### Push/Pop — per-thread nested stacks (ANA-03) — **gated on Open Question #1**
Intended: a stack per `(thread_id, domain)`; each `RangePush` pushes, each `RangePop` pops the
innermost, producing a nested span. The injection layer already models nesting per `(thread, domain)`
(`init.rs:41-82` `RANGE_DEPTH`), confirming per-thread+per-domain is the correct grain.

**Blocker:** the stream carries no `thread_id` (see Runtime State Inventory). Consequences:
- For **single-threaded** input (the `nvtx-example` fixture, and hand-crafted synthetic streams that
  model one thread), a single per-domain stack reconstructs correctly.
- For **multi-threaded** input, a global per-domain stack silently mismatches: a `RangePop` from
  thread B pops a `RangePush` from thread A. There is no sound way to separate them from the stream.

Do not plan a "best-effort global stack" as if it satisfies ANA-03 — it does not, and it produces
plausible-but-wrong nesting. Resolve Open Question #1 first.

### RangeStart/RangeEnd — process-wide by handle (ANA-04) — **no gap**
`HashMap<u64 range_id, OpenStartRange>`: `RangeStart` inserts (with `domain`, `attributes`, start ts);
`RangeEnd` removes and forms the span (end ts). Works across threads by construction because the match
key is `range_id`, which is process-globally unique (`init.rs` counter). Orphan `RangeEnd` (no open
start) → `warn!` and skip (tolerant). Unmatched `RangeStart` at trace end → close synthetic (D-13).
[VERIFIED: `lib.rs:48-63`, `convert.rs:55-72`, `init.rs:34-39`]

### Nested-range representation (Claude's Discretion)
Two options; both derivable from the same span set:
- **Flat spans + time-containment** — store every span flat with `(start, end, domain, thread, depth?)`;
  Phase 3 derives nesting by interval containment on the shared time axis. Simplest core; pushes
  layout work to Phase 3. Duplicate/zero-length spans can make containment ambiguous.
- **Explicit parent/child tree** — record parent at pop time (parent = the span now on top of the
  stack). Unambiguous nesting captured at reconstruction; slightly more state. Better matches
  swim-lane rendering (UI-02) and survives duplicate timestamps.

**Recommendation:** capture an explicit `parent: Option<SpanId>` at pop time for Push/Pop ranges
(cheap, computed while the stack is live), and *also* keep flat start/end so Phase 3 can choose. This
is only meaningful once thread stacks exist (Open Question #1). Start/End ranges are typically flat
(not nested) — represent flat.

---

## Model Representation

Define **own plain structs** (D-08), no `RtFsm`, no `quent-model`. Suggested shape:

```
NvtxSpan {
    domain: u64,
    thread_id: Option<u32>,      // None until Open Question #1 resolved; keep the field (D-03)
    name: String,                // resolved message or placeholder
    category: Option<u32>,       // raw id; resolved name via (domain, category) table
    color: Option<NvtxColor>,    // verbatim (UI-03 honors it later)
    payload: Option<NvtxPayload>,// verbatim, undecoded (v2)
    start: TimeUnixNanoSec,
    end: TimeUnixNanoSec,
    kind: PushPop | StartEnd | Resource,
    parent: Option<SpanId>,      // for nested Push/Pop (discretion)
    synthetic_end: bool,         // D-13: closed at trace-end, never observed
}
NvtxMark  { domain, thread_id: Option<u32>, name, category, color, payload, timestamp }
NvtxDomain{ domain, name (resolved/placeholder), created, destroyed: Option<..> }
NvtxThread{ thread_id, name (resolved/"thread {id}") }
NvtxResource = NvtxSpan { kind: Resource, .. } with identifier_type label (D-09/D-11)
```

- **MOD-01:** `NvtxSpan{kind: PushPop|StartEnd}` is the "range = single-state FSM" in shape. A Phase-3
  adapter (if D-16 option A) maps `NvtxSpan` → a 2-transition `RtFsm` (`open`→`exit`) trivially.
- **MOD-02:** domains/threads/categories are first-class (labels + grouping keys); marks are instants.
- **D-03 correlation-readiness:** keep `thread_id`, `domain`, and precise `start`/`end` on every span.

### Resource modeling (D-09/D-10/D-11)
`ResourceCreate → ResourceDestroy` matched by `handle` = a named lifespan span. Fields:
- `identifier_type: i32` → label the **core** NVTX identifier types (a small `match` → static `&str`),
  pass unknown/CUDA-extension types through as raw (e.g. `"<identifier_type {n}>"`). Same core-now /
  extension-deferred line as payloads (D-11). Do **not** fabricate capacity/occupancy (D-10).
- `name` resolved from `message` (immediate `String` or `RegisteredHandle` via the string table).
- Domain grouping recovered from the `ResourceCreate` (since `ResourceDestroy` lacks domain).
- Unclosed resource at trace end → close synthetic (D-13), same as ranges.

The set of "core" `identifier_type` constants is defined by the NVTX headers
(`nvtxResourceGenericType_t`, `nvtxResourceCUDAType_t`, etc.). Phase 1 captures the raw `i32`
verbatim; Phase 2 needs the numeric→label mapping for the *generic/core* set only. **[ASSUMED]** the
exact core enum values — confirm against the vendored NVTX headers used by
`integrations/nvtx/injection/build.rs` bindings before hardcoding (see Open Question #3).

---

## Tolerance by Construction (D-12, D-13) — ANA-05

### Legacy panic paths are genuinely off our path — VERIFIED
The blockers STATE.md flags (`crates/analyzer/src/fsm/runtime.rs`) only fire if we *use* that builder.
We do not: D-06 forbids depending on `crates/analyzer`, and D-08 gives us our own span type. Confirmed
panic points that we avoid by never calling them:
- `RtFsmBuilder::try_build()` — `transitions.last().unwrap()` panics on a zero-transition FSM
  (`runtime.rs`, per NVTX-PHASE2-MAP.md §3 lines 318-344; verified the `try_build` shape at
  `runtime.rs:302+`). **Not on our path** — we never construct an `RtFsmBuilder`.
- `RtFsmsBuilder::try_build()` — the `let fsm = fsm.try_build()?;` at `runtime.rs` propagates the first
  incomplete FSM's error and aborts all. **Not on our path** (VERIFIED by reading `runtime.rs` lines
  ~302-320: the `?` is exactly as documented). We never call it.
- `SpanUnixNanoSec::try_new(t, t)` zero-duration concerns — **not on our path**: we define our own span
  type and choose to allow zero-duration spans (clamp `end = max(end, start)`), so duplicate timestamps
  never error.

**No shared-framework change is required for Phase 2** (D-12). The "fix framework vs synthesize
locally" question dissolves — we synthesize locally in our own core.

### The three tolerance behaviors
1. **Out-of-order events** — sort the replay by `timestamp` before pass 2. Use
   `quent_time::TimeOrderedCollector` (O(1) in-order push, binary-search insert for late arrivals, no
   panic, no data loss) or a stable sort of the collected `Vec<Event>`. [VERIFIED behavior: NVTX-PHASE2-MAP.md §3 lines 380-407]
2. **Duplicate timestamps** — stable ordering preserves arrival order; zero-length spans allowed
   (clamp). Never panic. Test: two events at identical ns reconstruct deterministically.
3. **Unclosed ranges at trace end** — after replay, any span still open (Push with no Pop; Start with
   no End; Resource with no Destroy) is closed at the max observed timestamp (trace end) with
   `synthetic_end = true` (D-13). `warn!` each. Orphan `Pop`/`End`/`Destroy` (no matching open) →
   `warn!` and skip.

**Success criterion 3** ("a stream containing unclosed ranges, out-of-order events, and duplicate
timestamps analyzes to completion with no panic or abort") is met entirely inside our core.

---

## Range Statistics (ANA-06)

Aggregate over **completed** spans (including synthetically closed ones — but consider tagging their
contribution, since synthetic durations are inferred). Group key: `(resolved name, domain, category)`.
Per group compute: `count`, `total_duration`, `avg = total/count`, `min`, `max`. Duration =
`end - start` in ns (`u64`). Guard `count == 0` for avg. Zero-duration spans contribute `0`.

This is a straightforward fold over the span set — no library needed. Expose as a
`Vec<RangeStats>` or a `HashMap<StatsKey, RangeStats>` on the built model. Whether marks or resources
participate in "range statistics" is a modeling choice; the requirement says "range" → include
Push/Pop and Start/End spans; exclude marks (instants) and resources (or report separately).

---

## Crate Placement (Claude's Discretion — recommendation)

**Recommend: `integrations/nvtx/analyzer`** (sibling to `bridge`, `example`), workspace `members` only.

Rationale:
- The core is Quent-side (consumes `Event<NvtxEventEntity>`, depends on `quent-events`), so it does
  NOT belong in the upstreamable `events`/`injection` set — but `bridge` and `example` are *already*
  Quent-coupled crates living under `integrations/nvtx/`. Keeping the analyzer there keeps the entire
  NVTX vertical in one directory and mirrors the "sibling to bridge" lean in CONTEXT.md.
- `domains/nvtx/` (the placement suggested by NVTX-PHASE2-MAP.md §6) mirrors `domains/query_engine/`,
  but that domain is built on the rejected `model!`/`fsm!`/`entity!` DSL and depends on
  `crates/analyzer`/`crates/model`. Adopting its directory shape invites adopting its framework
  coupling, which D-05/D-06 forbid. `domains/` also implies a full model+analyzer+server+ui quartet;
  Phase 2 ships only reconstruction.
- Register in `[workspace] members` only (like the other NVTX crates, root `Cargo.toml:28-31`), NOT in
  `default-members` (`:79-115`), preserving the zero-cost-default guarantee.

Trade-off to note for the planner: if Phase 3 chooses D-16 option A (adapt onto legacy `crates/ui`),
a Phase-3 serving crate may still land under `domains/` or alongside the simulator server; that does
not force the Phase-2 core there. Keep the core in `integrations/nvtx/analyzer` regardless.

Suggested module layout (own types, no DSL):
```
integrations/nvtx/analyzer/
├── Cargo.toml            # members-only; deps: nvtx-events, nvtx-bridge, quent-events, quent-time,
│                         #                     uuid, serde, thiserror, tracing, (rustc-hash)
└── src/
    ├── lib.rs            # crate //! doc + re-exports; NvtxModel query surface
    ├── error.rs          # NvtxModelError + NvtxModelResult (thiserror)
    ├── model.rs          # NvtxModel (built) + NvtxModelBuilder (two-pass)
    ├── tables.rs         # pass-1 handle resolution tables + placeholder policy
    ├── span.rs           # NvtxSpan, NvtxMark, NvtxResource, SpanKind
    ├── ranges.rs         # push/pop stack + start/end matching (thread grain per OQ#1)
    ├── resource.rs       # resource lifespan + identifier_type labels
    └── stats.rs          # ANA-06 aggregation
```

---

## Test Fixture Strategy (Claude's Discretion)

- **Happy path (real capture)** — reuse `nvtx_example::run_capture` with a collecting `EventCallback`
  (exactly the pattern in `integrations/nvtx/example/tests/capture.rs:36-68`), or capture to ndjson and
  read back via `NdjsonImporter` (`crates/io/ndjson/src/lib.rs:117`). Exercises NameThread, Mark,
  Push/Pop, Start/End on the default domain — single-threaded, so Push/Pop reconstruction is valid
  even before OQ#1 is resolved.
- **Malformed (synthetic)** — hand-build `Vec<Event<NvtxEventEntity>>` with `Event::new(id, ts, data)`
  (`crates/events/src/lib.rs:38`) so you control `timestamp` exactly. Cover:
  - unclosed Push (no Pop) → synthetic close + flag;
  - orphan Pop (no Push) → warn + skip;
  - unmatched RangeStart / orphan RangeEnd;
  - out-of-order (later ts emitted before earlier) → correct sorted reconstruction;
  - duplicate timestamps (two events, identical ns) → deterministic, no panic;
  - referenced-but-unresolved domain/string/category → exact placeholder strings;
  - forward reference (use before `RegisterString`/`DomainCreate`) → resolved by pass 1.
- **Multi-thread Push/Pop** — cannot be written as a valid fixture until OQ#1 resolves (no thread field
  to populate). Flag as pending on the vocabulary decision.

Because the malformed conditions "don't occur naturally," synthetic fixtures are the primary evidence
for ANA-05 and success-criterion 3.

---

## Common Pitfalls

### Pitfall 1: Assuming a global Push/Pop stack satisfies ANA-03
**What goes wrong:** With no thread id, a single per-domain stack matches pops to the wrong thread's
pushes under concurrency, yielding confidently-wrong nesting.
**Why:** The stream serializes all threads' calls into one timestamp-ordered sequence with no thread
tag (verified: no thread field on events or envelope).
**How to avoid:** Treat ANA-03 as blocked on Open Question #1. Do not ship a global stack as ANA-03.
**Warning signs:** Tests only cover single-thread fixtures; nesting "works" but was never tested with
interleaved threads.

### Pitfall 2: Namespacing categories or strings globally
**What goes wrong:** Category 7 in domain A collides with category 7 in domain B; registered handle
collisions across domains.
**Why:** NVTX scopes categories and registered strings **per domain**.
**How to avoid:** Key category table by `(domain, category)` and string table by `(domain, handle)`
(ANA-02, ANA-01). The vocabulary already carries `domain` on `NameCategory`/`RegisterString`.
**Warning signs:** Table keyed by a bare `u32`/`u64`.

### Pitfall 3: Reaching for `crates/analyzer` builders "to save work"
**What goes wrong:** Reintroduces the exact panic paths STATE.md flags and the `quent-model` `RtFsm`
trait link D-08 exists to avoid.
**How to avoid:** Own span type + own two-pass builder. `crates/analyzer` must not appear in the new
crate's `Cargo.toml`.
**Warning signs:** `use quent_analyzer::...` or `RtFsmBuilder` anywhere in the NVTX core.

### Pitfall 4: Matching resources by `(domain, handle)`
**What goes wrong:** `ResourceDestroy` has no `domain` — a `(domain, handle)` key never matches the
destroy event, so every resource looks unclosed.
**Why:** `ResourceDestroy { handle }` (verified `lib.rs:121-125`); handles are process-globally unique.
**How to avoid:** Match resources by `handle` alone; recover domain from the create.

### Pitfall 5: Decoding payloads or extension events
**What goes wrong:** Scope creep into v2 (PAY-01/02).
**How to avoid:** Carry `NvtxPayload` verbatim on spans/marks; ignore `PayloadExtensionEvent` (not
wired into `NvtxEvent`, no capture emits it — `payload.rs:59-93`).

---

## State of the Art

| Old Approach (rejected) | Current Approach (this phase) | Why |
|-------------------------|-------------------------------|-----|
| Range = single-state legacy FSM via `crates/analyzer` `RtFsm` | Range = own plain `NvtxSpan` | D-08: avoids `quent-model` trait link + panic-prone runtime |
| Declare NVTX as a `schema::Schema` / `model!` DSL | Hand-written enum + hand-written reconstruction | D-05/D-07: NVTX is a foreign stream; nothing analyzes a Schema anyway (D-04) |
| Capture-time handle resolution | Resolve in the analyzer from the stream | Project constraint (REQUIREMENTS "Out of Scope"): capture-time resolution races |
| Strict validation (reject malformed) | Tolerance by construction | REQUIREMENTS "Out of Scope": strictness reproduces the analyzer panics this project eliminates |

**Deprecated/outdated in the maps:** NVTX-PHASE2-MAP.md §§1–3 and §6 recommend depending on
`crates/analyzer`/`quent-model`/`RtFsmBuilder` and placing the crate under `domains/nvtx/`. These
predate D-05/D-06/D-08 and are explicitly superseded — treat them as "what NOT to depend on"
(as CONTEXT.md's canonical-refs caveat states). Their §3/§4 factual content (panic locations,
vocabulary tables, `TimeOrderedCollector` behavior) remains accurate and is reused above.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The exact numeric values of the "core" `identifier_type` set (for D-11 labels) | Resource modeling | Wrong labels on resources; low blast radius (labels only). Mitigate by reading vendored NVTX headers (OQ#3). |
| A2 | libcudf/cuCascade are multi-threaded (making the thread-id gap material for the primary target) | Summary / OQ#1 | If they were single-threaded per domain, ANA-03 could ship without the gap. Low probability — GPU data libs are concurrent by design. Confirm with owner. |
| A3 | `quent_time::TimeOrderedCollector` is importable without pulling a legacy-analyzer dep | Tolerance | If it lives in `crates/analyzer` rather than `crates/time`, we cannot use it under D-06. NVTX-PHASE2-MAP.md §3 cites `crates/time/src/lib.rs:121-165` — verify the path before depending on it; a hand-rolled stable sort is a trivial fallback. |

---

## Open Questions (RESOLVED)

> **OQ#1 — RESOLVED by locked decision D-17** (CONTEXT.md): fold a Wave-0 capture-side
> `thread_id: u32` addition to `RangePush`/`RangePop` into Phase 2, then reconstruct ANA-03
> per-thread stacks on it. Implemented by plans `02-01` (capture) + `02-04` (reconstruct).
> **OQ#2 — RESOLVED (planner's call, D-17-delegated):** synthetically-closed spans ARE included
> in statistics with a `synthetic_end` flag on the span and a synthetic-count tracked per stats
> group — handled in plan `02-05`.
> **OQ#3 — RESOLVED-in-plan:** core vs extension `identifier_type` values are confirmed against the
> vendored NVTX headers at execution time in plan `02-05` Task 1, with a safe raw pass-through
> fallback (D-11). No success criterion depends on the exact core labels.

1. **[BLOCKING → RESOLVED by D-17] How does ANA-03 obtain thread identity?** *(highest priority — gates ANA-03 and success-criterion 1)*
   - **What we know (VERIFIED):** No thread id exists on `RangePush`/`RangePop` (`lib.rs:37-47`), on the
     `Event` envelope (`id` = session UUID, `crates/events/src/lib.rs:19-26`; `example/src/lib.rs:34`),
     or in captured files (ndjson yields only `Event{id,timestamp,data}`). The injection layer knows
     the thread (`init.rs:41-51` thread-local) but discards it before emit. `NameThread` is the only
     place a `thread_id: u32` appears.
   - **What's unclear:** Whether the owner accepts a small Phase-1 vocabulary addition, or wants ANA-03
     scoped/reframed.
   - **Recommendation:** Add `thread_id: u32` to `RangePush` and `RangePop` (and consider `Mark`,
     `RangeStart` for grouping/labeling by named thread). The injection callbacks already run on the app
     thread — capture the OS thread id there (Linux `gettid()` / `libc::pthread_self`-style), the same
     id space `NameThread` uses, so named-thread lanes resolve directly. This is a capture-side
     (Phase 1) change with a narrow blast radius (2–4 variants + convert/callbacks), and it makes ANA-03
     and named-thread swim lanes (UI-03) achievable. Alternative (larger blast radius): add thread id to
     the `Event<T>` envelope in `quent-events` — affects every domain, not recommended for an
     NVTX-specific need. **Escalate to discuss-phase before planning ANA-03.** All other requirements
     proceed independently.

2. **Do statistics and nesting include synthetically-closed spans, and how are they marked?**
   - What we know: D-13 says flag them; ANA-06 says compute stats per range.
   - Recommendation: include them in stats but keep the `synthetic_end` flag on the span so Phase 3 can
     render/label distinctly; consider reporting a synthetic-count per stats group. Planner's call.

3. **Which `identifier_type` values are "core" (labeled) vs "extension" (raw pass-through)?**
   - What we know: D-11 says core-now/extension-deferred; Phase 1 captures the raw `i32`.
   - Recommendation: read the vendored NVTX headers referenced by
     `integrations/nvtx/injection/build.rs`/bindgen to enumerate the generic/core resource-type enum,
     label those, pass the rest through raw. Resolves A1.

---

## Environment Availability

Pure Rust/Cargo workspace phase; no external services, GPU, or network dependencies.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | building the crate | ✓ | edition 2024, >= 1.93 (pixi pins >=1.96) | — |
| cargo (workspace resolver 3) | build/test | ✓ | — | — |
| In-workspace crates (nvtx-events/bridge, quent-events/time, io) | the core + tests | ✓ | in-repo | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none. (GPU is explicitly not required — VAL-01 mandates
GPU-free CI; malformed fixtures are synthetic.)

---

## Validation Architecture

> `workflow.nyquist_validation` is `true` in `.planning/config.json` → section included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` (workspace convention; CLAUDE.md) |
| Config file | none — `#[cfg(test)] mod tests` per source file + `tests/` integration dir (mirrors `integrations/nvtx/example/tests/capture.rs`) |
| Quick run command | `cargo test -p nvtx-analyzer` (crate not in default-members → must pass `-p`) |
| Full suite command | `cargo test -p nvtx-analyzer --all-features` (plus workspace clippy/fmt gates) |
| Gate commands | `cargo clippy -p nvtx-analyzer --all-targets --all-features -- -D warnings`; `cargo fmt --check` |

Note: because NVTX crates are `members`-only (not `default-members`), bare `cargo test` skips them —
CI and local runs MUST use `-p nvtx-analyzer` (or `--workspace`). [VERIFIED: root `Cargo.toml:28-31,79-115`]

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ANA-01 | Registered handle → string, keyed by `(domain,handle)` | unit | `cargo test -p nvtx-analyzer resolve_registered_string` | ❌ Wave 0 |
| ANA-02 | Domain + category names, `(domain,category)` namespacing (no global collision) | unit | `cargo test -p nvtx-analyzer category_namespaced_by_domain` | ❌ Wave 0 |
| ANA-02/D-14 | Stable placeholders for unresolved domain/string/category (exact strings) | unit | `cargo test -p nvtx-analyzer placeholder_stable` | ❌ Wave 0 |
| ANA-03 | Per-thread nested Push/Pop stacks | unit | `cargo test -p nvtx-analyzer pushpop_nested_per_thread` | ❌ **Blocked on OQ#1** |
| ANA-03 | Single-thread Push/Pop nesting (interim, from real capture) | integration | `cargo test -p nvtx-analyzer pushpop_single_thread` | ❌ Wave 0 |
| ANA-04 | RangeStart/End matched by handle across threads | unit | `cargo test -p nvtx-analyzer startend_match_by_handle` | ❌ Wave 0 |
| ANA-05 | Unclosed range → synthetic close + flag; no panic | unit | `cargo test -p nvtx-analyzer unclosed_closed_at_trace_end` | ❌ Wave 0 |
| ANA-05 | Out-of-order events → correct sorted reconstruction | unit | `cargo test -p nvtx-analyzer out_of_order_sorted` | ❌ Wave 0 |
| ANA-05 | Duplicate timestamps → deterministic, no panic | unit | `cargo test -p nvtx-analyzer duplicate_timestamps_no_panic` | ❌ Wave 0 |
| ANA-06 | count/total/avg/min/max per (name,domain,category) | unit | `cargo test -p nvtx-analyzer range_statistics` | ❌ Wave 0 |
| MOD-01 | Range materializes as an `NvtxSpan` (start/end interval) | unit | `cargo test -p nvtx-analyzer range_is_span` | ❌ Wave 0 |
| MOD-02 | Marks/domains/threads/categories present in model | unit | `cargo test -p nvtx-analyzer model_surface_present` | ❌ Wave 0 |
| D-09 | Resource lifespan (`Create→Destroy` by handle) + identifier_type label | unit | `cargo test -p nvtx-analyzer resource_lifespan` | ❌ Wave 0 |
| criterion 3 | Full malformed stream analyzes to completion, no panic/abort | integration | `cargo test -p nvtx-analyzer malformed_stream_completes` | ❌ Wave 0 |
| happy path | Real `nvtx-example` capture reconstructs expected labels | integration | `cargo test -p nvtx-analyzer example_capture_roundtrip` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p nvtx-analyzer` + `cargo clippy -p nvtx-analyzer --all-targets --all-features -- -D warnings`
- **Per wave merge:** `cargo test -p nvtx-analyzer --all-features` + `cargo fmt --check`
- **Phase gate:** all above green before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] New crate `integrations/nvtx/analyzer` (Cargo.toml + `src/lib.rs`) — registered in `members` only.
- [ ] `tests/` integration file mirroring `example/tests/capture.rs` (real-capture happy path).
- [ ] Synthetic-fixture helper: build `Vec<Event<NvtxEventEntity>>` with explicit timestamps via
      `Event::new(id, ts, NvtxEventEntity(..))` — needed for every malformed/tolerance test.
- [ ] Placeholder-string constants + exact-match assertions (success-criterion 2 stability).
- [ ] **Decision needed before ANA-03 tests:** thread-id source (OQ#1). Multi-thread fixtures cannot be
      authored until the vocabulary carries a thread field.

---

## Sources

### Primary (HIGH confidence — read in full this session)
- `integrations/nvtx/events/src/lib.rs` — `NvtxEvent` 12 variants
- `integrations/nvtx/events/src/attributes.rs` — `NvtxEventAttributes`, `NvtxMessage`, `NvtxColor`
- `integrations/nvtx/events/src/payload.rs` — `NvtxPayload`, `PayloadExtensionEvent` (deferred)
- `integrations/nvtx/bridge/src/lib.rs` — `NvtxEventEntity` (`NAME = "NvtxEvent"`)
- `integrations/nvtx/example/src/lib.rs` + `tests/capture.rs` — capture wiring + fixture pattern
- `integrations/nvtx/injection/src/{convert.rs,callbacks.rs,init.rs}` — confirms no thread id is
  stamped; per-thread `RANGE_DEPTH` is discarded before emit; handle/id counter semantics
- `crates/events/src/lib.rs` — `Event<T>` envelope (`id`/`timestamp`/`data`), `EntityEvent`
- `crates/io/ndjson/src/lib.rs` — importer yields `Event<T>` only (thread id unrecoverable from files)
- `crates/time/src/lib.rs` — `TimeUnixNanoSec = u64`, `Timestamp`
- root `Cargo.toml` — NVTX crates in `members` (28-31), excluded from `default-members` (79-115); workspace deps

### Secondary (MEDIUM — repo analysis docs, cross-checked against source)
- `.planning/codebase/NVTX-PHASE2-CURRENT-DESIGN.md` — producer-only verdict (D-04), read intro/Q1-Q2
- `.planning/codebase/NVTX-PHASE2-MAP.md` — §3 panic locations + `TimeOrderedCollector` behavior, §4
  vocabulary tables (§§1-3,6 superseded by D-05/D-06/D-08 per CONTEXT caveat)
- `crates/analyzer/src/fsm/runtime.rs` — confirmed `RtFsmsBuilder::try_build` `?`-abort shape

### Tertiary (LOW / ASSUMED — needs confirmation)
- Exact core `identifier_type` enum values (A1/OQ#3) — from vendored NVTX headers, not yet read
- libcudf/cuCascade multi-threadedness (A2) — training knowledge; confirm with owner

---

## Metadata

**Confidence breakdown:**
- NVTX vocabulary / stream shape: **HIGH** — read every relevant source file this session.
- Thread-identity gap (ANA-03 blocker): **HIGH** — verified absence across event variants, envelope,
  emit path, and file importer.
- Tolerance / legacy-off-path: **HIGH** — dependency boundary is D-06-enforced; panic paths verified
  in `runtime.rs` and confirmed uncalled.
- Handle resolution / namespacing / resources / stats: **HIGH** — directly grounded in variant fields.
- Crate placement: **MEDIUM** — a recommendation within Claude's Discretion.
- `TimeOrderedCollector` import path: **MEDIUM** — cited to `crates/time` by the map; verify before use.

**Research date:** 2026-07-23
**Valid until:** ~2026-08-22 (stable — in-repo vocabulary; only churns if Phase-1 crates change, e.g.
the recommended thread-id addition).
