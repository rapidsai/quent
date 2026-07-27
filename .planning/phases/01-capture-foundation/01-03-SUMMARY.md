---
phase: 01-capture-foundation
plan: 03
subsystem: nvtx-capture-bridge
tags: [nvtx, bridge, cdylib, ring, drop-count, ndjson, e2e, capture]
requires:
  - quent-nvtx-events crate (NvtxEvent, NvtxEventKind, local EntityEvent)
  - quent-nvtx-injection cdylib (install_hook, CORE2 push/pop callbacks, InitializeInjectionNvtx2)
provides:
  - quent-nvtx bridge crate (bounded ArrayQueue ring + drop-and-count + drain thread → EventSender)
  - NvtxEventEntity newtype implementing the real quent_events::EntityEvent (orphan-rule fix)
  - self-configuring capture cdylib (.init_array installs env-driven ndjson pipeline; .fini_array flushes)
  - deterministic NVTX test-app (nvtx_test_app) + subprocess capture e2e harness
  - first observable end-to-end NVTX capture: attach via NVTX_INJECTION64_PATH → timestamped ndjson
affects:
  - Cargo.toml (workspace members + default-members)
  - Cargo.lock (crossbeam-queue)
tech-stack:
  added:
    - crossbeam-queue 0.3 (bounded lock-free ArrayQueue for the hot-path ring, D-16)
  patterns:
    - bridge-local newtype implementing the real EntityEvent (orphan-rule adapter, D-03)
    - bounded ring + drop-and-count in front of the unbounded EventSender (CAP-05/D-07/D-16)
    - drain thread modelling spawn_forwarder's drain/shutdown-drain discipline (std thread, not tokio)
    - self-configuring capture cdylib via ELF .init_array/.fini_array (env-driven, load/exit lifecycle)
    - runtime + exporter built on the drain thread to avoid a dynamic-loader-lock deadlock
    - cc-compiled NVTX v3 client shim for a Quent-free deterministic emitter (feature-gated)
key-files:
  created:
    - integrations/nvtx/instrumentation/Cargo.toml
    - integrations/nvtx/instrumentation/src/lib.rs
    - integrations/nvtx/instrumentation/build.rs
    - integrations/nvtx/instrumentation/c/emit.c
    - integrations/nvtx/instrumentation/src/bin/nvtx_test_app.rs
    - integrations/nvtx/instrumentation/tests/capture_e2e.rs
  modified:
    - Cargo.toml
    - Cargo.lock
decisions:
  - "Option A (coordinator-confirmed): the cdylib self-configures the ndjson sink at load; the test-app does NOT call install and links nothing Quent — the only design under which NVTX_INJECTION64_PATH actually delivers events (RTLD_LOCAL isolates the app's and the cdylib's HOOK statics)"
  - "install takes the Observer (via a make_observer closure), not an EventSender: EventSender has no public constructor outside quent-instrumentation, and per the coordinator no core crate is modified"
  - "make_observer runs on the drain thread so the tokio runtime + exporter are built AFTER .init_array returns (loader lock released), avoiding a constructor deadlock"
  - "e2e feature gates the NVTX client shim + test-app so default library/cdylib builds stay hermetic (no NVTX client headers required)"
metrics:
  duration: ~2.5h (incl. an architectural checkpoint)
  completed: 2026-07-14
  tasks: 2
  files: 6
---

# Phase 1 Plan 03: quent-nvtx Bridge + End-to-End Capture Summary

Shipped `quent-nvtx` — the bounded, drop-and-count bridge that fronts Quent's
unbounded `EventSender`, plus a **self-configuring capture cdylib** that makes an
uninstrumented app's NVTX push/pop ranges land as timestamped events in a real
ndjson file, proven by a GPU-less subprocess test. This is the first observable
end-to-end NVTX capture in Quent (CAP-01/CAP-04/CAP-05/VAL-01/VAL-02).

## What Was Built

