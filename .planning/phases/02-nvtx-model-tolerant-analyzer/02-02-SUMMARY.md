---
phase: 02-nvtx-model-tolerant-analyzer
plan: 02
subsystem: analyzer
tags: [nvtx, analyzer, reconstruction, spans, tolerance, framework-free, rust]

# Dependency graph
requires:
  - phase: 01-capture-foundation
    provides: nvtx-events vocabulary, nvtx-bridge NvtxEventEntity
  - phase: 02-nvtx-model-tolerant-analyzer
    plan: 01
    provides: thread_id on RangePush/RangePop (not consumed here; StartEnd ranges are process-wide)
provides:
  - "nvtx-analyzer crate (members-only, framework-free reconstruction core)"
  - "Own plain span vocabulary: NvtxSpan, SpanKind {PushPop, StartEnd, Resource}, SpanId"
  - "NvtxModel + two-pass NvtxModelBuilder::build(events) -> NvtxModelResult<NvtxModel>"
  - "RangeStart/RangeEnd reconstruction matched by range_id alone (ANA-04)"
  - "Tolerance by construction: out-of-order replay, duplicate timestamps, synthetic close, orphan-end skip (ANA-05)"
  - "NvtxModelError / NvtxModelResult"
affects: [02-03, 02-04, 02-05, MOD-01, ANA-04, ANA-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "quent_time::TimeOrderedCollector as the timestamp-ordering primitive (no hand-rolled sort)"
    - "Anomalies are tracing::warn! + continue, never Err variants"
    - "end.max(start) clamp at span construction so duration() can never underflow"
    - "Placeholder names are pure functions of the raw handle (no counters/timestamps)"

key-files:
  created:
    - "integrations/nvtx/analyzer/Cargo.toml"
    - "integrations/nvtx/analyzer/src/lib.rs"
    - "integrations/nvtx/analyzer/src/error.rs"
    - "integrations/nvtx/analyzer/src/span.rs"
    - "integrations/nvtx/analyzer/src/model.rs"
    - "integrations/nvtx/analyzer/src/ranges.rs"
    - "integrations/nvtx/analyzer/tests/fixtures.rs"
    - "integrations/nvtx/analyzer/tests/reconstruction.rs"
  modified:
    - "Cargo.toml"
    - "Cargo.lock"

key-decisions:
  - "Match RangeStart to RangeEnd by range_id alone; the domain on a RangeEnd is redundant and deliberately ignored"
  - "Spans are ordered by completion (observed closes in close order, then synthetic closes sorted by start then range_id) for deterministic output"
  - "Unresolved registered-string handles render as the placeholder '<unregistered string 0x{H}>'; absent messages as '<unnamed>' (full resolution is plan 02-03)"
  - "thread_id is None on StartEnd spans — process-wide ranges carry no thread identity by definition"
  - "Category 0 (NVTX 'no category' sentinel) maps to None rather than Some(0)"

patterns-established:
  - "Reconstruction state lives in a small pub(crate) struct per range kind (StartEndRanges), returning spans rather than mutating the model"
  - "Two-pass shape: pass 1 materializes/orders, pass 2 replays — resolution tables slot into pass 1 later"

requirements-completed: [MOD-01, ANA-04, ANA-05]

# Metrics
duration: ~25 min
completed: 2026-07-28
---

# Phase 2 Plan 02: Framework-Free Reconstruction Core Summary

**A new members-only `nvtx-analyzer` crate replays captured NVTX events in timestamp order into plain `NvtxSpan`s, matching RangeStart/RangeEnd by `range_id` and tolerating malformed streams by construction — with zero dependency on the shared analyzer/model framework.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 2 (scaffold/RED, implement/GREEN)
- **Files created:** 8 (new crate) + 2 modified (root `Cargo.toml`, `Cargo.lock`)
- **Tests:** 5 passing

## Accomplishments

- Stood up `integrations/nvtx/analyzer` (`nvtx-analyzer`), registered in workspace `members` **only** — absent from `default-members`, so `cargo build` / `cargo test` are unchanged and the zero-cost default guarantee holds.
- Defined the crate's **own** plain span vocabulary (`NvtxSpan`, `SpanKind`, `SpanId`) with `duration()`. No shared runtime state machine, no shared-model entity, no macro DSL — the whole point of the off-legacy design.
- Implemented the two-pass `NvtxModelBuilder`: pass 1 materializes the stream in timestamp order via `quent_time::TimeOrderedCollector`; pass 2 replays and dispatches to the range reconstruction.
- Reconstructed process-wide ranges in `ranges.rs`, matching starts to ends by `range_id` alone — proven by a test whose `RangeEnd` deliberately carries a *different* domain than its `RangeStart` and still matches.
- Delivered all three tolerance guarantees (ANA-05) plus the two the plan's `<behavior>` block specified, each with a test.

## Task Commits

Each task committed atomically (DCO sign-off, Conventional Commits):

1. **Task 1: Scaffold crate + span types + fixtures + failing tests (RED)** — `86f2ebd` (test)
2. **Task 2: Two-pass builder + Start/End matching + tolerant replay (GREEN)** — `eb94a34` (feat)

## Files Created/Modified

- `integrations/nvtx/analyzer/Cargo.toml` — path deps on `nvtx-events`, `nvtx-bridge`, `quent-events`, `quent-time` + `thiserror`/`tracing`; `uuid` dev-dep. No forbidden framework deps.
- `integrations/nvtx/analyzer/src/lib.rs` — crate doc stating the framework-free/tolerant contract; private mods + `pub use` surface; re-exports `NvtxColor`/`NvtxPayload`.
- `integrations/nvtx/analyzer/src/error.rs` — small `NvtxModelError` (`Decode`, transparent `Other` + `other()` helper) and `NvtxModelResult`.
- `integrations/nvtx/analyzer/src/span.rs` — `NvtxSpan` (domain, thread_id, name, category, color, payload, start, end, kind, parent, synthetic_end), `SpanKind`, `SpanId`, saturating `duration()`.
- `integrations/nvtx/analyzer/src/model.rs` — `NvtxModel` + `spans()`; `NvtxModelBuilder::build` two-pass replay.
- `integrations/nvtx/analyzer/src/ranges.rs` — `StartEndRanges` open-range map, `close()` with the `end.max(start)` clamp, name placeholders, `warn!` on orphan end / restart / synthetic close.
- `integrations/nvtx/analyzer/tests/fixtures.rs` — synthetic `Event<NvtxEventEntity>` builders at exact timestamps (one shared `Uuid`).
- `integrations/nvtx/analyzer/tests/reconstruction.rs` — the 5 reconstruction/tolerance tests.
- `Cargo.toml` — added `"integrations/nvtx/analyzer"` to `[workspace] members` only.

## Decisions Made

- **Match key is `range_id` alone.** NVTX assigns range ids from one process-global counter, so a start/end pair correlates across threads *and* domains. The domain field on `RangeEnd` is redundant; ignoring it is what makes the match correct rather than merely convenient. The span's `domain` comes from the start.
- **Deterministic span ordering.** Observed closes append in close order; synthetically closed ranges are drained from the `HashMap` and sorted by `(start, range_id)` before appending — a `HashMap` drain order is otherwise unspecified and would have made multi-leak streams non-reproducible.
- **`thread_id: None` on StartEnd spans.** Plan 02-01 added thread ids to Push/Pop only, and correctly so: process-wide ranges have no single owning thread. The field exists on `NvtxSpan` for the push/pop slice (02-04).
- **Placeholders are pure functions of the raw handle.** `<unregistered string 0x{H}>` / `<unnamed>` — no counters or timestamps, so output stays reproducible and plan 02-03 can swap in real resolution without changing test shape.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `model.rs` stub created in Task 1 rather than Task 2**
- **Found during:** Task 1
- **Issue:** Task 1's `<files>` list omits `model.rs`, but its `<action>` requires stubbed `NvtxModelBuilder`/`NvtxModel` signatures "so the crate compiles and the tests fail on assertions, not on missing symbols". Those symbols have no other home.
- **Fix:** Created `src/model.rs` with the stub in Task 1; Task 2 replaced the stub body with the real two-pass replay, as its `<files>` list intends.
- **Files modified:** `integrations/nvtx/analyzer/src/model.rs`
- **Commits:** `86f2ebd` (stub), `eb94a34` (implementation)

**2. [Rule 2 - Missing coverage] Two extra tests added beyond the three named**
- **Found during:** Task 1
- **Issue:** Task 2's `<behavior>` block and the plan's `must_haves.truths` require synthetic close-at-trace-end and orphan-end-skip behavior, but Task 1 names only three tests. Shipping those behaviors untested would leave a `must_haves` truth unverified.
- **Fix:** Added `unclosed_start_closed_synthetic` and `orphan_end_skipped` alongside the three named tests, written RED in Task 1 and turned GREEN in Task 2. The three named tests are unaffected.
- **Files modified:** `integrations/nvtx/analyzer/tests/reconstruction.rs`
- **Commits:** `86f2ebd` (RED), `eb94a34` (GREEN)

**3. [Rule 3 - Blocking] Doc/comment prose reworded to satisfy the acceptance greps**
- **Found during:** Tasks 1 and 2
- **Issue:** The plan's acceptance criteria assert `grep -c "quent-analyzer\|quent-model\|..." Cargo.toml == 0` and `grep -rc "quent_analyzer\|quent_model\|RtFsm" src == 0`. Explanatory comments *naming* the banned crates ("`quent-analyzer` / `quent-model` are NOT dependencies", "no `RtFsm`") tripped those greps despite there being no actual dependency or import.
- **Fix:** Reworded the comments to describe the exclusion without the literal tokens ("the shared analysis/modelling framework crates are excluded by design", "no shared runtime state machine"). Semantics unchanged; the greps now return 0 and genuinely mean "no dependency".
- **Files modified:** `integrations/nvtx/analyzer/Cargo.toml`, `src/lib.rs`, `src/span.rs`
- **Commits:** `86f2ebd`, `eb94a34`

### Environment deviation (pre-execution)

**Worktree spawned from the wrong base.** This executor's worktree (`worktree-agent-a7dd03d6b6b5297f3`) was created at `f31e60b`, an older commit on `main`'s history, rather than the specified base `24994a0` — so `.planning/` and `CLAUDE.md` were absent entirely. The agent branch was clean with zero unique commits (`main..worktree-agent-… ` empty), so it was aligned to the specified base with `git reset --hard 24994a0` (the sanctioned startup branch-check remedy on a per-agent branch). No commits were destroyed; no protected ref was touched.

---

**Total deviations:** 3 auto-fixed (2× Rule 3, 1× Rule 2) + 1 pre-execution environment correction.
**Impact on plan:** No scope change, no architectural change. Deviations 1 and 3 are mechanical; deviation 2 strengthens coverage of behavior the plan already required.

## Requirement Status

- **MOD-01** (range materializes as a model span): **COMPLETE** for Start/End ranges — `NvtxSpan { kind: StartEnd }` is the "range = single-state FSM" in shape, with no framework dependency. Push/Pop spans land in 02-04.
- **ANA-04** (RangeStart/RangeEnd match process-wide by id): **COMPLETE.**
- **ANA-05** (tolerant reconstruction): **COMPLETE** — out-of-order, duplicate-timestamp, unclosed-start, and orphan-end streams all reconstruct to completion with no panic.

## Threat Model Compliance

| Threat ID | Disposition | Implementation |
|-----------|-------------|----------------|
| T-02-01 | mitigate | Orphan `RangeEnd` → `warn!` + skip; unclosed `RangeStart` → closed at trace end, flagged. No `unwrap`/`expect`/`panic!` on stream-derived data anywhere in `src/`. |
| T-02-03 | mitigate | `end: end.max(self.start)` clamp at span construction, plus `saturating_sub` in `duration()` — underflow is unreachable by two independent guards. |
| T-02-02 | accept | Open-range map growth bounded by the finite captured session, as planned. Documented; unchanged. |
| T-02-LEG | mitigate | Enforced: `grep -rc "quent_analyzer\|quent_model\|RtFsm" src` = 0 across all 5 files; forbidden-dep grep on `Cargo.toml` = 0. |

`expect()` appears only in test code (asserting the build succeeds), never in `src/`.

## Known Stubs

Intentional and scoped by the plan — this is the walking skeleton:

| Stub | File | Resolution |
|------|------|------------|
| Pass 1 is ordering-only; handle-resolution tables absent | `src/model.rs` | Plan 02-03 (`tables.rs`) |
| `NvtxMessage::RegisteredHandle` renders as `<unregistered string 0x{H}>` rather than the real string | `src/ranges.rs` | Plan 02-03 |
| `RangePush`/`RangePop`, `Mark`, `ResourceCreate`/`Destroy` fall through the replay match arm | `src/model.rs` | Plans 02-03 / 02-04 |
| `SpanKind::PushPop` / `SpanKind::Resource` defined but unconstructed; `NvtxSpan::parent` always `None` | `src/span.rs` | Plan 02-04 |

None of these prevent this plan's goal (Start/End reconstruction, tolerantly) from being achieved.

## Verification Evidence

- `cargo test -p nvtx-analyzer` — ok, **5 passed** (`startend_match_by_handle`, `out_of_order_sorted`, `duplicate_timestamps_no_panic`, `unclosed_start_closed_synthetic`, `orphan_end_skipped`).
- RED confirmed before implementation: all 5 failed on assertions (`left: 0, right: 2` etc.), not on missing symbols.
- `cargo clippy -p nvtx-analyzer --all-targets --all-features -- -D warnings` — clean.
- `cargo fmt --all -- --check` — clean (exit 0).
- `cargo build` (default-members) — succeeds; `nvtx-analyzer` correctly not built.
- `grep -c "quent-analyzer\|quent-model\|quent-schema\|model-macros" integrations/nvtx/analyzer/Cargo.toml` → **0**.
- `grep -rc "quent_analyzer\|quent_model\|RtFsm" integrations/nvtx/analyzer/src` → **0** for all 5 files.
- `grep -c "integrations/nvtx/analyzer" Cargo.toml` → 1; absent from the `default-members` block (lines 80–120) → 0.
- `grep -c "TimeOrderedCollector" src/model.rs` → 3; `grep -c "warn!" src/ranges.rs` → 3.
- No file deletions in either commit (`git diff --diff-filter=D HEAD~2 HEAD` empty).

## TDD Gate Compliance

Gate sequence satisfied: `test(02-02)` RED commit `86f2ebd` precedes `feat(02-02)` GREEN commit `eb94a34`. No REFACTOR commit was needed.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- The reconstruction spine exists. Plan 02-03 adds pass-1 resolution tables (`tables.rs`) — the two-pass shape and the placeholder policy are already in place for it to slot into.
- Plan 02-04 adds per-`(thread_id, domain)` push/pop stacks; `SpanKind::PushPop`, `NvtxSpan::thread_id`, and `NvtxSpan::parent` are already defined and waiting.
- No blockers introduced.

## Self-Check: PASSED

- Files verified present: `integrations/nvtx/analyzer/{Cargo.toml,src/lib.rs,src/error.rs,src/span.rs,src/model.rs,src/ranges.rs,tests/fixtures.rs,tests/reconstruction.rs}` — all FOUND.
- Commits verified present: `86f2ebd` (FOUND), `eb94a34` (FOUND).

---
*Phase: 02-nvtx-model-tolerant-analyzer*
*Completed: 2026-07-28*
