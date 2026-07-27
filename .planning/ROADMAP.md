# Roadmap: Quent NVTX Consumer

## Overview

The journey delivers one full vertical slice: an application emitting NVTX ranges is captured, reconstructed, and made visible in the Quent UI — without breaking that application's ability to be profiled by NSys/AON. We prove a single-consumer capture path end-to-end first (capture → tolerant analyzer → UI swim lanes), then insert the fan-out mediator *underneath* the proven slice so Quent can coexist with external consumers, and finally validate the whole pipeline against a real GPU library workload. Each phase completes a coherent, demonstrable capability; the two highest-risk items (tolerant analyzer, fan-out mediator) are isolated so a redesign in either does not ripple.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Capture Foundation** - FFI vocabulary + single-consumer injection + deterministic test app + subprocess harness ([#371](https://github.com/rapidsai/quent/issues/371)) (completed 2026-07-14)
- [ ] **Phase 2: NVTX Model & Tolerant Analyzer** - Model raw events as FSM ranges and reconstruct them tolerantly without panics ([#372](https://github.com/rapidsai/quent/issues/372))
- [ ] **Phase 3: Server & UI Swim Lanes** - Expose and render reconstructed NVTX ranges in the Quent UI (slice complete) ([#373](https://github.com/rapidsai/quent/issues/373))
- [ ] **Phase 4: Fan-out Mediator & Passthrough** - Coexist with external NVTX consumers via per-sink shadow tables ([#374](https://github.com/rapidsai/quent/issues/374))
- [ ] **Phase 5: Real-Workload Validation** - Prove the pipeline against a libcudf-style GPU workload before v1 is done ([#375](https://github.com/rapidsai/quent/issues/375))

## GitHub Tracking

Parent issue: [#76 Consume NVTX ranges](https://github.com/rapidsai/quent/issues/76). Each phase is a native sub-issue of #76; finer sub-issues are added under a phase's issue as its work is scoped.

| Phase | Issue |
|-------|-------|
| 1. Capture Foundation | [#371](https://github.com/rapidsai/quent/issues/371) |
| 2. NVTX Model & Tolerant Analyzer | [#372](https://github.com/rapidsai/quent/issues/372) |
| 3. Server & UI Swim Lanes | [#373](https://github.com/rapidsai/quent/issues/373) |
| 4. Fan-out Mediator & Passthrough | [#374](https://github.com/rapidsai/quent/issues/374) |
| 5. Real-Workload Validation | [#375](https://github.com/rapidsai/quent/issues/375) |

## Phase Details

### Phase 1: Capture Foundation
**Goal**: A running NVTX-emitting application is captured by Quent's injection library with raw events flowing into the standard pipeline, driven by a deterministic in-repo test app under GPU-less CI.
**Mode:** mvp
**Depends on**: Nothing (first phase)
**Requirements**: CAP-01, CAP-02, CAP-03, CAP-04, CAP-05, VAL-01, VAL-02
**Success Criteria** (what must be TRUE):
  1. Running the in-repo NVTX test app with Quent's injection library attached (via `NVTX_INJECTION64_PATH` or link-time, no app code changes) produces a stream of raw NvtxEvents covering push/pop ranges, start/end ranges, marks, domain create/destroy, registered strings, category naming, thread naming, and resource create/destroy.
  2. The CORE `nvtxEventAttributes` payload union is captured verbatim (undecoded) on the events that carry it. The payload-**extension** module (schema registration, enum registration, binary `nvtxPayloadData_t`) is DEFERRED to a later phase — natural home alongside PAY-01 decode (re-scoped per D-12; libcudf emits zero payload-extension events today).
  3. Every captured event carries a capture-time timestamp and reaches a standard exporter through `EventSender`.
  4. Injection and fan-out integration tests run green in subprocesses under CI with no GPU hardware present.
  5. The test app runs to completion under high-frequency emission with no capture-path blocking on locks, I/O, or unbounded allocation (stamp-and-hand-off demonstrated).
**Plans**: 4 plans
Plans:
- [ ] 01-capture-foundation-01-PLAN.md — quent-nvtx-events verbatim vocabulary + workspace registration + D-12 payload re-scope
- [ ] 01-capture-foundation-02-PLAN.md — quent-nvtx-injection cdylib: bindgen/git-dep headers, InitializeInjectionNvtx2 one-shot, push/pop verbatim callbacks (CAP-01/02/03)
- [ ] 01-capture-foundation-03-PLAN.md — quent-nvtx bridge (bounded ring + drop-count + drain) + thin end-to-end push/pop subprocess proof (CAP-04/05, VAL-01/02)
- [ ] 01-capture-foundation-04-PLAN.md — widen to full multi-threaded core NVTX coverage + assertions + bindings-regen README (CAP-02/03, VAL-01, D-14)

### Phase 2: NVTX Model & Tolerant Analyzer
**Goal**: Captured raw events reconstruct into labeled NVTX ranges, marks, and statistics — tolerating the malformed and out-of-order telemetry real streams contain, without panicking.
**Mode:** mvp
**Depends on**: Phase 1 (consumes `quent-nvtx-events` vocabulary; can start against synthetic fixtures)
**Requirements**: MOD-01, MOD-02, ANA-01, ANA-02, ANA-03, ANA-04, ANA-05, ANA-06
**Success Criteria** (what must be TRUE):
  1. Push/Pop events reconstruct into per-thread nested range stacks (Pop matches the most recent Push on the same thread), and RangeStart/RangeEnd pairs match process-wide by handle even when start and end occur on different threads.
  2. Registered-string, domain, and category handles resolve to their names — categories namespaced by `(domain, categoryId)`, never globally — with stable placeholders when a handle is unresolved.
  3. A stream containing unclosed ranges, out-of-order events, and duplicate timestamps analyzes to completion with no panic or abort; ranges left open at end-of-trace are closed at trace end.
  4. Range statistics (count and total/avg/min/max duration) are computed per range name/domain/category.
  5. Marks, domains, threads, and categories are represented in the NVTX Quent model so the analyzer can group and label ranges.
**Plans**: TBD

### Phase 3: Server & UI Swim Lanes
**Goal**: Reconstructed NVTX data is visible in the Quent UI as interactive swim-lane timelines, completing the demonstrable vertical slice.
**Mode:** mvp
**Depends on**: Phase 2
**Requirements**: UI-01, UI-02, UI-03, UI-04, UI-05
**Success Criteria** (what must be TRUE):
  1. HTTP endpoint(s) return reconstructed ranges, marks, and statistics as ts-rs typed views following the existing server layout (Axum routes, caching).
  2. Ranges render as nested spans in swim lanes grouped by domain and thread on the shared zoom/pan time axis.
  3. Marks render as instant events, NVTX-specified range colors are honored (with a deterministic fallback palette), and named threads label their lanes.
  4. Hovering a range shows its message, duration, thread, domain, and category.
  5. User can filter the displayed ranges by domain and category.
**Plans**: TBD
**UI hint**: yes

### Phase 4: Fan-out Mediator & Passthrough
**Goal**: Quent coexists with other NVTX consumers in a single process — external tools keep working unmodified while Quent captures simultaneously.
**Mode:** mvp
**Depends on**: Phase 1 (inserted under the proven single-consumer injection; recommended after Phase 3)
**Requirements**: FAN-01, FAN-02, FAN-03
**Success Criteria** (what must be TRUE):
  1. Multiple NVTX consumers observe a single process simultaneously through the mediator's per-sink shadow tables walked from the one real injection slot.
  2. An external injection library supplied via `NVTX_INJECTION64_PATH` (e.g. Nsight Systems) produces its correct trace unmodified while Quent captures the same process.
  3. A slow or erroring sink does not stop event delivery to the remaining sinks (isolation via `catch_unwind` at every FFI boundary).
**Plans**: TBD

### Phase 5: Real-Workload Validation
**Goal**: The full pipeline is proven against a real GPU library workload and documented before v1 is called done.
**Mode:** mvp
**Depends on**: Phase 3 (full slice) and Phase 4 (coexistence)
**Requirements**: VAL-03
**Success Criteria** (what must be TRUE):
  1. A libcudf-style GPU workload is captured, reconstructed, and viewed end-to-end in the Quent UI.
  2. The run's result — ranges observed, gaps, and fidelity versus expectation — is documented.
  3. Quent + Nsight coexistence is confirmed on the real workload (fan-out holds outside the deterministic test app).
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Capture Foundation | 4/4 | Completed | 2026-07-22 |
| 2. NVTX Model & Tolerant Analyzer | 0/TBD | Not started | - |
| 3. Server & UI Swim Lanes | 0/TBD | Not started | - |
| 4. Fan-out Mediator & Passthrough | 0/TBD | Not started | - |
| 5. Real-Workload Validation | 0/TBD | Not started | - |