- **`quent-nvtx` crate** (`integrations/nvtx/instrumentation/`,
  `crate-type = ["cdylib", "rlib"]`):
  - **`NvtxEventEntity`** — a `#[serde(transparent)]` newtype over
    `NvtxEventKind` implementing the *real* `quent_events::EntityEvent`
    (`NAME = "NvtxEvent"`). This resolves the orphan-rule gap plan 01 flagged:
    `NvtxEventKind` only implements the vocabulary crate's *local* `EntityEvent`
    (to keep `quent-nvtx-events` Quent-agnostic, D-03), and the orphan rule
    forbids implementing the real trait for it here. Being transparent, its
    ndjson bytes are identical to a bare `NvtxEvent`.
  - **`install(session, make_observer) -> Capture`** — builds a bounded
    `crossbeam_queue::ArrayQueue<Event<NvtxEventEntity>>` of `RING_CAPACITY`
    (65536), spawns a drain thread, and installs the sink-agnostic injection
    hook. The hook stamps each event with `Event::new_now` (CAP-04) and
    `push_or_drop`s it onto the ring — non-blocking, incrementing a global
    `DROPPED` counter on overflow (CAP-05/D-07/D-16). The drain thread mirrors
    `spawn_forwarder`'s drain/shutdown-drain discipline and forwards each event
    through `observer.send` (→ `EventSender::send`). Dropping the returned
    `Capture` stops + joins the drain thread, which drops the `Observer`
    (flushing the exporter).
  - **Self-configuring cdylib** (`mod cdylib`, Linux-only): an ELF `.init_array`
    constructor reads `QUENT_NVTX_OUTPUT_DIR` (+ optional `QUENT_NVTX_SESSION`)
    and calls `install`; a `.fini_array` destructor drops the pipeline at exit
    to flush. The runtime + ndjson exporter are built **on the drain thread**
    (after the constructor returns and the loader lock is released), not in the
    constructor.
  - Two `#[cfg(test)]` unit tests: `ring_drops_and_counts_when_full` (CAP-05:
    full ring drops-and-counts, producer never blocks) and
    `drain_forwards_events_in_order_to_ndjson` (FIFO forwarding through a real
    `Context` + ndjson tempdir, no core-crate changes).
- **Deterministic test-app** (`src/bin/nvtx_test_app.rs` + `c/emit.c`): a thin
  Rust `main` calling a cc-compiled NVTX v3 client shim that emits a fixed
  domain push/pop script (5 `RangePush` + 5 `RangePop` via CORE2
  `DomainRangePushEx`/`DomainRangePop`). It links **nothing** from Quent (only
  `InitializeInjectionNvtx2_fnptr`, the NVTX client's own null weak symbol,
  appears — verified with `nm`).
- **`build.rs`** — compiles `c/emit.c` under the `e2e` feature, locating NVTX
  headers via `NVTX_INCLUDE_DIR` or the pinned `nvidia-nvtx` git-dep checkout
  (`cargo_metadata` with `AllFeatures`, D-13). Default builds compile no C.
- **`tests/capture_e2e.rs`** — spawns `nvtx_test_app` as a subprocess with
  `NVTX_INJECTION64_PATH` = the `quent-nvtx` cdylib and `QUENT_NVTX_OUTPUT_DIR` =
  a tempdir, reads back `<dir>/<session>/NvtxEvent/*.ndjson` with the ndjson
  importer, and asserts ≥1 timestamped `RangePush` and ≥1 `RangePop`.
- **Workspace registration**: `integrations/nvtx/instrumentation` added to
  `members` + `default-members`.

## Verification

- `pixi run cargo test -p quent-nvtx` — 2 unit tests pass (ring drop-count, drain
  forwarding).
- `pixi run cargo test -p quent-nvtx --features e2e --test capture_e2e` — passes
  with **no GPU**: real subprocess attach → timestamped push/pop in ndjson.
- `pixi run cargo test -p quent-nvtx --features e2e` — full suite (unit + e2e) green.
- `pixi run cargo clippy -p quent-nvtx --all-targets --features e2e --locked -- -D warnings` — clean.
- `pixi run cargo clippy --workspace --all-targets --locked -- -D warnings` — clean
  (default features; see deviation on `--all-features`).
