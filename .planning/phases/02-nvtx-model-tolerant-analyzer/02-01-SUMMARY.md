---
phase: 02-nvtx-model-tolerant-analyzer
plan: 01
subsystem: capture
tags: [nvtx, thread-id, gettid, libc, injection, ffi, rust]

# Dependency graph
requires:
  - phase: 01-capture-foundation
    provides: nvtx-events vocabulary, nvtx-injection callbacks/convert, nvtx-example run_capture
provides:
  - "thread_id: u32 on NvtxEvent::RangePush and ::RangePop (verbatim capture vocabulary)"
  - "init::current_thread_id() OS-thread-id helper (Linux SYS_gettid, std ThreadId fallback)"
  - "All 5 Push/Pop injection callbacks stamp the OS thread id at capture time"
  - "End-to-end proof (nvtx-example) that Push/Pop carry a nonzero, matched thread id"
affects: [02-02, 02-04, ANA-03, per-thread-stack-reconstruction]

# Tech tracking
tech-stack:
  added: ["libc 0.2 (workspace dep; SYS_gettid raw syscall)"]
  patterns:
    - "OS thread id read on the app thread inside the callback catch_unwind guard, then passed into the pure convert:: functions"
    - "Separate integration-test file per run_capture call (one-shot install_hook is process-global)"

key-files:
  created:
    - "integrations/nvtx/example/tests/thread_id.rs"
  modified:
    - "integrations/nvtx/events/src/lib.rs"
    - "integrations/nvtx/injection/src/init.rs"
    - "integrations/nvtx/injection/src/convert.rs"
    - "integrations/nvtx/injection/src/callbacks.rs"
    - "integrations/nvtx/injection/Cargo.toml"
    - "Cargo.toml"

key-decisions:
  - "Scope thread_id to RangePush/RangePop only (per D-17); Mark/RangeStart deliberately not extended"
  - "Read the OS thread id via the raw SYS_gettid syscall, not the glibc gettid() wrapper (wrapper symbol needs glibc >= 2.30, absent on the conda CI sysroot)"
  - "Place the end-to-end test in its own file (tests/thread_id.rs) because install_hook is one-shot per process"

patterns-established:
  - "convert:: functions stay side-effect-free; the thread id is read in the callback and passed in as a parameter"

requirements-completed: []  # ANA-03 is UNBLOCKED by this plan but NOT satisfied here — its reconstruction lands in plan 02-04.

# Metrics
duration: ~20 min
completed: 2026-07-28
---

# Phase 2 Plan 01: Capture-Side Thread Identity Summary

**RangePush/RangePop now carry a real OS thread id (Linux gettid) stamped at capture time through all 5 injection callbacks, proven end-to-end by nvtx-example.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-07-28T05:13Z
- **Completed:** 2026-07-28T05:33Z
- **Tasks:** 3 (1 pre-approved gate + 2 implementation)
- **Files modified:** 7 (6 modified + 1 created; plus Cargo.lock)

## Accomplishments
- Added `thread_id: u32` to `NvtxEvent::RangePush` and `NvtxEvent::RangePop` in the verbatim capture vocabulary (same OS id space `NameThread` uses), enum `derive` block unchanged.
- Added `init::current_thread_id()` returning the Linux OS thread id via the raw `SYS_gettid` syscall, with a documented std `ThreadId`-derived non-Linux fallback (capture is Linux-primary).
- Wired all 5 Push/Pop callbacks (CORE2 domain surface: `on_domain_range_push_ex`, `on_domain_range_pop`; CORE default-domain surface: `on_range_push_ex`, `on_range_push_a`, `on_range_pop`) to read the thread id on the app thread inside the existing `catch_unwind` guard and pass it into the pure `convert::` functions. Wide-char stubs left untouched (they emit no event).
- Proved it end-to-end: the new `pushpop_carry_thread_id` test asserts every captured Push/Pop has a nonzero `thread_id` and that Push/Pop from the single example thread share one id.

## Task Commits

Each task was committed atomically (DCO sign-off, Conventional Commits):

1. **Task 1: Package legitimacy gate — libc** — no commit (verification gate). Pre-approved by the orchestrator/developer: `libc` at crates.io is the official `rust-lang/libc` (0.2.x, ~1.4B downloads, Rust-team maintained). Recorded APPROVED 2026-07-28; proceeded directly.
2. **Task 2: Add thread_id to the vocabulary + the OS-thread-id helper** — `8c02643` (feat)
3. **Task 3: Stamp thread_id in every Push/Pop callback + prove it end-to-end** — `dd86161` (feat)

**Plan metadata:** committed with this SUMMARY, STATE.md, ROADMAP.md.

## Files Created/Modified
- `integrations/nvtx/events/src/lib.rs` — added `thread_id: u32` field (with doc line) to RangePush and RangePop.
- `integrations/nvtx/injection/src/init.rs` — added `current_thread_id()` (Linux `SYS_gettid` + non-Linux fallback) and its `current_thread_id_is_stable_and_nonzero` unit test.
- `integrations/nvtx/injection/src/convert.rs` — threaded a `thread_id: u32` parameter through `range_pop` / `range_push` / `range_push_a`; added/updated per-variant unit tests to assert the id.
- `integrations/nvtx/injection/src/callbacks.rs` — 5 Push/Pop callbacks read `init::current_thread_id()` inside the guard and pass it into `convert`.
- `integrations/nvtx/injection/Cargo.toml` — added `libc = { workspace = true }`.
- `Cargo.toml` — added `libc = "0.2"` to `[workspace.dependencies]` (alphabetical).
- `integrations/nvtx/example/tests/thread_id.rs` — new end-to-end capture test `pushpop_carry_thread_id`.

