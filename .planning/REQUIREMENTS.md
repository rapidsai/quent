# Requirements: Quent NVTX Consumer

**Defined:** 2026-07-08
**Core Value:** An application emitting NVTX ranges can be observed by Quent end-to-end — events captured, reconstructed into a model, and visible in the Quent UI — without breaking that application's ability to also be profiled by NSys/AON.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Capture

- [ ] **CAP-01**: An application emitting NVTX v3 events is captured by the Quent injection library without any code changes to the application (attach via `NVTX_INJECTION64_PATH` or link-time)
- [ ] **CAP-02**: All core NVTX call types are captured verbatim as raw events: Push/Pop ranges, RangeStart/End, Marks, domain create/destroy, registered strings, category naming, thread naming, resource create/destroy
- [ ] **CAP-03**: The CORE `nvtxEventAttributes` payload union is captured verbatim (undecoded) on the events that carry it. The payload-**extension** module (schema registration, enum registration, binary `nvtxPayloadData_t`) is DEFERRED to a later phase — natural home alongside PAY-01 decode (re-scoped per D-12; libcudf emits zero payload-extension events today)
- [ ] **CAP-04**: Captured events are timestamped at capture and flow into Quent's standard event pipeline (`EventSender` → exporters/collector)
- [ ] **CAP-05**: Event capture never blocks the instrumented application's threads on locks, I/O, or unbounded allocation (stamp-and-hand-off hot-path discipline)

### Fan-out

- [ ] **FAN-01**: Multiple NVTX consumers can observe a single process simultaneously through the fan-out mediator (per-sink shadow tables walked from the one real injection slot)
- [ ] **FAN-02**: An external injection library supplied via `NVTX_INJECTION64_PATH` (e.g. Nsight Systems) continues to receive NVTX events unmodified as a passthrough sink alongside Quent
- [ ] **FAN-03**: A misbehaving sink (slow or erroring) does not stop event delivery to the remaining sinks

### Model

- [ ] **MOD-01**: NVTX ranges are modeled as Quent FSMs with a single "range open" state, in an NVTX domain mirroring the `domains/query_engine/` layout
- [ ] **MOD-02**: Marks, domains, threads, and categories are represented in the NVTX Quent model so the analyzer can group and label ranges

### Analysis

- [ ] **ANA-01**: Analyzer resolves registered-string handles to their string values from the event stream
- [ ] **ANA-02**: Analyzer resolves domain and category names with correct namespacing — categories keyed by `(domain, categoryId)`, never globally
- [ ] **ANA-03**: Push/Pop ranges reconstruct as per-thread nested stacks (Pop matches the most recent Push on the same thread)
- [ ] **ANA-04**: RangeStart/RangeEnd pairs match process-wide by handle, including when start and end occur on different threads
- [ ] **ANA-05**: Reconstruction is tolerant: ranges left open at end-of-trace are closed at trace end, and out-of-order or duplicate-timestamp events never panic or abort the analysis
- [ ] **ANA-06**: Range statistics are computed per range name/domain/category: count and total/avg/min/max duration

### Serving & UI

- [ ] **UI-01**: HTTP endpoint(s) expose reconstructed NVTX ranges, marks, and statistics following the existing server layout (ts-rs view types, Axum routes, caching)
- [ ] **UI-02**: Ranges render as nested spans in swim lanes grouped by domain and thread, on the shared zoom/pan time axis
- [ ] **UI-03**: Marks render as instant events; NVTX-specified range colors are honored (with a deterministic fallback palette); named threads label their lanes
- [ ] **UI-04**: Hovering a range shows its message, duration, thread, domain, and category
- [ ] **UI-05**: User can filter displayed ranges by domain and category

### Validation

- [ ] **VAL-01**: A deterministic in-repo NVTX test application exercises the full pipeline (capture → model → analyzer → endpoint) in CI without GPU hardware
- [ ] **VAL-02**: Injection and fan-out integration tests run in subprocesses (NVTX initialization is process-global one-shot state, incompatible with in-process `cargo test` threads)
- [ ] **VAL-03**: The pipeline is validated against a real GPU library workload (libcudf-style) with the result documented, before v1 is called done

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Payloads

- **PAY-01**: Analyzer decodes binary payloads against their registered schemas and enums
- **PAY-02**: Decoded payload contents display in the UI (range detail) and are queryable/aggregatable

### Correlation

- **COR-01**: NVTX ranges correlate to query-plan operators (libcudf range ↔ Quent operator) — Johan's "cherry on top"

### Ecosystem

- **ECO-01**: Injection crate offered upstream to NVIDIA/NVTX (parallel track; v1 only guarantees separability)
- **ECO-02**: Perfetto-format trace export for ecosystem interop

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Capture-time handle resolution | Registration and use interleave across threads; capture-time resolution races and loses fidelity — capture raw, resolve in the analyzer |
| GPU kernel / CUDA API correlation | NVTX injection yields NVTX events only; kernel/memcpy activity is CUPTI's job — Quent is complementary to Nsight, not a replacement |
| Strict trace validation (reject malformed streams) | Real NVTX streams routinely contain unclosed ranges and disorder; strictness reproduces the analyzer panics this project must eliminate |
| Windows support | NVTX injection relies on weak-symbol override / `NVTX_INJECTION64_PATH` semantics that don't exist on Windows |
| Device-side NVTX ingestion | Not publicly available until ~end of 2026; limited feature parity when it lands |
| Custom trace file format / Nsight export | Quent's existing event/exporter formats suffice; reinventing `.nsys-rep` is large and low-value |
| NVTX emission tooling | Quent is a consumer; emission is the instrumented application's job |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CAP-01 | Phase 1 | Pending |
| CAP-02 | Phase 1 | Pending |
| CAP-03 | Phase 1 | Pending |
| CAP-04 | Phase 1 | Pending |
| CAP-05 | Phase 1 | Pending |
| VAL-01 | Phase 1 | Pending |
| VAL-02 | Phase 1 | Pending |
| MOD-01 | Phase 2 | Pending |
| MOD-02 | Phase 2 | Pending |
| ANA-01 | Phase 2 | Pending |
| ANA-02 | Phase 2 | Pending |
| ANA-03 | Phase 2 | Pending |
| ANA-04 | Phase 2 | Pending |
| ANA-05 | Phase 2 | Pending |
| ANA-06 | Phase 2 | Pending |
| UI-01 | Phase 3 | Pending |
| UI-02 | Phase 3 | Pending |
| UI-03 | Phase 3 | Pending |
| UI-04 | Phase 3 | Pending |
| UI-05 | Phase 3 | Pending |
| FAN-01 | Phase 4 | Pending |
| FAN-02 | Phase 4 | Pending |
| FAN-03 | Phase 4 | Pending |
| VAL-03 | Phase 5 | Pending |

**Coverage:**
- v1 requirements: 24 total (the earlier "22" undercounted CAP-01..05 and VAL-01..03)
- Mapped to phases: 24
- Unmapped: 0 ✓

---
*Requirements defined: 2026-07-08*
*Last updated: 2026-07-09 after roadmap creation*
