# Feature Research

**Domain:** NVTX trace consumption & visualization tooling (profiler-class trace ingestion for Quent)
**Researched:** 2026-07-08
**Confidence:** HIGH (NVTX API surface + Nsight Systems behavior verified against official docs; Perfetto conventions verified; HPC-tool NVTX support MEDIUM — survey-level sources)

## Orientation

The reference consumers for NVTX data are **Nsight Systems** (the canonical, most complete
consumer), **Perfetto**-based flows (the dominant open trace-viewer convention), and the HPC
tracing tools (**Score-P/Vampir, TAU, HPCToolkit**) that treat NVTX as one annotation source
among many. Quent is not trying to beat Nsight at GPU profiling — it is bringing NVTX ranges
from GPU data-processing libraries (libcudf, cuCascade) into a telemetry pipeline that already
has span reconstruction and a timeline UI. So "table stakes" here means *the semantics an NVTX
emitter already relies on* — if Quent drops or misrenders them, the trace is simply wrong, and
users who cross-check against Nsight will not trust it.

The feature set splits cleanly along the pipeline the project already defines:
capture → model → analyzer (handle resolution + span reconstruction) → endpoint → UI. Table
stakes are distributed across all stages; a correct swim-lane render (UI) is worthless if
handle resolution (analyzer) is wrong.

## Feature Landscape

### Table Stakes (Users Expect These)