## Decisions Made
- **Thread id via `SYS_gettid` raw syscall, not the `libc::gettid()` wrapper.** The wrapper symbol is only exported by glibc >= 2.30; the conda CI sysroot is older and failed to link (`undefined symbol: gettid`). The raw syscall works on every Linux libc and still keys off `gettid` (`SYS_gettid`), satisfying the plan's `gettid` link requirement.
- **End-to-end test in its own file.** `nvtx_injection::install_hook` is one-shot per process; a second `run_capture` in `capture.rs`'s binary would fail with `AlreadyInstalled`. Each `tests/*.rs` is a separate test binary/process, so `tests/thread_id.rs` installs the hook independently — and `cargo test -p nvtx-example` (both binaries) stays green.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Compile-unblock shim so Task 2 could be verified independently**
- **Found during:** Task 2
- **Issue:** Adding the compiler-enforced `thread_id` field to the enum breaks every `RangePush`/`RangePop` construction site in `convert.rs`, so `nvtx-injection` (and thus Task 2's `cargo test -p nvtx-injection current_thread_id...` gate) could not compile with only the plan's Task 2 files changed.
- **Fix:** Constructed the three `convert` functions with `thread_id: 0` (and added `..` to two exact-match test patterns) in the Task 2 commit; Task 3 then replaced the shim with a real `thread_id` parameter threaded from the callbacks.
- **Files modified:** integrations/nvtx/injection/src/convert.rs
- **Verification:** `cargo test -p nvtx-injection` green after Task 2 (22 tests) and Task 3 (23 tests).
- **Committed in:** 8c02643 (shim), dd86161 (real threading)

**2. [Rule 3 - Blocking] `SYS_gettid` raw syscall instead of `libc::gettid()`**
- **Found during:** Task 2
- **Issue:** `libc::gettid()` failed to link on the conda CI sysroot (`rust-lld: error: undefined symbol: gettid`) — the glibc wrapper predates glibc 2.30.
- **Fix:** `unsafe { libc::syscall(libc::SYS_gettid) as u32 }`.
- **Files modified:** integrations/nvtx/injection/src/init.rs
- **Verification:** `current_thread_id_is_stable_and_nonzero` passes; clippy/fmt clean.
- **Committed in:** 8c02643

**3. [Rule 3 - Blocking] End-to-end test relocated to `tests/thread_id.rs`**
- **Found during:** Task 3
- **Issue:** The plan places `pushpop_carry_thread_id` in `tests/capture.rs`, but that binary already calls `run_capture`, and `install_hook` is one-shot per process — two `run_capture` calls in one binary collide, breaking `cargo test -p nvtx-example`.
- **Fix:** Added the test in a new, separate integration-test file `integrations/nvtx/example/tests/thread_id.rs` (its own process). Behavior and assertions match the plan exactly.
- **Files modified:** integrations/nvtx/example/tests/thread_id.rs (new); capture.rs left unchanged.
- **Verification:** `cargo test -p nvtx-example` runs both binaries green.
- **Committed in:** dd86161

---

**Total deviations:** 3 auto-fixed (all Rule 3 — blocking build/test issues).
**Impact on plan:** No scope change and no requirement impact — all three are mechanical unblocks that preserve the plan's intent (field on Push/Pop, `gettid` id space, end-to-end proof). No architectural change.

## Requirement Status

- **ANA-03** (Push/Pop reconstruct as per-thread nested stacks): **UNBLOCKED, not completed.** This plan delivers only the capture-side prerequisite (the `thread_id` field + stamping). The actual per-`(thread_id, domain)` stack reconstruction lands in plan 02-04. Left `Pending` in REQUIREMENTS.md — not marked complete.

## Issues Encountered
- glibc `gettid` wrapper link failure and the one-shot `install_hook` process constraint — both resolved as documented under Deviations.

## Verification Evidence

All gates run under `pixi run`:

- `cargo test -p nvtx-events` — ok (compiles; 0 unit tests).
- `cargo test -p nvtx-injection` — ok, 23 passed (incl. `current_thread_id_is_stable_and_nonzero`, `range_pop_carries_domain_and_thread_id_verbatim`, updated `range_push` asserts).
- `cargo test -p nvtx-example` — ok, both binaries: `captures_core_nvtx_kinds` + `pushpop_carry_thread_id` passed.
- `cargo test -p nvtx-example pushpop_carry_thread_id` — ok, 1 passed.
- `cargo clippy -p nvtx-injection --all-targets --all-features -- -D warnings` — clean. (also nvtx-events, nvtx-example — clean.)
- `cargo fmt --check` — clean.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The capture vocabulary now carries per-thread identity; Wave 2 (`02-02`) can build the members-only `nvtx-analyzer` crate, and Wave 4 (`02-04`) can build per-`(thread_id, domain)` nested stacks to satisfy ANA-03.
- No blockers introduced.

## Self-Check: PASSED
- Files verified present: events/src/lib.rs, injection/{init,convert,callbacks}.rs, injection/Cargo.toml, root Cargo.toml, example/tests/thread_id.rs — all FOUND.
- Commits verified present: 8c02643 (FOUND), dd86161 (FOUND).

---
*Phase: 02-nvtx-model-tolerant-analyzer*
*Completed: 2026-07-28*
