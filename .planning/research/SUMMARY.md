# Project Research Summary

**Project:** Quent NVTX Consumer
**Domain:** NVTX v3 injection/consumer library + fan-out mediation feeding an existing Rust telemetry pipeline (Quent)
**Researched:** 2026-07-08
**Confidence:** HIGH (stack + features verified against primary NVTX sources and Quent codebase map; injection internals and fan-out mechanics MEDIUM-HIGH)

## Executive Summary

This is an **integration project, not a greenfield one**. Quent already owns the entire model → instrument → export → collect → analyze → serve → UI spine; the NVTX work adds one new event *source* (a Rust injection library exporting `InitializeInjectionNvtx2`) at the top of that spine and one new *domain* (`domains/nvtx/`) mirroring `domains/query_engine/`. There is no consumer-side NVTX crate to adopt anywhere in the ecosystem — the upstream `nvtx`/`nvtx-sys` crates only *produce* events — so the consumer surface is custom FFI: bindgen over the NVTX C injection headers, populate the app's callback function tables with Rust callbacks, and convert callbacks into Quent events. PR #87 already prototyped the right toolchain (bindgen + cc + cargo_metadata) and the "capture raw, interpret later" event design; the roadmap should rebase/adopt it rather than restart.

The recommended approach follows a strict boundary rule: everything above `EventSender::emit` is NVTX-specific, application-agnostic, and upstreamable (`integrations/nvtx/` crates: sys, events, mediator, injection, instrumentation); everything below reuses the Quent spine unchanged. Capture is deliberately "dumb" — stamp a timestamp at callback entry, copy caller-owned strings immediately, emit raw handles verbatim — and all interpretation (handle resolution, span pairing) happens in the analyzer as a stream fold. A working vertical slice (NVTX app → capture → analyzer → UI swim lanes) is achievable with a *single-consumer* injection before the fan-out mediator lands, so coexistence hardening can be sequenced after the demonstrable slice.

