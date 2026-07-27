# Phase 2: NVTX Model & Tolerant Analyzer - Context

**Gathered:** 2026-07-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Reconstruct the raw, verbatim `NvtxEvent` stream captured in Phase 1 into a
structured, in-memory **NVTX model** — labeled ranges, marks, domains, threads,
categories, and resources — tolerating the malformed and out-of-order telemetry
real NVTX streams contain, without panicking.

**In scope:**
- A **framework-free, hand-written reconstruction core** that replays the flat
  `Event<NvtxEventEntity>` stream and produces an in-memory NVTX model.
- Handle resolution (domains, registered strings, categories, thread names,
  resource names) from the event stream, with correct namespacing.
- Push/Pop per-thread nested reconstruction; RangeStart/End process-wide matching
  by handle across threads.
- Tolerance **by construction**: close ranges open at end-of-trace, sort
  out-of-order events, handle duplicate timestamps — no panic, no abort.
- Range statistics (count, total/avg/min/max duration) per name/domain/category.
- Modeling the full explicit NVTX surface: ranges + resources as spans, marks as
  instants, domains/threads/categories as resolved labels/grouping.

**Out of scope (later phases / deferred):**
- HTTP endpoints and UI rendering (Phase 3).
- The Phase 3 **serving-side foundation decision** (adapt to legacy analyzer/UI
  view types vs. build a new consumer vertical) — flagged, decided at Phase 3.
- Fan-out mediator / coexistence (Phase 4); real-GPU-workload validation (Phase 5).
- Payload **extension** decode (v2, PAY-01/02); core payload union is carried
  verbatim on events but not decoded here.
- **Inferring Quent capacity/occupancy/utilization semantics** from NVTX ranges or
  resources (heuristic; deferred with operator-correlation).
- **Operator correlation** (NVTX ranges attached to query-engine operators) — v2
  cherry-on-top (COR-01).
</domain>

<decisions>
## Implementation Decisions

### Domain Framing (resolved during discussion)
- **D-01: NVTX is a self-standing telemetry stream, rendered on the shared
  timeline alongside other Quent telemetry — NOT merged into it.** The primary
  targets (libcudf, cuCascade) emit **only** NVTX and carry zero query-engine
  telemetry, so there is nothing to merge into for them; NVTX ranges *are* the
  whole trace. Sirius is a secondary target that also has query-engine telemetry;
  when both are present, both render on the same time axis. The NVTX model must
  therefore **stand entirely on its own** — it cannot require an `Engine` root,
  operators, or any query-engine entity to exist.
- **D-02: "Where the NVTX lane docks" (next to operators vs. a top block) is a
  Phase 3 UI-layout concern, not a Phase 2 concern.** This is pure rendering
  ("visual co-location on a shared axis") and requires no data-level linkage.
