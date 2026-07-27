# Phase 2: NVTX Model & Tolerant Analyzer - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-23
**Phase:** 2-nvtx-model-tolerant-analyzer
**Areas discussed:** Domain framing (merge vs standalone), Foundation (current-design vs legacy), Model machinery, Tolerance strategy, Unresolved-handle policy, Resource modeling

---

## Domain Framing — "merged into Quent traces" vs standalone stream

The owner initially asked to *correct* the framing: the value of NVTX is bringing
info into existing Quent telemetry, visible in the Quent UI. Explored what "merged"
means.

| Option | Description | Selected |
|--------|-------------|----------|
| Standalone stream, shared time axis | NVTX renders as its own lanes on the same timeline as other Quent telemetry | ✓ (as the Phase 2/data reality) |
| Semantic correlation (nested under operators) | NVTX ranges recorded as children of query-engine operators | Deferred to v2 (COR-01) |

**User's choice:** Wanted "#2 correlated" — but on drill-down, "correlated" meant
*where to dock the NVTX lane* (a UI-layout choice), NOT semantic data linkage.
**Notes:** Key technical reality surfaced — primary targets (libcudf, cuCascade)
emit *only* NVTX with zero query-engine telemetry, so there is nothing to merge
into; NVTX ranges are the whole trace. Sirius is a secondary target that has both.
Three concepts separated: (A) reconstruction [Phase 2], (B) lane placement [Phase 3
UI], (C) semantic data linkage [v2 COR-01]. Owner's ask = B. Locked: NVTX model
stands alone; correlation-ready but correlation not built.

---

## Foundation — current (#191 YAML/schema) design vs legacy analyzer

Owner corrected an earlier map that leaned on `model!`/`fsm!`/`entity!` macros:
those are legacy; the design is moving to YAML/schema. Triggered a re-investigation.

| Option | Description | Selected |
|--------|-------------|----------|
| (a) Legacy `crates/analyzer` framework | Hand-written analyzer on `RtFsm`/`EntityEvents` + `crates/ui` view types | Partial — only at Phase 3 serving seam if chosen |
| (b) Current-design analyzer on schema/fsm/resource | Reconstruct against a `Schema` | Rejected — producer-only, no consumer story, no target |
| (c) Framework-free reconstruction core | Plain spans, depends only on shared runtime + nvtx-events | ✓ for Phase 2 |

**User's choice:** (c) for Phase 2; Phase 3 serving decision (A vs B) deferred.
**Notes:** Investigation (`NVTX-PHASE2-CURRENT-DESIGN.md`) proved the #191 design is
producer-only — no consumer/analyzer/serving crate exists on it, and legacy is
actively developed and backs the only server+UI. Critical disambiguation: "legacy"
conflated the macro DSL (rejected; NVTX doesn't need it) with the reconstruction/UI
framework (no successor; only renderer). NVTX is schema-orthogonal — do not declare
it as a `Schema`. Phase 2 built framework-free; the legacy question moves to Phase 3.

---

## Model machinery — how a range is represented

| Option | Description | Selected |
|--------|-------------|----------|
| Proc-macro model (`entity!`/`fsm!`) | Macro-generated typed per-entity streams | Rejected — legacy DSL; doesn't fit a single flat stream |
| `schema::Schema` declaration | Declare NVTX as a YAML schema | Rejected — buys nothing (D-07) |
| Hand-written core, range = plain span | Own span type, isomorphic to a single-state FSM | ✓ |

**User's choice:** Plain span ("I don't see any model other than the reused
single-state FSM").
**Notes:** Confirmed span == single-state-FSM in shape; framework-free means our own
type (avoids the transitive `quent-model` link). Johan's "ranges as single-state
FSMs" honored in shape, not mechanism.

---

## Tolerance strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Fix shared `crates/analyzer` framework | Patch the `unwrap` panic + `?`-abort | Not needed — legacy not on our path |
| Synthesize exits locally | Close open ranges at trace-end in our core | ✓ (by construction) |

**User's choice:** Implicitly resolved by the framework-free decision — tolerance is
handled inside our own core.
**Notes:** The panic-prone legacy `fsm/runtime.rs` is simply not on the Phase 2 path.
We close at trace-end, sort by timestamp, handle duplicates ourselves. Flag
synthetically-closed ranges for Phase 3.

---

## Unresolved-handle policy

| Option | Description | Selected |
|--------|-------------|----------|
| Drop the range | Skip unresolved | Rejected — loses useful timing |
| Treat as None/unnamed | Generic unnamed | Partial (legitimately-unnamed cases) |
| Stable placeholder exposing raw id | `<domain 0xAB>` etc. | ✓ |

**User's choice:** Stable placeholders (also roadmap-locked, success criterion 2).
**Notes:** Distinguish legitimately-unnamed (default domain 0, unnamed thread → clean
default label) from referenced-but-unresolved (non-zero handle, no registration →
placeholder with raw id). Two-pass reconstruction for forward references.

---

## Resource modeling

| Option | Description | Selected |
|--------|-------------|----------|
| Defer resources (captured-only) | Don't reconstruct NVTX resources in v1 | Rejected after discussion |
| Model resources as named lifespans | create→destroy span + name + type label | ✓ |

**User's choice:** Model them — "capture what all NVTX provides."
**Notes:** Owner's reasoning: NVTX has no Quent-style capacity concept, so users
mimic resource lifespans via NVTX's own primitives; that signal is worth
reconstructing. Two boundaries drawn: model NVTX resources as named object lifespans
(NOT Quent capacity-resources — inference deferred), and core resource types now /
extension (CUDA) types raw-passthrough (mirrors payload D-12).

---

## Claude's Discretion

- Crate placement (lean: `integrations/nvtx/analyzer`).
- Nested-range representation (flat + containment vs. explicit tree).
- Test fixtures (real capture for happy path + synthetic malformed streams).
- Reconstruction strategy (batch / two-pass).

## Deferred Ideas

- `analyzer-build` — schema-driven consumer generator symmetric to
  `instrumentation-build`; the missing half of #191; a future initiative, would not
  serve NVTX anyway.
- Operator correlation (COR-01) — v2.
- Inferring Quent capacity/utilization from NVTX — heuristic, deferred.
- Payload extension decode (PAY-01/02) — v2.
- Phase 3 serving foundation (A adapt-to-legacy vs B new-consumer-vertical) — decided
  at Phase 3.
