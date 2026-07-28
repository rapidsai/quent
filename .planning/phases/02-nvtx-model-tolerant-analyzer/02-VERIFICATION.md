---
phase: 02-nvtx-model-tolerant-analyzer
verified: 2026-07-28T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run the real-capture roundtrip test"
    expected: "pixi run cargo test -p nvtx-analyzer --features real-capture-tests passes (24/24 tests green), confirming end-to-end reconstruction of a live NVTX capture"
    why_human: "The roundtrip links nvtx-injection whose build script runs bindgen against pixi-pinned nvtx-c headers. Execution requires the pixi toolchain (nvtx-c, libclang) which is not available in the verification environment. Code structure and feature gating are verified; test execution is not."
---

# Phase 2: NVTX Model & Tolerant Analyzer — Verification Report

**Phase Goal:** Stand up the hand-written, framework-free NVTX reconstruction core (`nvtx-analyzer`) and prove that it reconstructs a captured NVTX event stream — Start/End ranges, Push/Pop ranges, marks, domains, threads, categories, and resources — into a labeled, query-able `NvtxModel` with tolerance for malformed/out-of-order streams, using no `quent-analyzer`/`quent-model`/DSL dependency.

**Verified:** 2026-07-28
**Status:** HUMAN_NEEDED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| #   | Truth | Status | Evidence |
|-----|-------|--------|----------|
| 1 | Push/Pop events reconstruct into per-thread nested range stacks (Pop matches most recent Push on same thread); RangeStart/RangeEnd pairs match process-wide by handle | ✓ VERIFIED | `ranges.rs`: `type StackKey = (u32, u64)` — `PushPopRanges { stacks: HashMap<StackKey, Vec<OpenPushSpan>> }`. `StartEndRanges { open: HashMap<u64, OpenStartRange> }` keyed on `range_id` alone. Both dispatched in `model.rs` pass-2. |
| 2 | Registered-string, domain, and category handles resolve with correct namespacing, stable placeholders when unresolved | ✓ VERIFIED | `tables.rs`: `registered_strings: HashMap<(u64, u64), String>` (domain,handle key — never bare handle); `category_names: HashMap<(u64, u32), String>` (domain,category key). Placeholder helpers are pure functions of raw ids: `"<domain 0x{X}>"`, `"<unregistered string 0x{X}>"`, `"<category {n} @ domain 0x{X}>"`. `DEFAULT_DOMAIN_NAME = "default domain"`. |
| 3 | Stream containing unclosed ranges, out-of-order events, and duplicate timestamps analyzes to completion with no panic or abort | ✓ VERIFIED | `model.rs` feeds all events into `TimeOrderedCollector` (line 151), handles out-of-order and duplicates. All three reconstruction types (`StartEndRanges`, `PushPopRanges`, `Resources`) have `close_at_trace_end()`. `warn!`+skip for every orphan path. `grep -rn "unwrap()\|expect(\|panic!" src/` returns 0 matches across all 8 source files. |
| 4 | Range statistics computed per range name/domain/category: count and total/avg/min/max duration | ✓ VERIFIED | `stats.rs`: `struct RangeStats { count, total_duration, avg_duration, min_duration, max_duration, synthetic_count }`. `struct StatsKey { name, domain, category }`. `range_statistics(&spans)` folds over PushPop and StartEnd spans only; `model.rs` exposes `range_statistics()` accessor (lines 93–95). |
| 5 | Marks, domains, threads, and categories represented in the NVTX Quent model so the analyzer can group and label ranges | ✓ VERIFIED | `span.rs`: `struct NvtxMark`, `struct NvtxDomain`, `struct NvtxThread`, `struct NvtxCategory`. `model.rs`: `marks()`, `domains()`, `threads()`, `categories()`, `category_name()`, `thread_name()` accessors. |