- **D-03: True semantic correlation (a range recorded as a child of a specific
  operator) is deferred to v2 (COR-01).** It requires a fragile matching heuristic
  (time-containment breaks on GPU async; thread-affinity breaks on async launch;
  explicit correlation IDs require app cooperation libcudf won't give), only pays
  off in the instrumented-app case, and has no target for libcudf. Design the
  Phase 2 model so it does **not wall off** a later correlation pass (keep per-range
  thread/domain/precise timestamps) — but do not build correlation in v1.

### Foundation — Current-Design vs Legacy (the central investigation outcome)
- **D-04: The current (#191 YAML/schema) design is producer-only — it has NO
  consumer/analyzer/reconstruction/serving crate.** Verified with evidence (see
  `.planning/codebase/NVTX-PHASE2-CURRENT-DESIGN.md`). `schema`/`fsm`/`resource`/
  `yaml`/`instrumentation-build` declare an event model → validate → generate
  *producer* instrumentation code, and stop at codegen. Nothing consumes a stream
  back into a model on that design.
- **D-05: The "legacy" label conflates two separable things.** (1) The declaration
  DSL — `model!`/`fsm!`/`entity!` macros (`crates/model`) — is what YAML replaces
  and what the owner rejected; **NVTX does not need it at all** (NVTX is captured
  from a foreign library; there is no instrumentation to generate). (2) The
  reconstruction + UI-view-type framework (`crates/analyzer` + `crates/ui`) is
  **not** being replaced, has no successor, and is the only thing that renders.
  Avoiding (1) is fully compatible with (2). NVTX never touches the macro DSL.
- **D-06: Phase 2 = a framework-free, hand-written reconstruction core.** It
  depends only on the shared runtime (`quent-events`) + `nvtx-events` — **no
  proc-macros, no `schema::Schema`, no legacy `crates/analyzer` dependency.** This
  is genuinely off-legacy and off the rejected DSL.
- **D-07: Do NOT declare NVTX as a `schema::Schema`.** `NvtxEvent` is a hand-written
  enum captured from a foreign library; a Schema is a *producer* artifact for
  generating instrumentation, and nothing analyzes against a Schema anyway. Forcing
  NVTX into a Schema buys nothing and links the wrong vertical.

### Model Representation
- **D-08: A range is a plain span (start/end interval) — drop the "single-state
  FSM" framing.** A single-state FSM (`open → exit`) *is* a span; the FSM framing
  was only ever a trick to reuse the legacy analyzer's rendering. Since the Phase 2
  core is framework-free, define **our own plain span type** (avoids the transitive
  `quent-model` trait link that `RtFsm` drags in). It is isomorphic to a
  single-state FSM, so a Phase 3 adapter can map it onto the legacy analyzer
  trivially if that path is chosen.
- **D-09: Model the full explicit NVTX surface — everything NVTX provides.**
  Domains, threads, ranges (spans), marks (instants), categories (labels namespaced
  by `(domain, categoryId)`), **and resources**. Resources are modeled as **named
  lifespans** (`ResourceCreate → ResourceDestroy` span + resolved name +
  `identifier_type` label + domain grouping) — structurally just another span.
  (This supersedes an earlier "defer resources" lean: NVTX has no Quent-style
  capacity concept, so a user expressing a resource lifespan does it through NVTX's
  ranges / resource-naming API — that signal is worth reconstructing.)
- **D-10: Model NVTX resources as what NVTX says they are (named object lifespans),
  NOT as Quent capacity-resources.** Do not fabricate capacity/occupancy/utilization
  semantics NVTX does not carry; translating NVTX signal into Quent's resource-
  utilization abstraction is a heuristic, deferred with correlation (D-03).
- **D-11: Core resource types now, extension types later.** `identifier_type` gets a
  label for the core/generic NVTX resource types; unknown / CUDA-specific extension
  types pass through raw. Same "core-now, extension-deferred" line as payloads (D-12
  from Phase 1).

### Tolerance (ANA-05)
- **D-12: Tolerance is handled by construction inside our own reconstruction core.**
  Because Phase 2 owns reconstruction (framework-free), the panic-prone legacy
  `crates/analyzer/src/fsm/runtime.rs` (zero-transition `unwrap()` panic; the
  `RtFsmsBuilder` `?`-abort on the first incomplete FSM) is simply **not on our
  path**. We: close ranges open at end-of-trace (at trace-end timestamp), sort by
  timestamp, and handle duplicate timestamps ourselves. No shared-framework change
  is required for Phase 2; the "fix framework vs synthesize locally" question
  dissolves.
- **D-13: Flag synthetically-closed ranges** (ranges closed at trace-end because
  they were never popped) so Phase 3 can render them distinctly. Cheap; honest about
  observed-vs-inferred.

### Unresolved-Handle Policy (Phase 2 success criterion 2)
- **D-14: Stable placeholders — keep unresolved things visible, never drop.** A range
  referencing a handle that never got a create/register event renders with a
  deterministic placeholder that surfaces the raw id (e.g. `<domain 0xAB>`,
  `<unregistered string 0xCD>`). Distinguish two cases, both visible: *legitimately
  unnamed* (default domain `0`, unnamed thread → clean default label like
  `default domain` / `thread {id}`) vs *referenced-but-unresolved* (non-zero handle
  with no registration → placeholder exposing the raw id).
- **D-15: Two-pass reconstruction for handle resolution.** Registration events
  (`DomainCreate`, `RegisterString`, `NameCategory`, `NameThread`) are not guaranteed
  to precede their uses. First pass builds all lookup tables; second pass reconstructs
  entities. (Handles may also be genuinely unregistered per D-14.)

### Thread Identity for Push/Pop (resolved post-research — Open Question #1)
- **D-17: Fold a Phase-1 capture-side `thread_id` addition into Phase 2 (Wave 0), then
  reconstruct per-thread ANA-03 stacks on it.** Phase-2 research (`02-RESEARCH.md`, Open
  Question #1) verified the captured stream carries **no thread identity**: `RangePush`/
  `RangePop` carry only `domain` (`integrations/nvtx/events/src/lib.rs:37-47`), the `Event`
  envelope carries only session-UUID `id` + `timestamp` + `data`, and captured files are
  unrecoverable. The injection callbacks already run on the app thread (thread-local
  `RANGE_DEPTH`, `init.rs:41-51`) but discard the thread before emit. Without it,
  Success Criterion 1 / **ANA-03** ("Pop matches the most recent Push on the same thread")
  is **not satisfiable for multi-threaded producers** (libcudf/cuCascade). **Resolution
  (owner decision):** add `thread_id: u32` to `RangePush` and `RangePop` at capture time,
  stamping the OS thread id in the same id space `NameThread` uses (Linux `gettid()`-style),
  so per-thread stacks *and* named-thread grouping resolve. A Wave-0 plan makes this narrow
  capture change (2–4 event variants + convert/callbacks) as a prerequisite; ANA-03 is then
  planned fully on real per-thread `(thread_id, domain)` stacks. Do NOT ship a global/best-
  effort stack as ANA-03 (research warns it produces plausible-but-wrong nesting). The
  `thread_id` field stays on the span (D-03 correlation-readiness). Consider extending
  `thread_id` to `Mark`/`RangeStart` for named-thread grouping (planner's call).

### Phase 3 Serving Decision (flagged, deferred to Phase 3)
- **D-16: The legacy-vs-new tension bites at the *serving/UI* seam, not in Phase 2.**
  To render NVTX in the existing Quent UI, something must speak `crates/ui`'s ts-rs
  view types / the legacy `UiAnalyzer` contract — the only renderer that exists.
  Two options, decided at Phase 3:
  - **(A)** Adapt the Phase-2 core onto the legacy `crates/analyzer` + `crates/ui`
    view types (renders today; touches the framework — but not the rejected macros).
  - **(B)** Start a new-design consumer vertical (a hand-written new-design analysis
    runtime, eventually an **`analyzer-build`** schema-driven consumer generator
    symmetric to `instrumentation-build`, plus new view-type generation + server).
    Principled and off-legacy, but large greenfield with no precedent — and it would
    **not** serve NVTX directly anyway (NVTX is schema-orthogonal, so an
    `analyzer-build`-generated analyzer wouldn't cover it; NVTX stays hand-written
    either way).
  Owner lean during discussion: keep NVTX v1 lean → (A) most likely at Phase 3, with
  (B)/`analyzer-build` recorded as a separate future initiative. **Not decided yet.**
  Design the Phase 2 core so spans/marks map cleanly onto timeline view types either
  way.

### Claude's Discretion (handed to planner/researcher)
- **Crate placement** — the reconstruction core is Quent-side (consumes
  `Event<NvtxEventEntity>`), so it does NOT belong in the upstreamable
  `integrations/nvtx/{events,injection}` set. Lean: `integrations/nvtx/analyzer`
  (sibling to `bridge`). Planner finalizes vs. a `domains/nvtx/` placement.
- **Nested-range representation** — flat spans + time-containment vs. an explicit
  parent/child tree. Derivable either way; planner picks based on what Phase 3
  rendering wants.
- **Test fixtures** — real capture from `nvtx-example` for the happy path + hand-
  crafted synthetic `Event<NvtxEventEntity>` streams for the malformed cases
  (unclosed ranges, out-of-order, duplicate timestamps — which don't occur naturally).
- **Reconstruction strategy** — batch / two-pass (D-15) is the natural shape given
  trace-end closing and forward-reference tolerance; planner details.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Planning Source-of-Truth
- `.planning/ROADMAP.md` §"Phase 2: NVTX Model & Tolerant Analyzer" — goal + 5
  success criteria (esp. criterion 2: "stable placeholders when a handle is
  unresolved"; criterion 3: tolerance without panic/abort).
- `.planning/REQUIREMENTS.md` — MOD-01, MOD-02, ANA-01..06 (Phase 2 requirements);
  note the "Out of Scope" table (capture-time handle resolution, strict validation)
  and v2 items PAY-01/02, COR-01.
- `.planning/PROJECT.md` — locked constraints (Rust-first, separability, one
  injection slot per process) + Key Decisions table.
- `.planning/phases/01-capture-foundation/01-CONTEXT.md` — Phase 1 decisions;
  esp. D-12 (payload extension deferred / core-now) which this phase mirrors for
  resource identifier types (D-11).

### Codebase Maps (READ THESE — one supersedes the other on foundation)
- `.planning/codebase/NVTX-PHASE2-CURRENT-DESIGN.md` — **authoritative on the
  foundation decision.** Evidence that the current #191 design is producer-only,
  the legacy boundary, why NVTX shouldn't be a Schema, and the (a)+(c) hybrid
  recommendation. Read before choosing any framework dependency.
- `.planning/codebase/NVTX-PHASE2-MAP.md` — useful for the NVTX event vocabulary
  (§4: `NvtxEvent` variants, `NvtxEventAttributes`, handle-resolution tables) and
  reconstruction mechanics. **CAVEAT:** its Sections 1–3 ground the model on the
  legacy `model!`/`fsm!`/`entity!` DSL and `crates/analyzer`, which the owner
  rejected (see D-05/D-06); treat those sections as background on what NOT to
  depend on, not as the plan.
- `.planning/codebase/CURRENT-CRATES-MAP.md` — the current vs legacy crate roster;
  `Observer::sender()` and the `nvtx-bridge`/`nvtx-example` entries (PR #402).

### Phase 1 Artifacts To Consume
- `integrations/nvtx/events/src/{lib,attributes,payload}.rs` — the `NvtxEvent`
  vocabulary this phase reconstructs (12 variants; raw integer handles;
  `NvtxMessage::{String, RegisteredHandle}`; `NvtxEventAttributes`).
- `integrations/nvtx/bridge/src/lib.rs` — `NvtxEventEntity` newtype (`EntityEvent`
  NAME = `"NvtxEvent"`); the stream shape the core replays.
- `integrations/nvtx/example/src/lib.rs` — capture wiring; source of real
  happy-path test fixtures.

### External Domain Background
- NVTX resource-naming API (`nvtxDomainResourceCreate`/`Destroy`,
  `nvtxResourceAttributes_t`, `identifierType`) — for D-09/D-10/D-11 resource
  modeling; core vs extension (CUDA) resource-type namespaces.
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Event<T>` / `Event::new_now` (`crates/events`): the envelope the core replays;
  carries the capture-time timestamp reconstruction relies on. Both NVTX capture
  and (if Phase 3 chooses A) the legacy analyzer sit on this same envelope.
- `NvtxEvent` + `NvtxEventEntity` (`integrations/nvtx/{events,bridge}`): the input
  vocabulary — already complete; Phase 2 adds no capture code.
- io importers (`crates/io/{ndjson,...}`): yield `Iterator<Item = Event<T>>` — the
  form the reconstruction core consumes when reading back a captured file for tests.

### Established Patterns
- SPDX headers on every source file; `cargo clippy -D warnings` + `cargo fmt --check`
  are CI gates.
- NVTX crates register in workspace `members` only (not `default-members`) — the
  reconstruction crate follows suit.
- "Core-now, extension-deferred" is an established line (payloads D-12); reused for
  resource identifier types (D-11).

### Integration Points
- Input: the `Event<NvtxEventEntity>` stream (live via `Observer`, or replayed from
  a captured file via an importer for tests).
- Output: an in-memory NVTX model (plain structs) — consumed by a **Phase 3**
  serving adapter (foundation decided then, D-16). Phase 2 stops at the in-memory
  model; no ts-rs views, no HTTP.
</code_context>

<specifics>
## Specific Ideas

- Johan's original "model NVTX ranges as single-state Quent FSMs" is honored in
  *shape* (a range = one interval) but **not** in *mechanism* — we use a plain span
  type, not the legacy FSM machinery (D-08).
- Capture stays verbatim (Phase 1); resolution happens only here (Phase 2), matching
  the locked "capture raw, resolve in the analyzer" principle.
- "Capture what all NVTX provides" (owner) drove D-09: model resources too, since
  NVTX users mimic resource lifespans through NVTX's own primitives absent a
  capacity concept.
</specifics>

<deferred>
## Deferred Ideas

- **`analyzer-build`** — a schema-driven *consumer* generator symmetric to
  `instrumentation-build` (reads a `Schema` → generates a reconstruction/analyzer
  targeting a hand-written analysis-runtime). The "missing symmetric half" of the
  #191 redesign. A separate future initiative — it would not serve NVTX (schema-
  orthogonal) and is far larger than a v1 slice. Revisit at the Phase 3 serving
  decision (D-16 option B) and/or as its own milestone.
- **Operator correlation (COR-01)** — semantic linkage of NVTX ranges to query-engine
  operators. v2. Phase 2 keeps the model correlation-ready (D-03) but builds none.
- **Inferring Quent capacity/utilization from NVTX** (D-10) — heuristic translation of
  NVTX ranges/resources into Quent's resource-utilization model. Deferred.
- **Payload extension decode** (PAY-01/02) — v2, unchanged from Phase 1.
- **Phase 3 serving foundation (A vs B)** — flagged (D-16), decided at Phase 3.

None of the above are scope creep — they are explicit carry-forwards.
</deferred>

---

*Phase: 2-nvtx-model-tolerant-analyzer*
*Context gathered: 2026-07-23*
