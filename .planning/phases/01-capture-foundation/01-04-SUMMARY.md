---
phase: 01-capture-foundation
plan: 04
subsystem: nvtx-capture
tags: [nvtx, ffi, capture, callbacks, multi-thread, e2e, coverage]
requires:
  - quent-nvtx-events crate (all NvtxEvent variants + attributes/payload)
  - quent-nvtx-injection cdylib (push/pop slice, install_hook, CORE2 table fill)
  - quent-nvtx bridge + self-configuring capture cdylib + subprocess e2e harness
provides:
  - full CORE/CORE2 capture surface — RangeStart/End, Mark, Domain create/destroy,
    RegisterString, NameCategory, NameThread (CORE), Resource create/destroy
  - synthesized-handle machinery (atomic counter) so injection subscribers return
    valid domain/string/resource handles + range ids captured verbatim
  - hand-written ABI supplement (range id, resource attrs/handle) alongside the
    committed bindings.rs
  - deterministic MULTI-THREADED test-app exercising every core kind + cross-thread
    RangeStart/End pairing + per-thread naming, with Phase-2 nasty fixtures
  - full-coverage subprocess e2e harness asserting every kind, the CORE payload
    union, cross-thread pairing, and per-thread NameThread
  - integrations/nvtx/README.md (D-14 regen workflow doc)
affects:
  - integrations/nvtx/injection/src/{callbacks,convert,init}.rs
  - integrations/nvtx/instrumentation/{c/emit.c,src/bin/nvtx_test_app.rs,tests/capture_e2e.rs}
tech-stack:
  added: []
  patterns:
    - injection subscribers ARE the implementation: synthesize + RETURN handles
      (domain/string/resource/range-id) from an atomic counter, captured verbatim
    - hand-written #[repr(C)] ABI supplement for surface the committed-bindings
      allowlist omits (avoids libclang + keeps bindings.rs a byte-identical no-op)
    - Rust-orchestrated multi-threading over granular C NVTX-client primitives
    - subscribe! macro: transmute each typed subscriber to the ABI fn-pointer slot
key-files:
  created:
    - integrations/nvtx/README.md
  modified:
    - integrations/nvtx/injection/src/callbacks.rs
    - integrations/nvtx/injection/src/convert.rs
    - integrations/nvtx/injection/src/init.rs
    - integrations/nvtx/instrumentation/c/emit.c
    - integrations/nvtx/instrumentation/src/bin/nvtx_test_app.rs
    - integrations/nvtx/instrumentation/tests/capture_e2e.rs
decisions:
  - Injection subscribers for handle-returning calls (DomainCreateA, RangeStartEx,
    RegisterStringA, ResourceCreate) synthesize a nonzero handle/id from a
    process-global atomic counter and RETURN it, because in injection mode the
    subscriber's return value IS the token the application uses; the same bits are
    captured verbatim so later references/pairing correlate (Rule 2 correctness)
  - Extra ABI types (nvtxRangeId_t, nvtxResourceAttributes_v0, nvtxResourceHandle_t)
    declared by hand in convert.rs `mod abi` instead of regenerating bindings.rs —
    the committed allowlist omits them and regen would need libclang + rewrite the
    plan-02 hermetic artifact (files_modified excludes bindings.rs/build.rs)
  - Multi-threading lives in the Rust test-app (thread::spawn over granular C
    primitives), not in C; the domain handle crosses threads via a Send wrapper
requirements: [CAP-02, CAP-03, VAL-01]
metrics:
  duration: ~13m
  completed: 2026-07-14
  tasks: 3
  files: 6
---

# Phase 1 Plan 04: Full Core NVTX Capture Coverage Summary

Widened the proven push/pop slice to the FULL core NVTX surface: every remaining
CORE/CORE2 kind now converts to a verbatim `NvtxEvent` behind a panic-safe
callback, a deterministic multi-threaded test-app emits every kind (with a CORE
payload union and a cross-thread `RangeStart`/`RangeEnd` pair), and a GPU-less
subprocess harness proves all of it lands in ndjson — completing the Phase-1
capture surface (CAP-02/CAP-03/VAL-01, success-criterion 1).

## What Was Built

- **All remaining callbacks + conversions** (`callbacks.rs`, `convert.rs`,
  `init.rs`): 12 `extern "C"` subscribers (14 `catch_unwind` boundaries) for
  `DomainMarkEx`, `DomainRangeStartEx`/`End`, `DomainRangePushEx`/`Pop`,
  `DomainResourceCreate`/`Destroy`, `DomainNameCategoryA`,
  `DomainRegisterStringA`, `DomainCreateA`/`Destroy` (CORE2) and `NameOsThreadA`
  (CORE). Each wraps its body in `catch_unwind` (T-04-01), copies caller strings
  in (Pitfall 3), reads attribute members bounded by `size` (Pitfall 4), and
  keeps raw `u64` handles verbatim. The CORE payload union is preserved verbatim
  on marks/ranges (D-12). Registered strings are captured once at registration;
  every later reference carries only the raw handle.