Features an NVTX emitter and anyone who has used Nsight assumes exist. Missing these = the
trace is incomplete or wrong, and users leave for Nsight.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Per-thread swim lanes** | Push/Pop ranges are inherently per-thread; Nsight/Perfetto both render one lane per OS thread. A single merged lane destroys the meaning. | MEDIUM | Analyzer must key push/pop state by `(tid, domain)`. UI already has `OperatorGanttChart` (spans per row) — maps directly to lanes. |
| **Push/Pop nested-stack reconstruction** | Pop auto-associates with the most recent Push on the *same thread*. Nesting IS the data — users read call-hierarchy from depth. | MEDIUM | Per-thread stack (LIFO). Depth → vertical nesting in the lane. This is the core of the FSM "range open" state model. |
| **Start/End cross-thread range matching** | Start returns a handle; End (possibly on another thread) passes it. Ranges may overlap arbitrarily. Distinct from push/pop and rendered on per-*process* rows in Nsight. | MEDIUM | Handle→open-range map keyed process-wide. Cannot reuse the per-thread stack path. |
| **Handle resolution (registered strings)** | NVTX registers a message once, returns a handle, then ranges reference the handle. Unresolved handle = ranges labeled with opaque integers. | MEDIUM | Analyzer must build a handle→string table from the event stream (capture emits raw calls verbatim per PR #87 design). Registration may arrive before or interleaved with use. |
| **Domain resolution & grouping** | Every annotation is scoped to a domain (a library's namespace). Domains carry category-ID namespacing. Tools group/label by domain. | MEDIUM | Handle→domain-name table. Domain is the top-level grouping key in the UI, above threads in most layouts. |
| **Category resolution & rendering** | Categories are named by ID *within a domain*; same ID in different domains is distinct. Nsight shows category name alongside events. | LOW-MEDIUM | `(domain, categoryId)`→name table. Namespacing bug risk: do not resolve category globally. |
| **Message / label rendering on ranges** | The whole point — a range without its message is unreadable. Supports both inline strings and registered-string handles. | LOW | Two code paths (immediate ASCII/UTF string vs handle lookup). |
| **Marks as instant events** | `nvtxMark` annotates a point in time (not a range). Standard render is a vertical tick/instant marker on the lane. | LOW | Perfetto "instant event" convention. Distinct rendering from spans. |
| **Color rendering** | NVTX event attributes carry an ARGB color; users deliberately color-code ranges and expect that color honored (Nsight does). | LOW | Pass color through capture→model→UI. Provide a deterministic fallback palette (by category/domain) when unset. |
| **Thread naming** | `nvtxNameOsThread` labels a thread; Nsight uses it as the lane label. Without it lanes are bare TIDs. | LOW | Name event → lane-label override, keyed by tid. |
| **Unclosed-range tolerance at process exit** | Real streams routinely end with ranges still open (crash, early exit, deliberate). Nsight renders these as running to end-of-trace. **The existing analyzer panics on incomplete FSM lifecycles (`fsm/runtime.rs:309-313`) — this is a hard blocker.** | MEDIUM-HIGH | Must NOT inherit the panic. Open-at-EOF ranges → close at trace end (or render as unbounded). Called out as a known hazard in PROJECT.md CONCERNS. |
| **Out-of-order / duplicate-timestamp tolerance** | Cross-thread event streams are not globally ordered; timestamps collide. Existing analyzer panics on this. | MEDIUM | Sort/tolerate rather than assert. Must be designed in, not bolted on. |
| **Time-axis zoom / pan / shared ruler** | Baseline trace-viewer interaction. Quent UI already has `TimelineController`/`TimelineRuler`. | LOW | Reuse existing machinery — near-free. |
| **Range duration / hover tooltip** | Users expect to hover a span and see message, duration, thread, domain, category, payload. | LOW | Standard span detail panel. |

### Differentiators (Competitive Advantage)

Features that set Quent apart from "just run Nsight" — aligned with Core Value (coexistence +
integration into an existing analytical pipeline).

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Multi-consumer fan-out (coexist with Nsight/AON)** | THE headline differentiator. NVTX allows exactly one injection library per process (invariant, like CUPTI). Quent installs itself as that one, keeps per-sink shadow tables, and passes through the external tool named by `NVTX_INJECTION64_PATH`. Lets a user profile with Nsight *and* feed Quent simultaneously — impossible today. | HIGH | Core hard problem (Johan/Lawrence design, never prototyped). Everyone else forces you to pick one tool. |
| **Payload extension display (schemas / enums / binary payloads)** | Payloads carry the *structured, analytically useful* data (row counts, sizes, IDs). Nsight shows them minimally; a pipeline with an analyzer can index, filter, and aggregate on them. This is where "trace" becomes "queryable telemetry." | HIGH | Requires parsing `nvtxPayloadSchemaRegister`/enum registration + binary payload decode against registered schema. PR #87 defers this to "Phase 5." High value, high cost — see MVP note. |
| **Domain / category filtering** | Profiling across multiple libraries, users want only some domains. Nsight supports `nvtx-include`/`nvtx-exclude`. Table stakes *for Nsight parity*, differentiating vs a naive first render. | MEDIUM | Filter at query/UI layer once domain resolution exists. |
| **Range statistics / aggregation** | Count, total/avg/min/max duration per range name/domain/category. Nsight's stats reports are heavily used. Quent's analyzer already produces "binned timelines" — natural fit. | MEDIUM | Leverages existing analyzer aggregation. Higher analytical value than raw timeline alone. |
| **Integration with existing Quent domains (operator correlation)** | Tie NVTX ranges to query-plan operators (libcudf range ↔ Quent operator). Explicitly the "cherry on top" and **out of scope for v1** per PROJECT.md, but the strategic differentiator long-term — no other tool knows about Quent's query model. | HIGH | Deferred. Keep model boundaries clean so this is addable. |
| **Deterministic in-repo trace source for CI** | An NVTX test app that emits a known event stream lets the whole pipeline be tested without a GPU. Not a user feature but a durable engineering differentiator vs GPU-only validation. | MEDIUM | Called out as required in PROJECT.md. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Reinterpret / resolve handles at capture time** | Seems efficient to resolve strings/domains where captured. | Registration and use interleave arbitrarily and cross threads; capture-time resolution races and loses fidelity. PR #87 deliberately mirrors raw calls verbatim and defers resolution to the analyzer. | Capture raw, resolve in analyzer where the full stream is visible. |
| **Rebuild Nsight's GPU/CUDA correlation** | "It's a profiler, show me kernels/memcpy/CUDA API." | NVTX injection gives you NVTX events only — not CUDA runtime/driver/kernel activity (that's CUPTI). Trying to reconstruct GPU timelines from NVTX is out of scope and impossible from this data source. | Position Quent as *complementary* to Nsight (that's why coexistence exists). Correlate NVTX↔Quent operators, not NVTX↔kernels. |
| **Strict trace validation (reject malformed streams)** | "A well-formed trace should nest perfectly and close all ranges." | Real NVTX telemetry has unclosed ranges, out-of-order events, duplicate timestamps as a matter of course. Strictness = the exact panics that already exist in the analyzer. | Tolerant reconstruction: close-at-EOF, stable-sort, best-effort matching. Log anomalies, never abort. |
| **Live / streaming device-side NVTX** | GPU-side annotations sound powerful. | Device-side NVTX isn't publicly available (~end 2026 per Akash Goel) and lands with limited parity (8-byte payloads, const-literal strings). Building for it now is speculative. | Host-side only for v1; keep model extensible. Explicitly out of scope in PROJECT.md. |
| **Windows support** | Broad platform coverage. | NVTX injection relies on weak-symbol override / `NVTX_INJECTION64_PATH`; `wchar_t` size + linker semantics break on Windows (PR #87 already compile-errors). | Linux (+ maybe macOS) 64-bit only. Documented constraint. |
| **Custom NVTX-emitting UI / editor** | "Let users annotate." | Quent is a *consumer*, not an emitter. Emission is the instrumented app's job (NVTX library / RAPIDS already emit). | Consume only. |
| **Own trace file format / export to Nsight** | "Interop with the ecosystem." | Reinventing `.nsys-rep`/Perfetto protobuf is large and low-value when Quent has its own pipeline. | Use Quent's existing event/exporter formats (ndjson/msgpack/postcard). Perfetto export is a possible *future* nicety, not v1. |

## Feature Dependencies

```
Capture (raw NVTX calls verbatim)
    └──requires──> Fan-out mediator (to be the single injection consumer at all)
                       └──requires──> external-tool passthrough (NVTX_INJECTION64_PATH)

Handle resolution (strings / domains / categories)
    └──requires──> Capture emits registration events verbatim
    └──enables───> Message rendering, Domain grouping, Category rendering, Filtering

Push/Pop stack reconstruction ──requires──> Per-thread keying + tolerant ordering
Start/End matching           ──requires──> Process-wide handle map + tolerant ordering
    └──both feed──> Span model ("range open" FSM state)
                        └──requires──> Unclosed-range tolerance (NOT the current panic)
                        └──enables───> Swim-lane render, Duration/stats, Tooltips

Payload display ──requires──> Payload schema/enum resolution (analyzer)
              └──requires──> Capture actually emits payload-extension events (PR #87 "Phase 5")

Domain/category filtering ──enhances──> Swim-lane render (needs resolution first)
Range statistics          ──enhances──> Span model (reuses binned-timeline analyzer)
Operator correlation      ──enhances──> Span model + existing query_engine domain (v2)
```

### Dependency Notes

- **Everything downstream requires the fan-out mediator to even function:** if Quent can't
  become the single injection consumer (or breaks the existing Nsight consumer by grabbing the
  slot), there is no event stream. This is the critical-path, highest-risk item — it gates the
  entire pipeline, so it must be proven early.
- **All rendering/labeling depends on analyzer-side handle resolution:** capture is deliberately
  "dumb" (raw calls). If resolution is wrong, every label, group, and filter is wrong. Resolution
  correctness > UI polish.
- **Unclosed-range tolerance is a prerequisite, not a feature:** the span model cannot exist over
  real NVTX data until the analyzer stops panicking on incomplete FSM lifecycles and out-of-order
  events. This must be sequenced before any UI work has real data to show.
- **Payload display conflicts with a tight MVP:** high value but high cost (schema parse + binary
  decode) and PR #87 never emitted it. It can be captured-but-not-rendered in v1, or deferred.

## MVP Definition

### Launch With (v1)

Minimum to prove the vertical slice: an NVTX-emitting app is observed end-to-end in Quent
without breaking Nsight.

- [ ] **Fan-out mediator + external-tool passthrough** — without it there is no data and no
  coexistence story; it is the differentiator and the critical path.
- [ ] **Full-surface raw capture** (push/pop, start/end, marks, domains, registered strings,
  category/thread naming, resources) — the PR #87 base already targets this. Payload capture
  wired even if not yet rendered.
- [ ] **Analyzer handle resolution** (strings, domains, categories) with correct namespacing.
- [ ] **Tolerant span reconstruction** — per-thread push/pop stacks + process-wide start/end
  matching, close-at-EOF, out-of-order/dup-timestamp tolerant. Removes the existing panic path.
- [ ] **HTTP endpoint** exposing reconstructed ranges (mirrors query_engine layout).
- [ ] **UI swim-lane render** — per-domain/per-thread lanes, nested spans, marks as instants,
  color + message + category, hover tooltip. Reuses `TimelineController`/`OperatorGanttChart`.
- [ ] **Deterministic in-repo NVTX test app** for GPU-less CI.

### Add After Validation (v1.x)

- [ ] **Payload extension rendering** (schema/enum/binary decode + display) — trigger: capture
  layer emitting payloads is stable; users ask "what's in this range?"
- [ ] **Domain / category filtering** — trigger: multi-library traces get noisy.
- [ ] **Range statistics / aggregation** — trigger: users want counts/durations, not just timeline.
- [ ] **Manual libcudf-style GPU validation** (required before calling v1 "done," but distinct
  from CI).

### Future Consideration (v2+)

- [ ] **Operator correlation** (NVTX ↔ query-plan operators) — defer: needs stable NVTX model
  first; explicitly "cherry on top."
- [ ] **Device-side NVTX ingestion** — defer: not publicly available until ~end 2026, limited parity.
- [ ] **Perfetto/Nsight interop export** — defer: low value while Quent's own pipeline suffices.
- [ ] **Upstreaming injection crate to NVIDIA/NVTX** — parallel track, keep crate separable.

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Fan-out mediator + passthrough | HIGH | HIGH | P1 |
| Full-surface raw capture | HIGH | MEDIUM | P1 |
| Handle resolution (str/domain/category) | HIGH | MEDIUM | P1 |
| Tolerant span reconstruction (no panic) | HIGH | MEDIUM-HIGH | P1 |
| Per-thread swim-lane render | HIGH | MEDIUM | P1 |
| Marks / color / thread-name render | MEDIUM | LOW | P1 |
| HTTP endpoint | HIGH | LOW | P1 |
| In-repo deterministic test app | MEDIUM | MEDIUM | P1 |
| Payload extension display | HIGH | HIGH | P2 |
| Domain / category filtering | MEDIUM | MEDIUM | P2 |
| Range statistics / aggregation | MEDIUM | MEDIUM | P2 |
| Operator correlation | HIGH | HIGH | P3 |
| Device-side NVTX | LOW (today) | HIGH | P3 |

## Competitor Feature Analysis

| Feature | Nsight Systems | Perfetto-based flows | Score-P / TAU / HPCToolkit | Quent's Approach |
|---------|----------------|----------------------|----------------------------|------------------|
| Push/pop swim lanes | Per-thread rows, nested | Per-thread tracks, nested slices (B/E must nest) | Timeline (Vampir) / calling-context | Per-(domain,thread) lanes via existing Gantt machinery |
| Start/end ranges | Separate per-process rows | Async tracks | Varies | Process-wide handle map, distinct rows |
| Domain grouping / filter | `nvtx-include/exclude`, per-domain enable | Track hierarchy grouping | Domain-aware (Score-P) | Domain = top grouping key + query-layer filter |
| Payload | Shown in event detail | Debug annotations | Metric/counter mapping | Analyzer-indexed, schema-decoded (v1.x) |
| Color / category | Honored + report columns | Slice colors | Limited | Honor color, fallback palette by category |
| Multi-consumer coexistence | Owns the injection slot (exclusive) | N/A (post-hoc file) | Owns the slot (exclusive) | **Fan-out mediator — coexist, not exclusive** |
| Unclosed-range handling | Renders to end-of-trace | Renders open / auto-closes | Tool-dependent | Close-at-EOF, tolerant (must fix analyzer panic) |
| GPU kernel/CUDA correlation | Yes (CUPTI) | Via importers | Yes (CUPTI/PC sampling) | **No — out of scope; complementary to Nsight** |

## Sources

- [NVTX C API Reference — Markers & Ranges](https://nvidia.github.io/NVTX/doxygen/group___m_a_r_k_e_r_s___a_n_d___r_a_n_g_e_s.html) — HIGH (push/pop vs start/end semantics, event attributes)
- [NVTX C API Reference — Domains](https://nvidia.github.io/NVTX/doxygen/group___d_o_m_a_i_n_s.html) — HIGH (domain namespacing, category ID scoping)
- [NVTX C API Reference — Resource Naming](https://nvidia.github.io/NVTX/doxygen/group___r_e_s_o_u_r_c_e___n_a_m_i_n_g.html) — HIGH (thread naming, resource objects, domain association)
- [NVTX project site](https://nvidia.github.io/NVTX/) — HIGH (registered strings, single-injection-library init model)
- [NVTX GitHub / nvtx3.hpp](https://github.com/NVIDIA/NVTX/blob/release-v3/c/include/nvtx3/nvtx3.hpp) — HIGH (payload schema macros, v3 per-binary init)
- [Nsight Systems User Guide](https://docs.nvidia.com/nsight-systems/UserGuide/index.html) — HIGH (timeline rendering, per-thread vs per-process rows, thread-name labeling, nvtx-include/exclude filtering, capture ranges, stats)
- [NVIDIA Nsight VSE — NVTX docs](https://archive.docs.nvidia.com/nsight-visual-studio-edition/2020.1/nvtx/index.html) — HIGH (domains as grouping, category names in reports, payload field)
- [Perfetto — synthetic track events / other formats](https://perfetto.dev/docs/reference/synthetic-track-event) — HIGH (track-per-thread convention, nesting requirement, instant events, track hierarchy grouping)
- [CUPTI NVTX Integration Tutorial (eunomia)](https://eunomia.dev/others/cupti-tutorial/cupti_nvtx/) — MEDIUM (single injection library / NVTX_INJECTION64_PATH mechanics, coexistence constraint)
- [GPU Profiling Under the Hood survey (eunomia)](https://eunomia.dev/blog/2025/04/21/gpu-profiling-under-the-hood-an-implementation-focused-survey-of-modern-accelerator-tracing-tools/) — MEDIUM (Score-P/TAU/HPCToolkit NVTX handling, tool positioning)
- `.planning/PROJECT.md` — project scope, PR #87 base, fan-out design, analyzer-panic hazard, out-of-scope list

---
*Feature research for: NVTX trace consumption & visualization*
*Researched: 2026-07-08*