Two risks dominate and both warrant dedicated phases with deeper research. First, the **fan-out mediator** (Quent owns the single injection slot, external tools like Nsight become dlopen'd sinks with per-sink shadow function tables) is the headline differentiator and has never been prototyped — its failure modes (double-initializing nsys, sink registration races, one bad sink crashing the app) are subtle and a wrong design is expensive to recover from. Second, **Quent's existing analyzer panics on exactly what real NVTX streams contain**: unclosed ranges at process exit, out-of-order and duplicate timestamps (`crates/analyzer/src/fsm/runtime.rs:309-313`, `fsm/mod.rs:125-135`). Tolerant reconstruction (pre-close dangling ranges, bucket malformed pairs, deferred two-pass handle resolution keyed by `(domain, kind, id)`) is a prerequisite, not a feature — no UI work has real data until it exists.

## Key Findings

### Recommended Stack

Thin FFI problem, not a framework problem. Full detail in `STACK.md`. The primary runtime path (`NVTX_INJECTION64_PATH` dlopen of Quent's lib) needs **zero C code** — just a `#[no_mangle] extern "C"` entry point; the C shim (via `cc`) is only needed for the statically-linked-injection test scenario and should be feature-gated off the default build.

**Core technologies:**
- **bindgen 0.72.1** (build-dep): generate Rust FFI bindings for NVTX C headers — same tool upstream `nvtx-sys/build.rs` uses; headers are macro-heavy/versioned and untenable to hand-write. Requires `libclang` added to `pixi.toml` (the single most common bindgen CI failure).
- **libloading 0.9.0**: dlopen external injection libs (`NVTX_INJECTION64_PATH` passthrough) in the mediator — the only new runtime dependency the fan-out introduces.
- **cargo_metadata 0.23**: locate NVTX headers from the `nvtx-sys` git dependency at build time (PR #87's mechanism). Pin `nvtx-sys` to a commit SHA, not the moving `release-v3` branch.
- **std `OnceLock` + workspace `thiserror 2`**: one-shot `install_hook()` guard and error types — no new deps.
- **Avoid:** `cxx`/`autocxx` (plain C ABI, not C++), Windows tooling (weak-symbol injection doesn't work there; `#[cfg(unix)]` compile-error as PR #87 does), and using upstream `nvtx` crates as a consumer (they can't).

### Expected Features

Full detail in `FEATURES.md`. Table stakes = the semantics NVTX emitters rely on and Nsight users cross-check against; getting them wrong means the trace is simply untrusted.

**Must have (table stakes):**
- Per-thread swim lanes with push/pop nested-stack reconstruction (keyed by `(tid, domain)`)
- Start/End cross-thread range matching by correlation id (a *different* mechanism than push/pop — process-wide handle map, not the thread stack)
- Handle resolution: registered strings, domains, categories — with per-domain namespacing (never a global id map)
- Message/label + color + thread-name rendering; marks as instant events
- **Unclosed-range and out-of-order/duplicate-timestamp tolerance** — the existing analyzer panics on both; hard blocker
- Zoom/pan/ruler and hover tooltips (near-free — reuse `TimelineController`/`OperatorGanttChart`)

**Should have (competitive):**
- **Multi-consumer fan-out** (coexist with Nsight/AON) — THE differentiator; everyone else forces exclusive ownership of the injection slot
- Payload extension display (schemas/enums/binary decode) — where "trace" becomes "queryable telemetry"; capture wired in v1, render in v1.x
- Domain/category filtering; range statistics/aggregation (reuse binned-timeline analyzer)
- Deterministic in-repo NVTX test app for GPU-less CI

**Defer (v2+):**
- Operator correlation (NVTX ↔ query-plan operators) — explicit "cherry on top"; keep model boundaries clean so it's addable
- Device-side NVTX (not publicly available until ~end 2026), Perfetto/Nsight export, Windows

### Architecture Approach

New capture layer in `integrations/nvtx/` (five crates with one dependency direction each), new domain in `domains/nvtx/` mirroring query_engine, everything between reused unchanged. Two constraints shape every boundary: NVTX callbacks fire synchronously on the app's own threads with no timestamp and a strict latency budget (stamp-and-hand-off; no locks/allocation/serialization on the hot path), and NVTX allows exactly one injection per process (the mediator owns the slot; sinks get shadow tables). Full detail in `ARCHITECTURE.md`.

**Major components:**
1. `quent-nvtx-sys` + `quent-nvtx-events` — bindgen FFI + verbatim raw `NvtxEvent` serde types (zero Quent deps; upstreamable vocabulary)
2. `quent-nvtx-mediator` — owns the single injection slot; lock-free sink registry; per-sink shadow function tables; dlopen passthrough (zero Quent deps; most upstreamable piece)
3. `quent-nvtx-injection` + `quent-nvtx-instrumentation` — the Quent sink (stamp + copy + hand-off) and the thin `From<NvtxEvent>` → `EventSender` bridge (unbounded emit; never the bounded collector channel)
4. `domains/nvtx/{model,analyzer,server,ui}` — single-"open"-state FSM range entity via `model!`; tolerant analyzer (handle resolution as a stream fold, pre-close dangling ranges, bucket malformed); axum routes; React route reusing Timeline/Gantt
5. In-repo deterministic NVTX test app — CI vehicle, no GPU; also the subprocess test harness (process-global one-shot state is incompatible with in-process `cargo test`)

### Critical Pitfalls

Top 5 of 11 catalogued in `PITFALLS.md`:

1. **Fan-out multiplexing breaks the external tool** — nsys assumes sole ownership; double-init or a shared-looking table breaks it, and one crashing sink kills the app for everyone. Avoid: init external tool exactly once with its own shadow table; freeze/COW the sink list; `catch_unwind` at every FFI boundary; provide a Quent-alone mode. *Flag for dedicated prototyping.*
2. **Analyzer panics on real telemetry** — unclosed ranges, out-of-order/dup timestamps abort the whole trace build today. Avoid: pre-close at EOF in the NVTX analyzer, bucket malformed pairs, land the additive incomplete-FSM bucket in the shared analyzer, build malformed-stream fixtures as first-class tests.
3. **Callback distortion / backpressure** — real work in the callback inflates the ranges being measured; a bounded blocking channel stalls libcudf, an unbounded one OOMs it. Avoid: capture-only callback, bounded ring with drop-counter, drain thread; keep app threads off the collector's bounded mpsc.
4. **C-string lifetime + registered-string confusion** — caller-owned `const char*` is valid only during the call (copy immediately or use-after-free crashes the *app*); registered strings must be captured once at registration and referenced by handle thereafter.
5. **Events before hook install + handle-before-registration** — NVTX resolves its table once, lazily; static-init registrations in libcudf can precede capture. Avoid: `NVTX_INJECTION64_PATH` discovery, and analyzer degrades to stable placeholders (`domain#D/string#7`) instead of dropping ranges; two-pass deferred resolution.

## Implications for Roadmap

Based on combined research, a 5-phase structure. The vertical slice is demonstrable after Phase 3 with a single-consumer injection; fan-out (Phase 4) hardens coexistence without blocking "NVTX visible in the UI."

### Phase 1: Capture Foundation (FFI + single-consumer injection + test app)
**Rationale:** Nothing works without the FFI vocabulary and a proven capture path; the PR #87 adopt-vs-reference decision gates everything and must be resolved first. The deterministic test app must exist from day one because process-global one-shot hook state forces subprocess-based integration testing (Pitfall 11) — the harness shapes how every later phase is testable.
**Delivers:** `quent-nvtx-sys` (bindgen, pinned nvtx-sys commit, libclang in pixi), `quent-nvtx-events` (verbatim raw types), `quent-nvtx-injection` single-consumer (stamp-and-hand-off, immediate string copy, version/size guarding, idempotent init), `quent-nvtx-instrumentation` bridge, in-repo NVTX test app + subprocess test harness. End-to-end file dump of raw events.
**Addresses:** Full-surface raw capture (payload capture wired even if unrendered); in-repo CI test app.
**Avoids:** Pitfalls 2, 3, 4, 5, 6, 11 (callback cost, backpressure policy, string lifetime, ABI drift, symbol/init hazards, test contamination).

### Phase 2: NVTX Domain Model + Tolerant Analyzer
**Rationale:** Tolerance is a prerequisite for any real data reaching the UI — the existing analyzer's panics are a hard blocker called out by every research file. This is the critical-path risk and touches shared-analyzer invariants. Can start against synthetic `NvtxEvent` fixtures; does not wait on the mediator.
**Delivers:** `domains/nvtx/model` (single-"open"-state FSM range entity, marks, resources), `domains/nvtx/analyzer` (two-pass handle resolution keyed by `(domain, kind, id)`; per-thread push/pop stacks; process-wide start/end matching by correlation id; close-at-EOF; malformed-event diagnostics bucket), plus the additive incomplete-FSM bucket / span-error propagation fix in the shared analyzer. Malformed-stream fixtures (unclosed, reordered, cross-thread, dup-ts) as first-class tests.
**Uses:** `crates/model-macros` + `crates/analyzer` FSM/span runtimes (NOT the dead preliminary `crates/fsm`/`crates/schema`).
**Avoids:** Pitfalls 8, 9 (analyzer panics; handle-before-registration/namespacing).

### Phase 3: Server + UI Swim Lanes (vertical slice complete)
**Rationale:** Pure mirroring of `domains/query_engine/` — well-trodden internal patterns, low risk, high demo value. Completes "an NVTX app is visible end-to-end in Quent."
**Delivers:** `domains/nvtx/server` (axum routes + collector composition + caches), `domains/nvtx/ui` ts-rs view types, React route with per-(domain, thread) lanes, nested spans, marks as instants, color/message/category, tooltips — reusing `TimelineController`/`OperatorGanttChart`.
**Implements:** The serve/UI end of the pipeline; HTTP endpoint mirroring query_engine layout.

### Phase 4: Fan-out Mediator + External-Tool Passthrough
**Rationale:** The headline differentiator and the least-charted design (never prototyped). Sequenced after the slice so it's inserted *under* a working injection — Quent becomes one sink, the dlopen'd external tool another — and so a design misstep doesn't block the demonstrable pipeline. Recovery cost of getting this wrong is HIGH; consider an early throwaway spike during Phases 1–3 to de-risk.
**Delivers:** `quent-nvtx-mediator` (owns `InitializeInjectionNvtx2`; lock-free/COW sink list; per-sink shadow function tables; `libloading` dlopen of `NVTX_INJECTION64_PATH`; `catch_unwind` at FFI boundaries; Quent-alone mode). Verified: nsys produces a correct trace *simultaneously* with Quent capture.
**Uses:** libloading 0.9; synthetic `getExportTable` per sink per the STACK.md mechanics.
**Avoids:** Pitfall 7 (single-subscriber invariant, double-init, sink isolation).

### Phase 5: Payload Extension (schemas, enums, binary payloads)
**Rationale:** High value (structured, queryable data) but high cost, delivered through a *separate* NVTX extension-module mechanism with no existing Rust tooling — the second least-charted area after fan-out. PR #87 explicitly deferred it ("Phase 5 — not yet emitted"). Layers onto the existing events→bridge→analyzer→UI chain without blocking anything.
**Delivers:** Ext-module glue (`nvtxExtInit`/`NvtxExtModuleInfo`), verbatim schema/enum registration capture, analyzer-side blob decoding honoring declared offsets/alignment (never assume packed), unknown-entry preservation, payload display in the UI. Gated behind a flag so a payload bug can't break range capture.
**Avoids:** Pitfall 10 (schema/alignment/forward-compat mis-parse).

Post-phase: manual libcudf-style GPU validation is required before calling v1 done (distinct from CI); domain/category filtering and range statistics are natural v1.x follow-ons once multi-library traces get noisy.

### Phase Ordering Rationale

- **Dependency-driven:** events/FFI vocabulary precedes everything; analyzer tolerance precedes any UI over real data; the mediator is inserted under a proven single-consumer injection rather than built first (prove capture, then generalize to fan-out).
- **Parallelizable:** Phases 2–3 (model/analyzer/server/ui) can begin against synthetic `NvtxEvent` fixtures before injection work finishes — they depend only on `quent-nvtx-events`.
- **Risk placement:** the two HIGH-risk items (mediator, payload extension) are isolated in their own phases so a redesign in either doesn't ripple; the slice ships without them.
- **Boundary discipline avoids pitfalls structurally:** raw-capture/interpret-later kills hot-path resolution races (Pitfalls 2, 9); zero-Quent-dep mediator/events crates preserve the upstreamability constraint.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 2 (analyzer):** touches shared-analyzer invariants; needs the malformed/out-of-order fixture design CONCERNS.md says doesn't exist yet; exact FSM-builder integration (`RtFsmsBuilder` bucket semantics) needs code-level investigation.
- **Phase 4 (mediator):** novel with no Quent analogue and no public prior art; injection-slot ordering vs. an already-set `NVTX_INJECTION64_PATH`, nsys double-init behavior, and sink-isolation limits must be prototyped against real nsys before committing the design.
- **Phase 5 (payload extension):** separate ext-module init protocol with zero existing Rust tooling; `nvtxExtPayloadTypeInfo.h`-driven layout parsing needs header-level research.

Phases with standard patterns (skip research-phase):
- **Phase 1:** PR #87 prior art + upstream `nvtx-sys/build.rs` establish the toolchain; pitfalls are catalogued with concrete mitigations.
- **Phase 3:** direct mirror of `domains/query_engine/` — internal conventions, fully documented in the codebase map.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Crate versions verified on docs.rs/crates.io; NVTX injection ABI verified against `NVIDIA/NVTX` release-v3 source |
| Features | HIGH | NVTX API + Nsight behavior from official docs; HPC-tool comparison MEDIUM (survey-level) |
| Architecture | MEDIUM-HIGH | Quent layering HIGH (codebase map); NVTX injection internals MEDIUM (headers/docs, not exercised live) |
| Pitfalls | MEDIUM-HIGH | Quent-specific hazards HIGH (CONCERNS.md); some FFI/lifetime details MEDIUM (training-data + header inference) |

**Overall confidence:** HIGH for scoping/ordering decisions; MEDIUM for the two flagged unknowns (mediator mechanics, payload ext-module protocol), which is exactly why they're isolated phases with research flags.

### Gaps to Address

- **Fan-out mediator viability:** never prototyped anywhere; validate the shadow-table + dlopen-passthrough design against real nsys early (spike during Phases 1–3, commit in Phase 4). If nsys misbehaves as a shadowed sink, the coexistence story needs redesign.
- **Pure-Rust strong-symbol override for static injection:** STACK.md hypothesizes `#[no_mangle] #[used]` static can beat NVTX's weak C symbol without the cc shim — validate at link time in Phase 1; fall back to the cc shim if not.
- **Hand-off cost under high-frequency emission:** direct unbounded `EventSender::emit` vs. ring-buffer + drainer is a measured decision — benchmark in Phase 1 with the test app before adding complexity.
- **Payload ext-module init protocol:** `nvtxExtInit`/`NvtxExtModuleInfo` wiring is undocumented territory; budget header-level research at Phase 5 planning.
- **PR #87 rebase decision:** adopt vs. reference must be resolved at Phase 1 start — it gates the FFI vocabulary everything builds on.

## Sources

### Primary (HIGH confidence)
- `NVIDIA/NVTX` release-v3 source — `nvtxTypes.h`/`nvtxInit.h` (injection ABI, function tables, `NVTX_INJECTION64_PATH`), `nvToolsExtPayload.h` (payload schemas), `rust/crates/nvtx-sys/build.rs` (toolchain precedent)
- NVTX C API Reference + Nsight Systems User Guide — range/domain/category semantics, rendering conventions, `nvtx-include/exclude`
- docs.rs / crates.io — bindgen 0.72.1, libloading 0.9.0, cargo_metadata 0.23.1, cc 1.2.66 (versions verified current)
- `.planning/PROJECT.md` + `.planning/codebase/{ARCHITECTURE,STRUCTURE,CONCERNS,INTEGRATIONS}.md` — Quent spine, PR #87 prior art, analyzer panic locations, scope constraints
- Perfetto docs — track-per-thread / instant-event / nesting conventions

### Secondary (MEDIUM confidence)
- CUPTI docs — `InitializeInjectionNvtx2` contract, once-per-process init, thread-safety notes
- CUPTI-NVTX tutorial + GPU-profiling survey (eunomia) — single-injection-slot mechanics, Score-P/TAU/HPCToolkit positioning

### Tertiary (LOW confidence)
- Training-data FFI/linkage knowledge (weak/strong symbol resolution details, static init order) — validate empirically in Phase 1

---
*Research completed: 2026-07-08*
*Ready for roadmap: yes*
