<!-- refreshed: 2026-07-23 -->
# NVTX Phase 2 — Consumer/Analyzer Foundation (Current-Design Ground Truth)

**Analysis Date:** 2026-07-23
**Question:** Does the *current* (issue #191) crate design have a consumer/analyzer/reconstruction
path Phase 2 ("NVTX Model & Tolerant Analyzer") can build on? If not, what is the realistic foundation?

**Verdict up front:** The current-design crate set is **producer-only**. It has **no consumer,
reconstruction, analysis, timeline, or serving crate** — it stops at codegen. The **only** working
consumer→reconstruct→serve→UI vertical in the repo is the *legacy* stack
(`crates/analyzer` → `crates/model` → `crates/ui` → `domains/query_engine/{analyzer,server,ui}` →
`ui/`), and that stack is **still actively developed** (not deprecated). Phase 2's realistic
foundation is the legacy `crates/analyzer` framework with a **hand-written** NVTX domain analyzer
(no `model!`/`fsm!`/`entity!` proc-macros). Evidence below.

---

## Q1 — Is there ANY current-design consumer/analyzer/reconstruction path?

**No.** Nothing in the 11 current crates reads an event stream back and reconstructs anything.

**What the io importers produce.** `crates/io/types/src/lib.rs:95`:

```rust
pub trait Importer<T>: Iterator<Item = Event<T>> {}
```

An importer is just an iterator of `quent_events::Event<T>` envelopes (`crates/io/types/src/lib.rs:6`).
The format importers (`crates/io/{ndjson,msgpack,postcard}/src/lib.rs`, `crates/io/src/filesystem/importer.rs`)
all yield `Event<T>`. That is the raw material a consumer would reconstruct *from*.

**Who consumes importers** (`grep Importer` across the repo — every hit is legacy or io-internal):

- `crates/analyzer/src/error.rs` (re-exports `ImporterError`) — **legacy**
- `crates/model-macros/src/model_macro.rs` — **legacy**
- `domains/query_engine/{analyzer,server}/src/*` — **legacy domain**
- `examples/simulator/analyzer/src/lib.rs` — **legacy example**
- `crates/io/*` — the importer definitions themselves

**Current-design crates checked and found to have NO consumer path:**
`schema`, `constraints`, `fsm`, `resource`, `ref-target`, `ref-tree`, `yaml`, `instrumentation-build`,
`instrumentation`. The only `Event<>` references in `instrumentation-build` are **text tokens emitted
for codegen** (`crates/instrumentation-build/src/any_event.rs:40,44,94`), not consumption. The runtime
`crates/instrumentation` crate has zero `Importer`/`reconstruct`/`analyze` symbols.

**Conclusion:** importers exist and produce `Event<T>`, but the only things that turn that stream
into a model/spans/timeline live in the legacy vertical.

---

## Q2 — What does "model" mean in the current design, concretely?

Two unrelated meanings of "model" exist; do not conflate them.

- **Current design (`schema::Schema`)** — a `Schema` is a **declared event-model definition**
  (entities, events, records, fields, annotations/constraints), authored in YAML and lowered by
  `crates/yaml` (`parse_from_str → Parsed { schema, .. }`). Its consumers are **validation**
  (`crates/constraints::validate`) and **codegen** (`crates/instrumentation-build::generate`). A
  `Schema` is a **codegen/validation input only**. There is **no runtime/analysis facility that
  interprets an event stream against a `Schema`** — nothing takes `(Schema, Iterator<Event<T>>)`
  and produces spans/state-durations. You cannot reconstruct or analyze *against* a `Schema` today.

- **Legacy design (`quent_model::Model` + `crates/analyzer`)** — here "model" means a
  **reconstructed in-memory model** built from an event stream. `crates/analyzer/src/lib.rs`
  defines the analysis traits (`Entity`, `Span`, `Instant`, `Model`, plus `Fsm`/`FsmUsages`,
  `EntityEvents`, resource/timeline machinery). This is the only "interpret an event stream"
  facility in the repo.

**So:** on the current design a "model" is a static declaration, not something you analyze against.
The reconstruction meaning of "model" exists **only** in legacy `crates/analyzer`.

---

## Q3 — Confirm the legacy boundary for the whole serving vertical.

**All of these are legacy, and they form one connected component rooted at `crates/model`:**

`crates/model`, `crates/model-macros`, `crates/analyzer`, `crates/ui`,
`domains/query_engine/{analyzer,server,ui}` (+ `examples/simulator/*`).

**Signal 1 — dependency edges (they all pull in the legacy `quent-model` macro DSL):**
`grep quent-model = **/Cargo.toml` →
`crates/analyzer`, `crates/stdlib`, `crates/codegen`, `domains/query_engine/{model,analyzer}`,
`examples/{readme,simulator/*}`. `crates/model/src/lib.rs` re-exports the proc-macros
(`entity, fsm, model, resource, state, instrumentation, Attributes`) — the DSL the owner called legacy.

**Signal 2 — the serving stack is hardwired to `crates/analyzer`:**
`grep quent-analyzer **/Cargo.toml` → `crates/analyzer`, `crates/ui`,
`domains/query_engine/{analyzer,server,ui}`, `examples/simulator/{analyzer,ui}`. `crates/ui/src/lib.rs:6`
imports `quent_analyzer` directly and builds the **ts-rs view types** the frontend consumes.

**Signal 3 — the two worlds never touch.** No current crate depends on `quent-analyzer` or
`quent-model`, and no `schema`/`yaml`/`fsm`/`resource` crate depends on `quent-model`/`quent-analyzer`
(`grep` returned empty both directions). `crates/model` does **not** reference `quent-schema`. They are
**parallel, disconnected verticals**, sharing only the runtime substrate below.

**Shared substrate (not legacy, not producer-only):** `crates/events` (`Event<T>`, `EntityEvent`),
`crates/dynamic-attributes` (`DynamicAttribute`), and `crates/io` (`Exporter`/`Importer`). Note that
`crates/analyzer` itself depends on `quent-events` + `quent-dynamic-attributes`
(`crates/analyzer/Cargo.toml`) — so **the legacy analyzer already speaks the same `Event<T>` envelope
that NVTX capture emits.**

**Does anything current still depend on legacy?** The reverse edge exists via the shared runtime only:
`crates/instrumentation` (current) depends on `quent-io` + `quent-build-info` (support crates). No
Cluster-A (schema) crate depends on any legacy crate.

**Is legacy deprecated/abandoned? No — it is actively developed.** Most-recent commits:
- `crates/analyzer`: `#393` DAG data-flow timeline, **2026-07-20**; `#323` per-transition attrs, 2026-07-13.
- `crates/model`: `#418` attribute rename, **2026-07-20**.
- (For comparison, the #191 producer set is also hot: `schema` `#426` 2026-07-21; `yaml` `#437` 2026-07-22;
  `resource` `#438` 2026-07-22.)

Both verticals are under active development. The legacy vertical is **the serving product**; the #191
vertical is **the producer redesign**. There is no in-flight commit migrating the analyzer/serving side
onto the schema world.

---

## Q4 — What is the analyzer/consumer redesign plan, if any?

**Undetermined / not started, based on repo evidence.** Searches for design docs, RFCs, TODOs, or
issue references describing a *consumer-side* redesign found nothing concrete:

- `docs/README.md:6` states the vision ("tooling reconstructs, analyzes, and visualizes … from the
  emitted telemetry") and `:40` mentions a "statically typed, schema-driven model" — but the only
  *implemented* reconstruct/analyze/visualize pipeline it points to is the **legacy query_engine
  simulator** (`docs/README.md:31`). This is aspirational framing, not a consumer-redesign plan.
- Issue **#191** scope, per root `Cargo.toml:104` and `CURRENT-CRATES-MAP.md`, is the
  **producer/event-model/validation/codegen** set. No #191 sub-item is an analyzer/reconstruction crate.
- No `crates/schema`-consuming analyzer, no "schema-driven analyzer" TODO, no deprecation notice on
  `crates/analyzer` was found anywhere in code or docs.

**Is `crates/analyzer` slated for rewrite or staying?** No evidence of a scheduled rewrite; it is
receiving new features (Q3, `#393`). Treat it as **staying and current-for-serving** until the owner
says otherwise. If a schema-driven analyzer is intended, **it does not exist yet and has no documented
architecture** — Phase 2 would be pioneering it with no target to build against.

---

## Q5 — NVTX-specific fit.

**Would Phase 2 need to declare NVTX as a `schema::Schema`? No — and it cannot usefully.**

- `NvtxEvent` is a **hand-written Rust enum** (`integrations/nvtx/events/src/lib.rs`), and
  `NvtxEventEntity` is a `#[serde(transparent)]` newtype implementing `EntityEvent` with
  `NAME = "NvtxEvent"` (`integrations/nvtx/bridge/src/lib.rs`). The NVTX crates depend **only** on
  `nvtx-events` + `quent-events` (`integrations/nvtx/bridge/Cargo.toml`) — i.e. the shared runtime,
  neither the schema world nor the legacy model world.
- A `schema::Schema` is a **producer artifact** (drives codegen of an instrumentation API). NVTX is
  captured **in-process from a foreign library that already emits NVTX** — Quent does not generate its
  instrumentation. So the entire schema→codegen pipeline is **irrelevant to NVTX capture**; there is no
  generated `{Entity}Handle` to produce.
- Structurally, `NvtxEvent` is a **single heterogeneous enum** (12 variants: ranges, marks, domains,
  strings, resources). The schema model expects **per-entity typed event sets** with FSM/resource
  annotations. Forcing NVTX into a `Schema` would mean re-declaring the enum as an entity/event model
  purely to satisfy validation — buying nothing, since (a) nothing analyzes against a `Schema`, and
  (b) NVTX capture doesn't use generated code. **The hand-written enum is orthogonal to the schema
  world, not compatible-and-pending.**

**How would a current-design consumer receive/interpret the stream? It couldn't — there is none.**
NVTX arrives as one flat `Event<NvtxEventEntity>` stream (`integrations/nvtx/example/src/lib.rs`
emits via `observer.sender().emit(session, event)`). The **only** trait in the repo that ingests such a
stream is the legacy `domains/query_engine/analyzer` `UiAnalyzer::try_new(engine_id, events:
impl Iterator<Item = Event<Self::Event>>)`. Set `type Event = NvtxEventEntity` and the envelope matches
directly — because both NVTX capture and the legacy analyzer sit on `quent-events::Event<T>`.

---

## Q6 — The Phase 3 serving/UI reality: is the legacy analyzer effectively forced?

**Yes — for anything to render in the existing UI, the legacy analyzer is on the critical path.**

- The only server is `domains/query_engine/server`. Its `Cargo.toml` depends on **`quent-analyzer`,
  `quent-query-engine-analyzer`, `quent-query-engine-ui`, `quent-ui`, `quent-io`** — the full legacy
  vertical. It composes an Axum analyzer API + tonic collector (`domains/query_engine/server/src/lib.rs`).
- The frontend view types come from `crates/ui`, which **imports `quent_analyzer` directly**
  (`crates/ui/src/lib.rs:6`) and generates the ts-rs bindings the React app (`ui/`) consumes.
- Therefore the UI is **not decoupled** from the analyzer at the type level: the shapes the frontend
  renders (`ResourceTypeDecl`, timelines, entity lists) are derived from `quent_analyzer` types via
  `crates/ui`. There is no schema-world equivalent producing ts-rs view types.

**Nuance for Phase 2/3:** "using the legacy analyzer" means implementing the trait **framework** in
`crates/analyzer` + emitting `crates/ui` view types + implementing the `UiAnalyzer` composition the
server expects. It does **not** require the `model!`/`fsm!`/`entity!` **proc-macros** (`quent-model`
the DSL). The macro-free runtime path exists: `RtFsmBuilder`/`RtFsm`
(`crates/analyzer/src/fsm/runtime.rs`) builds FSMs at analysis time from arbitrary events, and
`EntityEvents<M>` (`crates/analyzer/src/entity/mod.rs`) accumulates non-FSM entities. The one residual
coupling: `crates/analyzer` uses a few `quent_model` **traits** (`EntityData` in `entity/mod.rs:7`;
`FsmEvent`/`TransitionInfo`/`ModelBuilder` in `fsm/events.rs`, `resource/events.rs`) — so a legacy-based
NVTX analyzer still **transitively links `quent-model`**, but a hand-written analyzer can stick to the
`RtFsm` + hand-rolled-entity path and avoid the macro DSL entirely. The prior `NVTX-PHASE2-MAP.md`
already sketched exactly this (Sections 2–3, 6–7).

---

## Q7 — Honest recommendation.

**Recommended: (a) build the NVTX analyzer on the legacy `crates/analyzer` framework, using the
macro-free `RtFsm` + hand-written-entity path (not `model!`/`fsm!`/`entity!`).** This is the only
option with a real target and an actual serving story.

Rationale, evidence-driven:

| Option | Foundation | Reality check |
|--------|-----------|---------------|
| **(a) Legacy `crates/analyzer`** | Traits `Model`/`Span`/`Fsm`/`FsmUsages`, `RtFsmBuilder`, `EntityEvents`, `crates/ui` view types, `UiAnalyzer` | **Works today, actively developed, backs the only server+UI.** Envelope already matches NVTX (`Event<NvtxEventEntity>` on `quent-events`). Renders in existing UI. Cost: transitive `quent-model` link; must tolerate incomplete FSMs (unclosed ranges) — the `RtFsmsBuilder` `?`-abort and zero-transition `unwrap()` panic in `fsm/runtime.rs` must be handled (synthesize `exit` transitions or partition incomplete FSMs; see `NVTX-PHASE2-MAP.md` §7). |
| **(b) Current-design analyzer on `schema`/`fsm`/`resource`** | none — would have to be invented | **No precedent, no target, no serving path.** The #191 set is producer-only (Q1); nothing consumes a `Schema` at analysis time (Q2); no documented redesign exists (Q4); NVTX gains nothing from being a `Schema` (Q5); and it still wouldn't render (Q6 — no schema-world ts-rs views/server). This is a research project, not a Phase-2 foundation. |
| **(c) Standalone spans, no framework** | plain `Vec<Span>` reconstruction independent of both | Cleanest domain model (a range **is** just a start/end interval — no FSM needed), and matches the constraint that NVTX must stand alone with zero query-engine telemetry. **But it renders nowhere:** the UI only knows `crates/ui`/`quent_analyzer` view types (Q6). Viable only as an internal reconstruction core that is then **adapted** to the analyzer/UI view types for Phase 3 — i.e. it collapses into (a) at the serving boundary. |

**Do not default to the new design.** The evidence is unambiguous: **the current design has no consumer
story.** Building Phase 2 on `schema`/`fsm`/`resource` would mean inventing an entire
reconstruction-and-serving vertical from scratch with no target — far beyond a "model + tolerant
analyzer" phase, and still unable to render.

**Concrete recommendation for Phase 2:**

1. **Reconstruction core (option-c spirit, framework-free):** a hand-written
   `domains/nvtx/analyzer` that replays `Event<NvtxEventEntity>`, resolves handle tables
   (domain/string/category/thread/resource — `NVTX-PHASE2-MAP.md` §4), and reconstructs ranges as
   **plain spans** (start+end intervals), not stateful FSMs. This honors the constraints: flat single
   stream, span-not-FSM, standalone. Keep this core independent of `quent-model` macros.
2. **Serving adapter (option-a):** implement the legacy `crates/analyzer` traits + `crates/ui` view
   types + `UiAnalyzer`/`Viewer` over that core so Phase 3 can render in the existing
   `domains/query_engine/server` + `ui/` (or a sibling NVTX server modeled on it). Use `RtFsmBuilder`
   only if a range genuinely benefits from state modeling; otherwise map spans straight to timeline
   view types. Prefer synthesizing `exit`/using the always-succeeds `FsmEvents` path over the
   panic-prone `RtFsm` zero-transition path.
3. **Do not declare NVTX as a `schema::Schema`.** It buys nothing and links the wrong vertical (Q5).

This is effectively **(a)+(c) hybrid**: a clean framework-free reconstruction core, adapted to the
legacy analyzer/UI at the serving seam because that seam is the only one that renders. The "don't build
on legacy" guidance is real but **cannot be honored for the serving side without first building a
consumer vertical the current design simply does not have** — flag this explicitly to the owner as the
central Phase-2 decision.

---

## Evidence index (files cited)

- Importer output: `crates/io/types/src/lib.rs:6,86-98`
- No current-design consumer: `grep Importer` (legacy-only hits); `crates/instrumentation-build/src/any_event.rs:40-99` (codegen text, not consumption)
- Schema = codegen/validation input: `crates/yaml/src/lib.rs`, `crates/constraints`, `crates/instrumentation-build::generate`
- Legacy analysis traits: `crates/analyzer/src/lib.rs`, `crates/analyzer/src/fsm/runtime.rs`, `crates/analyzer/src/entity/mod.rs`
- Legacy DSL re-export: `crates/model/src/lib.rs:19-21`
- Worlds disconnected: `grep quent-model/quent-analyzer` in schema/yaml/fsm/resource → empty
- Legacy still active: `git log` `crates/analyzer` `#393` 2026-07-20; `crates/model` `#418` 2026-07-20
- Serving hardwired to analyzer: `domains/query_engine/server/Cargo.toml:13-22`; `crates/ui/src/lib.rs:6`
- Analyzer↔model residual trait coupling: `crates/analyzer/src/entity/mod.rs:7`, `crates/analyzer/src/fsm/events.rs:12`
- NVTX on shared runtime only: `integrations/nvtx/bridge/Cargo.toml`; `integrations/nvtx/example/src/lib.rs`
- Vision framing (not a plan): `docs/README.md:6,40`; #191 scope: root `Cargo.toml:104`

*NVTX Phase 2 current-design analysis: 2026-07-23*