- **Synthesized-handle machinery** (`init::next_handle`, atomic counter from 1):
  handle-returning subscribers generate and return a nonzero token that the app
  then uses — the mechanism by which cross-thread `RangeEnd` and handle
  references correlate. `init.rs` now fills BOTH the CORE and CORE2 module tables
  (CORE best-effort) via a `subscribe!` transmute macro; distinct CBIDs: 12.
- **Hand-written ABI supplement** (`convert.rs` `mod abi`): `nvtxRangeId_t`,
  `nvtxResourceAttributes_v0` (+ identifier union), and `nvtxResourceHandle_t` —
  the surface the committed `bindings.rs` allowlist omits — declared `#[repr(C)]`
  so `offset_of!` reads match the NVTX layout. `bindings.rs` is untouched (empty
  diff), so default/CI builds still need no libclang.
- **Multi-threaded test-app** (`c/emit.c` + `nvtx_test_app.rs`): `emit.c` is now
  granular NVTX v3 primitives; the Rust `main` orchestrates every core kind on
  the main thread plus two worker threads that pair a cross-thread
  `RangeStart`/`RangeEnd` on one id and each name themselves. It leaves an
  unclosed range and a second domain as ready-made Phase-2 fixtures. Links
  nothing from Quent.
- **Full-coverage e2e harness** (`capture_e2e.rs`): asserts, from the captured
  ndjson — presence of every core kind (CAP-02), the verbatim CORE payload union
  on the mark (CAP-03/D-12), a `RangeStart`/`RangeEnd` pair sharing one range id
  (D-11), and a distinct `NameThread` (by id and by name) for main + each worker.
- **`integrations/nvtx/README.md`** (D-14): crate layout, the committed-bindings /
  no-libclang-in-CI guarantee, the exact `--features regenerate-bindings` command
  and its rev-bump trigger (D-13), the `NVTX_INJECTION64_PATH` primary +
  `static-injection` secondary attach paths, Linux-64-only, and the GPU-less e2e
  run command.

## Verification

- `pixi run cargo test -p quent-nvtx-injection` — 10 passed (pure-convert tests
  for every remaining kind, incl. size-bounded resource reads).
- `pixi run cargo build -p quent-nvtx --features e2e` (build cdylib FIRST) then
  `pixi run cargo test -p quent-nvtx --features e2e --test capture_e2e` — passes
  GPU-less; asserts every core kind + payload union + cross-thread pairing +
  per-thread NameThread.
- `pixi run cargo test -p quent-nvtx --features e2e` — full suite green.
- `pixi run cargo clippy -p quent-nvtx-injection --all-targets --locked -- -D warnings`
  and `... -p quent-nvtx --all-targets --features e2e ...` — clean; every
  `unsafe` block carries `// SAFETY:`.
- `pixi run cargo clippy --workspace --all-targets --locked -- -D warnings`
  (default features) — clean (only a pre-existing, unrelated cxx `operator`
  keyword warning in `quent-qe-bridge`, out of scope).
- `pixi run cargo fmt -p quent-nvtx-injection -- --check` and `-p quent-nvtx` —
  clean.
- `uvx rumdl@0.1.67 check integrations/nvtx/README.md` — no issues.
- Grep checks: 14 `catch_unwind` in `callbacks.rs`; no `PayloadSchema/EnumRegister`
  wiring; `bindings.rs` diff vs base is empty (untouched).

## Build-ordering note (carried from Wave 3)

The e2e test hard-requires the capture cdylib built first:

```sh
pixi run cargo build -p quent-nvtx --features e2e
pixi run cargo test  -p quent-nvtx --features e2e --test capture_e2e
```

This requirement is preserved and now documented in both the test's `//!` doc and
`integrations/nvtx/README.md`.

## Deviations from Plan

### [Rule 2 - Missing critical functionality] Synthesized + returned handles

- **Found during:** Task 1. In NVTX injection mode a subscriber *is* the
  implementation, so the value it returns for `DomainCreateA`, `RangeStartEx`,
  `RegisterStringA`, and `ResourceCreate` becomes the token the application uses.
  The plan prose only described capturing; returning a valid, unique handle is
  required for correctness (and for cross-thread `RangeEnd` to correlate).
- **Fix:** `init::next_handle()` (atomic counter from 1) mints a nonzero handle;
  the callback captures it verbatim in the event and returns it to the app.
- **Files:** `callbacks.rs`, `init.rs`. **Commit:** 82c04d0.