- `pixi run cargo fmt -p quent-nvtx -- --check` — clean.
- `pixi run cargo build -p quent-nvtx --offline` (default features) — succeeds
  hermetically (no NVTX client headers, no C compile).
- Symbol checks: cdylib exports `InitializeInjectionNvtx2` (1) and carries
  `.init_array`/`.fini_array`; the test-app binary contains **no** Quent
  `InitializeInjectionNvtx2`/`quent_nvtx::` symbols (only the NVTX client's null
  weak `InitializeInjectionNvtx2_fnptr`).
- Acceptance greps: `NVTX_INJECTION64_PATH` + `Command` in the harness;
  `ArrayQueue`, `fetch_add`, `new_now`, `.send(` in `lib.rs`; test-app imports no
  injection.

## Deviations from Plan

### MAJOR — [Rule 4, coordinator-confirmed Option A] Self-configuring cdylib, not test-app `install`

- **Found during:** Pre-implementation analysis (raised as a checkpoint;
  coordinator confirmed **Option A**).
- **Root cause (NVTX resolution order + RTLD_LOCAL):** The plan's Task 2 had the
  *test-app* call `quent_nvtx::install(sender, session)` while the harness set
  `NVTX_INJECTION64_PATH` to the injection cdylib. Verified against NVTX v3's own
  source (`c/include/nvtx3/nvtxDetail/nvtxInit.h`): when the env var is set, NVTX
  `dlopen(path, RTLD_LAZY)`s that library (`RTLD_LOCAL` — no global symbol merge)
  and calls **its** `InitializeInjectionNvtx2`; the callbacks that fire read
  **that library's** `HOOK` `OnceLock`. A hook installed in a copy of the
  injection code linked into the *application* is a different, unreachable
  static. So the plan's literal design would produce an **empty** ndjson file.