**Score: 5/5 truths verified**

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `integrations/nvtx/events/src/lib.rs` | `thread_id: u32` on RangePush and RangePop | ✓ VERIFIED | Lines 41 and 50 show `thread_id: u32` on both variants; same u32 id space as NameThread (line 108). |
| `integrations/nvtx/injection/src/init.rs` | `current_thread_id()` OS-thread-id helper | ✓ VERIFIED | Two `current_thread_id()` impls (lines 45 and 60): `#[cfg(target_os = "linux")]` uses `libc::syscall(libc::SYS_gettid) as u32`; non-Linux fallback uses std `ThreadId`. Unit test `current_thread_id_is_stable_and_nonzero` present (line 567). |
| `integrations/nvtx/analyzer/src/span.rs` | Own `NvtxSpan / SpanKind` types | ✓ VERIFIED | `struct NvtxSpan` (line 41), `enum SpanKind { PushPop, StartEnd, Resource }` (line 26). Hand-written, no framework types. |
| `integrations/nvtx/analyzer/src/model.rs` | `NvtxModel + NvtxModelBuilder` two-pass entry | ✓ VERIFIED | `NvtxModelBuilder::build()` implements two passes: 1a `TimeOrderedCollector`, 1b `ResolutionTables::build()`, pass 2 replay with full event dispatch. Returns `NvtxModelResult<NvtxModel>`. |
| `integrations/nvtx/analyzer/src/ranges.rs` | RangeStart/RangeEnd by range_id; push/pop by (thread_id, domain) | ✓ VERIFIED | `StartEndRanges::start(range_id, ...)` / `end(range_id, ...)`. `PushPopRanges::push(id, thread_id, domain, ...)` / `pop(thread_id, domain, ...)`. Both substantive and wired from model.rs pass-2. |
| `integrations/nvtx/analyzer/src/tables.rs` | Pass-1 handle-resolution tables + `fn resolve` | ✓ VERIFIED | `ResolutionTables::build()`, `resolve_message()`, `resolve_domain()`, `resolve_category()`, `resolve_thread()`. Registered strings keyed by `(domain, handle)` (line 168). Category names keyed by `(domain, category)`. Placeholder policy as pure functions. |
| `integrations/nvtx/analyzer/src/resource.rs` | Resource lifespan matching by handle + `identifier_type` labels | ✓ VERIFIED | `Resources { open: HashMap<u64, OpenResource> }` — handle alone, not `(domain, handle)` (forced by vocabulary: `ResourceDestroy` carries no domain). `label_identifier_type(i32)` encodes `NVTX_RESOURCE_MAKE_TYPE(CLASS, INDEX)` rule; unknown types pass through as `"<identifier_type {n}>"` (line 73). |
| `integrations/nvtx/analyzer/src/stats.rs` | `RangeStats` per-(name,domain,category) aggregation | ✓ VERIFIED | `struct RangeStats`, `struct StatsKey`, `range_statistics()` fold with `BTreeMap` for deterministic iteration. Marks and resource spans filtered out. `avg_duration` uses `checked_div` (line 97). |
| `integrations/nvtx/analyzer/tests/roundtrip.rs` | Feature-gated real-capture roundtrip | ✓ EXISTS | File present; gated `#![cfg(feature = "real-capture-tests")]`; feature declared in `Cargo.toml` line 24. Dependencies (`nvtx-example`, `quent-instrumentation`) declared as optional regular deps (not dev-deps, Cargo limitation noted in SUMMARY). Execution requires pixi — see Human Verification. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `injection/src/callbacks.rs` | `convert::range_push / convert::range_pop` | `current_thread_id()` on app thread | ✓ WIRED | `grep -n "current_thread_id"` in callbacks.rs returns 5 call sites (matches plan requirement exactly). |
| `injection/src/init.rs` | `libc::gettid` | OS thread-id syscall | ✓ WIRED | `libc::syscall(libc::SYS_gettid)` (line 51). Used raw syscall instead of `libc::gettid()` wrapper (glibc < 2.30 compatibility — documented deviation). |
| `analyzer/src/model.rs` | `quent_time::TimeOrderedCollector` | timestamp-ordered replay | ✓ WIRED | `use quent_time::{TimeOrderedCollector, ...}` (line 19); `TimeOrderedCollector::default()` (line 151); `collector.extend(events)` + `into_inner()`. |
| `analyzer/Cargo.toml` | nvtx-events, nvtx-bridge, quent-events, quent-time | path deps only | ✓ WIRED | Deps confirmed: path deps only for these four. No `quent-analyzer`, `quent-model`, `quent-schema`, or `model-macros`. `grep -c "quent-analyzer\|quent-model\|quent-schema\|model-macros" Cargo.toml` = 0. |
| `analyzer/src/model.rs` | `tables.rs` pass-1 lookups | `resolve_message()` in pass-2 | ✓ WIRED | `resolve_message` called at RangeStart (line 181), RangePush (line 196), Mark (line 211), ResourceCreate (line 229). `resolve_category` and `resolve_thread` exposed on NvtxModel. |
| `tables.rs` registered strings | `(domain, handle)` key | per-domain namespacing | ✓ WIRED | `registered_strings.insert((*domain, *handle), string.clone())` (line 168). `get(&(domain, *handle))` in resolve. |
| `ranges.rs` | `(thread_id, domain)` stack key | RangePush push / RangePop pop innermost | ✓ WIRED | `type StackKey = (u32, u64)` (line 134); `HashMap<StackKey, Vec<OpenPushSpan>>` (line 187); `entry((thread_id, domain))` at push (line 202); `get_mut(&key).and_then(Vec::pop)` at pop (line 231). |
| `model.rs` | `ranges.rs` push/pop dispatch | pass-2 replay | ✓ WIRED | `NvtxEvent::RangePush` arm (line 191) and `NvtxEvent::RangePop` arm (line 201) dispatch to `pushes.push()` and `pushes.pop()`. |
| `resource.rs` | handle-keyed Create/Destroy matching | ResourceDestroy carries only handle | ✓ WIRED | `HashMap<u64, OpenResource>` (line 121); `self.open.insert(handle, open)` at create; `self.open.remove(&handle)` at destroy. Domain recovered from create. |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `NvtxModel::spans()` | `spans: Vec<NvtxSpan>` | Two-pass `NvtxModelBuilder::build()` over event stream; slots filled by `StartEndRanges`, `PushPopRanges`, `Resources` close paths | Yes — fold over real event data; three separate reconstruction paths produce NvtxSpan values | ✓ FLOWING |
| `NvtxModel::marks()` | `marks: Vec<NvtxMark>` | Pass-2 `Mark` event arm builds NvtxMark with resolved name, timestamp, category | Yes — produces from real events | ✓ FLOWING |
| `NvtxModel::range_statistics()` | `BTreeMap<StatsKey, RangeStats>` | `stats::range_statistics(&self.spans)` — on-demand fold over spans | Yes — computed from real span data | ✓ FLOWING |
| `ResolutionTables` | registered_strings / category_names / domain_names | Pass-1b `ResolutionTables::build(&ordered)` scans all events | Yes — populated from DomainCreate/RegisterString/NameCategory/NameThread events | ✓ FLOWING |