### [Rule 3 - Blocking] Hand-written ABI supplement instead of regenerating bindings

- **Issue:** RangeStart/End and Resource create/destroy need `nvtxRangeId_t`,
  `nvtxResourceAttributes_t`, and `nvtxResourceHandle_t`, which the committed
  `bindings.rs` allowlist omits. Regenerating would require libclang and rewrite
  the plan-02 hermetic artifact (explicitly out of `files_modified`, and flagged
  by the Wave-3 warning).
- **Fix:** declared the small extra surface by hand in `convert.rs` `mod abi`
  (`#[repr(C)]`, documented, kept in sync note added to the README). `bindings.rs`
  stays a byte-identical no-op.
- **Files:** `convert.rs`. **Commit:** 82c04d0.

### [Rule 3 - Blocking] Rust 2021 disjoint closure capture of the domain pointer

- **Issue:** the worker `thread::spawn` closures accessed `domain.0` (a
  `*mut c_void`), so 2021-edition disjoint capture tried to move the non-`Send`
  raw pointer field, failing the `Send` bound.
- **Fix:** added `let domain = domain;` at the top of each closure to force
  whole-`Domain` (which is `Send`) capture.
- **Files:** `nvtx_test_app.rs`. **Commit:** d4eaea6.

### TDD collapse (Task 1, tdd="true")

Same rationale as plan 02: the crate is in `default-members`, and `callbacks.rs`/
`init.rs` reference the new `convert` fns (a compile-failing RED commit would
register a broken crate, and unreferenced `convert` fns would fail
`clippy -D warnings` on `dead_code`). Impl + the per-kind `convert` unit tests
landed together in the `feat` commit (82c04d0); the multi-threaded harness is the
Task-2 `test(...)` commit (d4eaea6). No separate failing-RED commit. Plan type is
`execute` (no plan-level TDD gate); MVP/TDD runtime gate was not signaled.

### `--all-features` workspace test/clippy not run (carried from Wave 3)

`--all-features` activates the injection crate's `regenerate-bindings`, which runs
bindgen and rewrites the committed `bindings.rs` (a plan-02 artifact this worktree
must not touch). Ran the workspace clippy at default features (clean) and the
nvtx crates' tests/clippy under `--features e2e` (clean) instead. Recommend the
phase-level `--all-features` gate run with `NVTX_INCLUDE_DIR` pinned so the regen
is a verified no-op diff.

### Worktree base correction

Worktree spawned with HEAD at `eb51b76` (missing waves 1–3). After the mandatory
HEAD/namespace assertion passed, fast-forwarded to the expected base `24ae339`
via `git merge --ff-only` (a pure fast-forward) because `git reset --hard` was
denied by the sandbox.

## Threat Mitigations (from plan `<threat_model>`)

| Threat ID | Disposition | Status |
|-----------|-------------|--------|
| T-04-01 (panic across C ABI, all new callbacks) | mitigate | `catch_unwind` at every new callback boundary (14 total). |
| T-04-02 (caller-owned `const char*` UAF) | mitigate | strings copied in-callback; registered strings captured once, handle-only thereafter. |
| T-04-03 (read past `size` on attribute-bearing kinds) | mitigate | event- and resource-attribute reads guarded by `size` (Pitfall 4). |
| T-04-SC (npm/pip/cargo installs) | mitigate | no new packages introduced; no install checkpoint needed. |

## Known Stubs

None. Payload-EXTENSION callbacks remain deliberately unwired (D-12, deferred to
the PAY-01 decode phase) — a documented deferral, not a stub. The convert layer
already handles the full `NvtxEventAttributes` so future kinds are additive.

## Commits

- 82c04d0 — feat(01-04): wire all remaining CORE/CORE2 NVTX callbacks + conversions
- d4eaea6 — test(01-04): full multi-threaded NVTX coverage + end-to-end assertions
- 342c39a — docs(01-04): document the NVTX bindings regeneration workflow (D-14)

## Self-Check: PASSED

- FOUND: integrations/nvtx/README.md
- FOUND: integrations/nvtx/injection/src/callbacks.rs (12 new subscribers)
- FOUND: integrations/nvtx/injection/src/convert.rs (mod abi + per-kind converts)
- FOUND: integrations/nvtx/injection/src/init.rs (next_handle + CORE/CORE2 fill)
- FOUND: integrations/nvtx/instrumentation/c/emit.c (granular primitives)
- FOUND: integrations/nvtx/instrumentation/src/bin/nvtx_test_app.rs (multi-thread)
- FOUND: integrations/nvtx/instrumentation/tests/capture_e2e.rs (full assertions)
- FOUND: commit 82c04d0
- FOUND: commit d4eaea6
- FOUND: commit 342c39a
