# Quent NVTX Consumer

## What This Is

An NVTX ingestion pipeline for Quent: a Rust injection library that captures NVTX events (ranges, marks, domains, registered strings, resources, and payload-extension data) from instrumented applications and turns them into Quent events, plus the Quent model, analyzer, API endpoint, and UI rendering needed to see those ranges as traces. It exists so Quent can observe GPU-accelerated data processing libraries (libcudf, cuCascade, and similar) that already emit NVTX, and it includes a fan-out mediator so Quent can coexist with other NVTX consumers like Nsight Systems and AON in the same process.

## Core Value

An application emitting NVTX ranges can be observed by Quent end-to-end — events captured, reconstructed into a model, and visible in the Quent UI — without breaking that application's ability to also be profiled by NSys/AON.

## Requirements

### Validated

<!-- Existing capabilities inferred from the codebase map. -->

- ✓ Model-driven instrumentation: declarative FSM/Resource/Entity models via proc macros generate typed instrumentation APIs — existing
- ✓ Event pipeline: sync-facing `EventSender` over tokio, pluggable exporters (ndjson/msgpack/postcard/collector/callback) — existing
- ✓ gRPC collector: client-streaming event collection with per-source contexts (`:7836`) — existing
- ✓ Domain-agnostic analyzer: FSM/resource runtimes, span reconstruction, binned timelines — existing
- ✓ Query-engine domain: Engine/Worker/Query/Plan/Operator model + analyzer + Axum HTTP API (`:8080`) — existing
- ✓ React UI: engine→query drill-down with ECharts timelines, operator Gantt chart, plan DAGs, ts-rs typed API contract — existing
- ✓ C++/Python bridges: cxx and PyO3 codegen from models — existing
- ✓ NVTX capture foundation: `nvtx-events` verbatim vocabulary, `nvtx-injection` (with `static-injection` feature for in-process capture), and `nvtx-bridge` (`NvtxEventEntity` newtype adapter into Quent's `EntityEvent`) — Validated in Phase 1: Capture Foundation (full core NVTX surface: push/pop, start/end, marks, domain lifecycle, registered strings, category/thread naming, resources; CORE payload union captured verbatim). `Observer::sender()` added to `quent-instrumentation` so the `'static` injection hook can emit into an app-owned observer that still flushes on drop.
- ✓ In-repo deterministic NVTX test application (`nvtx-example`) with GPU-less capture test: app owns its `Context`/`Observer`/exporter, installs injection hook via `static-injection`, emits via the `nvtx` crate, asserts all core NVTX kinds round-trip — Validated in Phase 1: Capture Foundation

### Active

<!-- v1 scope. Hypotheses until shipped and validated. -->

- [ ] Payload **extension** capture (schemas, enums, binary `nvtxPayloadData_t`) — deferred from Phase 1 per D-12 (libcudf emits none today); core NVTX surface already captured and validated in Phase 1
- [ ] Fan-out mediator: multiple NVTX consumers can attach to one process via per-sink shadow tables, including passthrough of an external tool supplied via `NVTX_INJECTION64_PATH` (real Quent + NSys coexistence)
- [ ] NVTX events flow into Quent's standard event pipeline (`EventSender` → exporters/collector) as a Quent model — NVTX ranges modeled as FSMs with a single "range open" state
- [ ] Analyzer reconstructs NVTX ranges into traces/spans, resolving handles (domains, registered strings, categories) from the event stream
- [ ] HTTP endpoint(s) expose reconstructed NVTX data through the existing server layer
- [ ] NVTX data rendered in the Quent UI (exact visualization decided at phase planning / UI-spec time)
- [ ] Manual validation against a real GPU library workload (libcudf-style) before v1 is called done

### Out of Scope

- Operator correlation (tying NVTX ranges back to query-plan operators) — Johan's "stretch goal / cherry on top"; revisit after v1
- Windows support — NVTX injection via the weak-symbol mechanism doesn't work on Windows (wchar_t size, linker semantics); PR #87 already compile-errors there
- Device-side NVTX as an event source — not publicly available yet (~end of 2026 per Akash Goel); limited feature parity when it lands (8-byte payloads, const-literal strings)
- Upstreaming the Rust injection layer to NVIDIA/NVTX as a v1 deliverable — parallel track; keep the injection crate cleanly separable so it can be offered upstream, but v1 does not depend on or include the upstream PR

## Context

**Origin:** rapidsai/quent#76 ("Consume NVTX ranges", johanpel). Quent should consume NVTX ranges to form traces for insight into GPU-accelerated data processing libraries. Two challenges named there: (1) no Rust bindings exposing NVTX's injection/consumer API — the NVTX Rust library only produces events; (2) NVTX allows only one injection library per process (like CUPTI), making multi-consumer setups non-trivial.

**Prior work — PR #87 (draft, johanpel/quent-old fork branch `nvtx`, stale since April 2026):** a Rust base layer with three crates under `integrations/nvtx/`:
- `quent-nvtx-events` — serde types mirroring raw NVTX API calls verbatim (no interpretation; handle resolution deferred to the analyzer). Payload-extension types defined but marked "not yet emitted — Phase 5".
- `quent-nvtx-injection` — bindgen over NVTX C headers (located via the `nvtx-sys` git dep), a C shim providing a strong `InitializeInjectionNvtx2_fnptr` symbol overriding NVTX's weak one, callback→`NvtxEvent` conversion, one-shot `install_hook()`. Linux/macOS 64-bit only. Has an integration test.
- `quent-nvtx-instrumentation` — thin bridge: `install<T: From<NvtxEvent>>(sender, session_id)` forwards captured events into Quent's `EventSender`.

The PR predates the exporter/serde-bounds refactors on main (e.g. #324), so it must be rebased and evaluated before being adopted as the base.

**Slack discussion (Johan, Lawrence Mitchell, Robert Dietrich):** Johan's proposed sequence — model NVTX ranges as single-state Quent FSMs; provide a Rust ingestion library (PR #87 as base); then analyzer endpoint + UI rendering. The single-subscriber problem: Johan sketched a fan-out library that populates the global NVTX table itself and keeps a shadow table per sink, implicitly treating whatever `NVTX_INJECTION64_PATH` points to as one more sink; Lawrence independently described the same design as a walked handler list. Never prototyped — this project builds it.

**Codebase fit (from `.planning/codebase/`):** the injection library plays the role of an "instrumented application" source feeding an `EventSender`; the missing pieces mirror the existing `domains/query_engine/` layout (model → analyzer → server → ui). The UI already has the visualization machinery an NVTX range view needs (`TimelineController`/`TimelineRuler`, `OperatorGanttChart` — spans per row on a shared time axis).

**Known hazard (CONCERNS.md):** the analyzer panics on out-of-order/duplicate-timestamp events and fails the whole build on incomplete FSM lifecycles (`crates/analyzer/src/fsm/runtime.rs:309-313`). Real NVTX streams will routinely contain unclosed ranges at process exit and imperfect ordering across threads — the NVTX analyzer path must tolerate incomplete/malformed telemetry rather than inherit those panics.

## Constraints

- **Platform**: Linux (and possibly macOS) 64-bit only — NVTX injection relies on weak-symbol override / `NVTX_INJECTION64_PATH`, which excludes Windows
- **Language**: Ingestion library in Rust — explicit preference from Johan ("do not want to write anything new in C/C++ if I can help it"); only minimal C shims where the linker mechanism demands it
- **Architecture**: Follow the established repo layering — application-agnostic capture crates, domain model/analyzer/server/ui split, `quent-*` naming, workspace `members`+`default-members` registration
- **Separability**: The injection crate must stay cleanly separable from Quent so it can be offered upstream to NVIDIA/NVTX later
- **NVTX semantics**: One injection slot per process is an NVTX invariant we must design around (fan-out), not something we can change
- **Compatibility**: External NVTX consumers (nsys via `NVTX_INJECTION64_PATH`) must keep working unmodified when the fan-out mediator is in place
- **CI**: End-to-end validation must be runnable without GPU hardware (deterministic in-repo NVTX test app); GPU-library validation is manual

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| v1 is a full vertical slice (capture → model → analyzer → endpoint → UI) | Proves the whole pipeline; matches Johan's proposed sequence | — Pending |
| Evaluate PR #87 by rebasing onto current main early, then decide adopt-vs-reference | It has working bindgen/hook plumbing but is draft, stale, and predates exporter refactors | Referenced — Phase 1 shipped fresh `nvtx-events`/`nvtx-injection`/`nvtx-bridge`/`nvtx-example` (no `quent-` prefix, upstreaming-friendly) |
| App-integrated in-process capture via `static-injection` (not cdylib + `NVTX_INJECTION64_PATH`) | Johan's review of the cdylib bridge design pivoted the approach: app owns its `Context`/`Observer`/exporter; `static-injection` feature links a strong `InitializeInjectionNvtx2` symbol so NVTX initializes injection in-process at the first NVTX call — no cdylib, no env var. The bridge collapses to a single `NvtxEventEntity` newtype. | Phase 1: `nvtx-example` wires this pattern and its test asserts all core NVTX kinds round-trip |
| Fan-out mediator with external-tool passthrough is IN v1 | Quent is positioned as complementary to NSys/AON; coexistence is the common case, not an edge case | — Pending |
| Capture full NVTX surface including payload extension in v1 | Payloads carry the structured data that makes ranges analytically useful | Core surface + CORE payload union shipped in Phase 1; payload **extension** deferred (D-12) |
| Model NVTX ranges as single-state Quent FSMs | Johan's suggestion; fits the existing FSM/span analyzer machinery | — Pending |
| UI visualization shape decided at planning time | Commit to "NVTX data visible in the UI"; pick listing vs range-timeline once analyzer output shape is concrete | — Pending |
| Validation via both in-repo test app and manual libcudf-style run | CI-friendly determinism plus proof on the real target | — Pending |
| Upstreaming to NVTX is a parallel track, not a v1 deliverable | Johan: "can be figured out in parallel"; separability keeps the door open | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-07-22 after Phase 1 (Capture Foundation) fully merged to upstream (PR #402 — app-integrated in-process capture)*