---

### Behavioral Spot-Checks

Step 7b: SKIPPED — the unit tests (`cargo test -p nvtx-analyzer`) and the roundtrip (`pixi run cargo test -p nvtx-analyzer --features real-capture-tests`) require the cargo/pixi build toolchain. Cannot run in the verification environment. Code structure and wiring are fully verified at Levels 1–4.

---

### Probe Execution

Step 7c: No `scripts/*/tests/probe-*.sh` probes declared for this phase. SKIPPED.

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| MOD-01 | 02-02 | NVTX ranges modeled as Quent FSMs with a single "range open" state | ✓ SATISFIED (with NOTE) | REQUIREMENTS.md text says "Quent FSMs" but Design Decision D-06 deliberately chose framework-free plain `NvtxSpan`. ROADMAP SC #1 (the phase contract) says "reconstruct into nested range stacks" — not FSMs. The ROADMAP reflects D-06; REQUIREMENTS.md was not updated. Implementation satisfies ROADMAP SC; see note below. |
| MOD-02 | 02-03, 02-05 | Marks, domains, threads, and categories represented in the NVTX Quent model | ✓ SATISFIED | `NvtxMark`, `NvtxDomain`, `NvtxThread`, `NvtxCategory` in span.rs; `resources()` from SpanKind::Resource in model.rs. All accessible via model accessors. |
| ANA-01 | 02-03 | Resolver resolves registered-string handles from event stream | ✓ SATISFIED | `tables.rs::registered_strings` keyed `(domain, handle)`; `resolve_message()` with unregistered-handle placeholder. Forward references resolve because pass 1 is complete before pass 2 starts. |
| ANA-02 | 02-03 | Domain and category name resolution with `(domain, categoryId)` namespacing | ✓ SATISFIED | `category_names: HashMap<(u64, u32), String>` — never a global key. `resolve_category(domain, category)` returns `None` for category 0. |
| ANA-03 | 02-01, 02-04 | Push/Pop ranges reconstruct as per-thread nested stacks | ✓ SATISFIED | `PushPopRanges { stacks: HashMap<(u32, u64), Vec<OpenPushSpan>> }`. Pop is `get_mut(&(thread_id, domain)).and_then(Vec::pop)` — a pop on thread B cannot close thread A's push. |
| ANA-04 | 02-02 | RangeStart/RangeEnd match process-wide by handle, across threads | ✓ SATISFIED | `StartEndRanges { open: HashMap<u64, OpenStartRange> }` — `range_id` alone is the key; domain on RangeEnd is explicitly ignored ("redundant"). |
| ANA-05 | 02-02, 02-04 | Tolerant reconstruction: open-at-trace-end closed, no panic on malformed streams | ✓ SATISFIED | All three reconstruction types have `close_at_trace_end()`. Orphan closes: `warn!` + skip. `unwrap()/expect()/panic!` in src/ = 0. `TimeOrderedCollector` handles out-of-order + duplicate timestamps. |
| ANA-06 | 02-05 | Range statistics: count and total/avg/min/max per name/domain/category | ✓ SATISFIED | `stats::range_statistics()` returns `BTreeMap<StatsKey, RangeStats>`. `StatsKey { name, domain, category }`. Marks and resource spans filtered by kind. `synthetic_count` tracked separately. |