- **Fix (Option A):** The `quent-nvtx` cdylib bundles injection + bridge +
  ndjson exporter in one image. An ELF `.init_array` constructor installs the
  env-configured ndjson sink at `dlopen` (before NVTX's `InitializeInjectionNvtx2`),
  so the hook is set in the **same** module whose callbacks NVTX invokes; a
  `.fini_array` destructor flushes at exit. The test-app is a pure NVTX emitter
  linking nothing Quent — matching the real GPU-library scenario ("no app code
  changes", CAP-01).
- **Impact:** New machinery not in the plan prose — ELF `.init_array`/`.fini_array`,
  a tokio runtime built at load, an atexit-style flush, a C NVTX-emitter shim,
  and a `QUENT_NVTX_OUTPUT_DIR`/`QUENT_NVTX_SESSION` env protocol. Task-1 core
  (`install`, ring, drain, drop-count) is unchanged in intent. All Task-2
  acceptance greps still pass.
- **Files:** `src/lib.rs` (mod cdylib), `build.rs`, `c/emit.c`,
  `src/bin/nvtx_test_app.rs`, `tests/capture_e2e.rs`. **Commits:** 86e30c0, f0d6552.

### [Rule 3 — Blocking] `install` takes an `Observer`, not an `EventSender`

- **Issue:** `EventSender<T>` has no public constructor except `noop()` (drops
  everything), and `spawn_forwarder` is `pub(crate)`; a usable `EventSender`
  cannot be built outside `quent-instrumentation`. Per the coordinator, no core
  crate is modified (no `Observer::sender()` added).
- **Fix:** `install(session, make_observer)` where `make_observer` yields an
  `Observer<NvtxEventEntity>`; the drain thread forwards through `observer.send`,
  which internally calls the required `EventSender::send`. `make_observer` runs
  on the drain thread to keep the runtime/exporter construction off `.init_array`
  (loader-lock safety). The Task-1 forwarding unit test uses a real `Context` +
  ndjson tempdir, exactly per the coordinator's sub-decision.
- **Files:** `src/lib.rs`. **Commit:** 86e30c0.

### [Rule 3 — Blocking] `e2e` feature gates the test-app; e2e command needs `--features e2e`

- **Issue:** The NVTX client shim needs NVTX client headers at build time.
  Compiling it unconditionally would break hermetic default/CI builds of the
  whole workspace.
- **Fix:** The shim + `[[bin]] nvtx_test_app` are gated behind a non-default
  `e2e` feature. The plan's acceptance command therefore runs as
  `pixi run cargo test -p quent-nvtx --features e2e --test capture_e2e` (feature
  flag added). Default `cargo build -p quent-nvtx` stays hermetic and offline.
- **Files:** `Cargo.toml`, `build.rs`. **Commit:** f0d6552.

### [Rule 3 — Blocking] `cargo metadata --all-features` to locate the NVTX headers

- **Issue:** `nvidia-nvtx` is an *unactivated optional* build-dep of the
  injection crate, so a default `cargo metadata` does not list it (the injection
  build.rs was only ever verified via the `NVTX_INCLUDE_DIR` override).
- **Fix:** `build.rs` runs `MetadataCommand` with `CargoOpt::AllFeatures` so the
  pinned checkout appears (resolve only — nothing is built). `NVTX_INCLUDE_DIR`
  remains the primary override for hermetic CI.
- **Files:** `build.rs`. **Commit:** f0d6552.

### Worktree base correction

- The worktree spawned with HEAD at `eb51b76` (missing waves 1–2). After the
  mandatory HEAD/namespace assertion passed, I fast-forwarded to the expected
  base `099c295` (which carries waves 1–2) — `git reset --hard` was denied by the
  sandbox, so `git merge --ff-only` achieved the same (pure fast-forward).

### `--all-features` workspace clippy not run

- The phase-gate `cargo clippy --workspace --all-targets --all-features` enables
  the injection crate's `regenerate-bindings`, which runs bindgen and **rewrites
  the committed `bindings.rs`** (a plan-02 artifact I must not touch in this
  worktree). I instead ran the workspace clippy with default features (clean) and
  the `quent-nvtx` crate clippy under `--features e2e` (clean). Recommend running
  the full `--all-features` gate at the phase level with `NVTX_INCLUDE_DIR` pinned
  so the regen is a verified no-op diff.

## Threat Mitigations (from plan `<threat_model>`)

| Threat ID | Disposition | Status |
|-----------|-------------|--------|
| T-03-01 (unbounded allocation / OOM) | mitigate | Bounded `ArrayQueue` + drop-and-count in front of the unbounded `EventSender` (CAP-05); producer never blocks. |
| T-03-02 (producer blocking on the sink) | mitigate | Drain runs off the app thread; forwarding is `EventSender::send` (non-blocking unbounded), never the collector's blocking mpsc. |
| T-03-03 (ndjson deserialization in the harness) | accept | Test-only file produced by the same run; not untrusted input. |
| T-03-SC (crossbeam-queue supply chain) | mitigate | `crossbeam-queue` returned `[OK]` in RESEARCH; already present in the cargo cache; no blocking checkpoint needed. |

## Scope Notes (not stubs)

Only push/pop are exercised end-to-end (the plan's explicit thin-slice objective;
CORE2 is what plan 02 wired). Full event-kind coverage and multi-threaded
emission are Phase-1 plan 04. Timestamps come from `Event::new_now` in the bridge
(CAP-04), so the test-app needs no clock override and stays in `default-members`.

## Known Stubs

None. The `install` API, ring/drain, and cdylib self-config are all fully wired
and exercised by the passing unit + e2e tests.

## Commits

- 86e30c0 — feat(01-03): quent-nvtx bridge — bounded ring + drain + self-configuring capture cdylib
- f0d6552 — test(01-03): deterministic NVTX test-app + subprocess capture e2e harness

## Self-Check: PASSED

- FOUND: integrations/nvtx/instrumentation/Cargo.toml
- FOUND: integrations/nvtx/instrumentation/src/lib.rs
- FOUND: integrations/nvtx/instrumentation/build.rs
- FOUND: integrations/nvtx/instrumentation/c/emit.c
- FOUND: integrations/nvtx/instrumentation/src/bin/nvtx_test_app.rs
- FOUND: integrations/nvtx/instrumentation/tests/capture_e2e.rs
- FOUND: commit 86e30c0
- FOUND: commit f0d6552