**MOD-01 NOTE:** The REQUIREMENTS.md text says "Quent FSMs with a single 'range open' state, in an NVTX domain mirroring the `domains/query_engine/` layout." The implementation deliberately chose plain `NvtxSpan` structs (D-06, documented in 02-RESEARCH.md). The ROADMAP Phase 2 success criteria (the verification contract) do not mention Quent FSMs and instead describe the tolerant reconstruction behavior — which IS satisfied. REQUIREMENTS.md should be updated to reflect D-06 (suggested: "NVTX ranges are modeled as reconstructed `NvtxSpan` intervals in the framework-free `nvtx-analyzer` crate, capturing start/end, push/pop, and resource lifespans."). This is a documentation gap, not a code gap — the ROADMAP contract is met.

---

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| No files | No TBD/FIXME/XXX markers found in any modified file | — | None |
| No files | No `unwrap()/expect()/panic!` in any `src/` file (0 matches across all 8 source files) | — | None |
| No files | No `quent_analyzer`/`quent_model`/`RtFsm` in any `src/` file (0 matches across all 8 source files) | — | None |
| No files | No `occupancy`/`capacity`/`utilization` in any `src/` file (D-10 compliance) | — | None |

---

### Human Verification Required

#### 1. Real-Capture Roundtrip Test

**Test:** In the pixi environment, run:
```
pixi run cargo test -p nvtx-analyzer --features real-capture-tests example_capture_roundtrip
```

**Expected:**
- Test passes (1 passed, 0 failed)
- The reconstructed model contains: a named thread (from NameThread), a "startup" mark (not a span), a "phase-1" PushPop span with a nonzero `thread_id`, a "phase-2" StartEnd span
- `range_statistics()` reports both ranges ("phase-1" and "phase-2") in the output

**Why human:** The roundtrip test links `nvtx-injection` whose `build.rs` runs bindgen against pixi-pinned nvtx-c headers and requires libclang. The verification environment does not have the pixi toolchain. The code structure, feature gating, and test file content have been verified; test execution has not. The SUMMARY claims this passed (`pixi run cargo test -p nvtx-analyzer --features real-capture-tests` → 24 passed, 0 failed), but execution evidence is required per the goal-backward methodology.

---

### Gaps Summary

No gaps found. All 5 ROADMAP success criteria are verified in the codebase. All 8 requirement IDs (MOD-01, MOD-02, ANA-01, ANA-02, ANA-03, ANA-04, ANA-05, ANA-06) are satisfied. The single blocking blocker pattern — the one item that cannot be verified programmatically — is the real-capture roundtrip which requires the pixi toolchain. All unit-level code (23/24 tests' code paths) is substantive, wired, and data-flowing.

The MOD-01 / REQUIREMENTS.md divergence (FSMs vs plain NvtxSpan) is a documentation gap in REQUIREMENTS.md, not a code gap. The ROADMAP success criteria — which are the phase contract — are all met.

---

_Verified: 2026-07-28_
_Verifier: Claude (gsd-verifier)_
