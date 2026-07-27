---
phase: 01-capture-foundation
plan: 02
subsystem: nvtx-injection
tags: [nvtx, ffi, cdylib, bindgen, injection, capture]
requires:
  - quent-nvtx-events crate (NvtxEvent, NvtxEventAttributes, NvtxMessage, NvtxPayload)
provides:
  - quent-nvtx-injection cdylib (InitializeInjectionNvtx2 entry, CORE2 table fill)
  - sink-agnostic install_hook(Fn(NvtxEvent) + Send + Sync + 'static) (D-03)
  - verbatim push/pop convert (const char* copy-in, size-bounded reads, CORE payload union)
  - committed bindgen output for the NVTX injection ABI (D-14)
  - feature-gated regenerate-bindings (git-dep + bindgen + cargo_metadata) and static-injection (cc shim)
affects:
  - Cargo.toml (workspace members + default-members + workspace.dependencies)
  - Cargo.lock (nvidia-nvtx / nvtx-sys git entries, optional)
  - deny.toml (allow-git NVIDIA/NVTX, D-13)
tech-stack:
  added:
    - bindgen 0.72 (build-dep, optional, regenerate-bindings only)
    - cargo_metadata 0.23 (build-dep, optional, regenerate-bindings only)
    - cc 1.2 (build-dep, optional, static-injection only)
    - NVIDIA/NVTX git-dep pinned rev 7d113f290f89 (v3.5.0), optional
  patterns:
    - committed bindgen output + feature-gated regeneration (hermetic CI, D-14)
    - catch_unwind at every C-ABI callback boundary (panic containment)
    - unaligned raw-pointer reads bounded by nvtxEventAttributes_t.size (Pitfall 4)
    - OnceLock one-shot for both table fill and hook install (no #[ctor])
key-files:
  created:
    - integrations/nvtx/injection/Cargo.toml
    - integrations/nvtx/injection/build.rs
    - integrations/nvtx/injection/wrapper.h
    - integrations/nvtx/injection/c/symbol.c
    - integrations/nvtx/injection/src/bindings.rs
    - integrations/nvtx/injection/src/lib.rs
    - integrations/nvtx/injection/src/init.rs
    - integrations/nvtx/injection/src/callbacks.rs
    - integrations/nvtx/injection/src/convert.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - deny.toml
decisions:
  - Push/pop wired via CORE2 (DomainRangePushEx/DomainRangePop) because NvtxEvent::RangePush carries a domain handle; CORE (non-domain) push/pop deferred to plan 04
  - convert produces RangePush { domain, attributes: NvtxEventAttributes } to match the plan-01 crate shape (not the flat fields sketched in the plan interface)
  - build.rs supports an NVTX_INCLUDE_DIR override alongside the cargo_metadata git-dep locate, so regeneration is testable offline without a network fetch
  - bindings allow uses the `nonstandard_style` lint group to keep the generated header rustfmt-stable
metrics:
  duration: ~1h
  completed: 2026-07-13
  tasks: 3
  files: 12
---

# Phase 1 Plan 02: quent-nvtx-injection cdylib Summary

Built `quent-nvtx-injection` — the Quent-agnostic cdylib NVTX loads via
`NVTX_INJECTION64_PATH`. It exports an unmangled `InitializeInjectionNvtx2`, fills
the CORE2 push/pop callback slots one-shot, converts those calls to verbatim
`NvtxEvent`s (owned strings, raw `u64` handles, CORE payload union), and hands
them to a sink-agnostic `Fn(NvtxEvent)` hook — all built from committed bindgen
output with no CI libclang requirement and no NVTX git fetch on default builds.

## What Was Built

- **`quent-nvtx-injection` crate** (`integrations/nvtx/injection/`,
  `crate-type = ["cdylib", "rlib"]`):
  - **`init.rs`** — `#[unsafe(no_mangle)] pub extern "C" fn
    InitializeInjectionNvtx2(NvtxGetExportTableFunc_t) -> c_int`. Under a
    `OnceLock` (all state built here, never in a `#[ctor]`) it retrieves
    `NVTX_ETID_CALLBACKS`, fetches the `NVTX_CB_MODULE_CORE2` function table, and
    installs `DomainRangePushEx`/`DomainRangePop` at their `NVTX_CBID_*` indices
    (bounds-checked against the ABI-reported table size). Exposes
    `install_hook(impl Fn(NvtxEvent) + Send + Sync + 'static)` stored in a
    `OnceLock` (D-03), plus a `thiserror` `InstallHookError`.
  - **`callbacks.rs`** — the two `extern "C"` subscribers. Each body is wrapped in
    `std::panic::catch_unwind`; a caught panic is discarded, never crossing the C
    ABI (T-02-01). The callback does only: null-check, convert, dispatch to the
    hook.
  - **`convert.rs`** — pure, side-effect-free `args -> NvtxEvent`. Immediate
    `const char*` messages are copied into an owned `String` inside the call
    (Pitfall 3); registered messages keep only the raw handle. Reads are bounded
    by `nvtxEventAttributes_t.size` via unaligned raw-pointer loads that never
    materialize a `&` to the full struct (Pitfall 4). The CORE payload union is
    captured verbatim into `NvtxPayload`/`NvtxPayloadValue` (D-12). Includes the
    three behavior tests (Task 3).
  - **`bindings.rs`** — committed bindgen 0.72.1 output for the injection ABI
    (D-14); contains `nvtxEventAttributes`, the export-table types, and the
    CORE/CORE2 CBID module-const enums.
  - **`build.rs`** — `rerun-if-changed=wrapper.h`; default builds do nothing else.
    `regenerate-bindings` runs bindgen against headers located either via
    `NVTX_INCLUDE_DIR` (offline) or the `nvidia-nvtx` git-dep checkout found with
    `cargo_metadata` (`<checkout>/c/include`, D-13). `static-injection` compiles
    `c/symbol.c` via `cc`.
  - **`c/symbol.c`** — strong-symbol override of NVTX's weak
    `InitializeInjectionNvtx2_fnptr` for the static/link-time path (D-15
    secondary), compiled only under `static-injection`.
  - **`lib.rs`** — `compile_error!` on non-Linux/32-bit (D-04); module wiring +
    re-exports.
- **Workspace registration**: crate added to `members` + `default-members` under
  the `# NVTX integration crates` block; `bindgen`/`cargo_metadata`/`cc` added to
  `[workspace.dependencies]`.
- **`deny.toml`**: `allow-git = ["https://github.com/NVIDIA/NVTX"]` (D-13), with a
  comment explaining the pinned-rev reproducibility trade-off.

## Verification

- `pixi run cargo build -p quent-nvtx-injection --locked --offline` — succeeds
  from committed `bindings.rs`; no libclang, no NVTX git fetch (default build
  compiles only injection + events, NOT nvidia-nvtx).
- `nm -D target/debug/libquent_nvtx_injection.so | grep -c InitializeInjectionNvtx2`
  → 1 (`T InitializeInjectionNvtx2`).
- `pixi run cargo test -p quent-nvtx-injection` — 3 passed, 0 failed.
- `pixi run cargo clippy -p quent-nvtx-injection --all-targets --locked -- -D warnings`
  — clean; every `unsafe` block carries a `// SAFETY:` comment.
- `pixi run cargo fmt -p quent-nvtx-injection -- --check` — clean.
- `cargo deny check` (cargo-deny 0.19.0) — advisories/bans/licenses/**sources** ok
  (NVIDIA/NVTX git source accepted via the allow-list).
- `pixi run cargo build -p quent-nvtx-injection --features static-injection` —
  compiles the `cc` shim.
- `pixi run cargo build -p quent-nvtx-injection --features regenerate-bindings`
  (with `NVTX_INCLUDE_DIR` at the pinned v3.5.0 clone) — regenerates
  `src/bindings.rs` as a **no-op diff** (byte-identical to the committed file);
  the `nvidia-nvtx`/`nvtx-sys` git-deps built cleanly.
- Grep checks: `catch_unwind` present per callback boundary; `OnceLock` in
  init.rs; `compile_error` in lib.rs; `nvidia-nvtx` git-dep is `optional = true`
  and pins `rev` (not `branch`); Cargo.toml depends on no `quent-*` crate except
  `quent-nvtx-events` (D-03).

## Deviations from Plan

### Design reconciliation

**1. [Rule 3 - Blocking] `RangePush` carries `attributes`, not flat fields**
- **Found during:** Task 3 (convert).
- **Issue:** The plan's `<interfaces>` sketched
  `NvtxEvent::RangePush { domain, category, message, payload, color }`, but the
  plan-01 vocabulary crate actually defines
  `RangePush { domain, attributes: NvtxEventAttributes }` (with category/color/
  message/payload nested in `NvtxEventAttributes`).
- **Fix:** `convert::range_push` builds an `NvtxEventAttributes` and returns
  `RangePush { domain, attributes }`. All safety discipline (copy-in, size guard,
  verbatim payload) is unchanged.
- **Files:** `integrations/nvtx/injection/src/convert.rs`. **Commit:** 9f8fb5b.

**2. [Rule 2 - Missing critical functionality] `NVTX_INCLUDE_DIR` regen override**
- **Issue:** D-13's canonical regen path (git-dep + `cargo_metadata`) requires a
  network fetch and building `nvidia-nvtx`, which makes offline verification of
  the regeneration path impossible and couples binding regen to a heavy build.
- **Fix:** `build.rs` honors an `NVTX_INCLUDE_DIR` env override before falling back
  to the `cargo_metadata` git-dep locate. The canonical D-13 path is preserved;
  the override adds hermetic, offline regeneration (used to verify the no-op
  diff).
- **Files:** `integrations/nvtx/injection/build.rs`. **Commit:** ce20562.

### Task ordering / TDD collapse

Task 3 is `tdd="true"`, but the CORE2 table fill in Task 2 cannot compile without
`convert`/`callbacks`. So the `convert`/`callbacks` **implementations** landed in
the Task 2 commit (9f8fb5b) and Task 3 (2f99cfb) committed the three `convert`
unit tests as a `test(...)` commit. A compile-failing RED commit would have
registered a broken intermediate crate in `default-members`; tests were written
after and confirmed green. No separate failing-RED commit exists (same rationale
as plan 01). The plan type is `execute` (no plan-level TDD gate), and MVP/TDD
runtime gate mode was not signaled by the orchestrator.

### Minor

- **Bindings lint allow** uses the `nonstandard_style` lint group rather than
  listing `non_upper_case_globals`/`non_camel_case_types`/`non_snake_case`, so the
  generated `#![allow(...)]` stays on one line and rustfmt-stable across
  regeneration.
- **Push/pop module choice:** wired via **CORE2** (`DomainRangePushEx`/
  `DomainRangePop`) rather than CORE, because the verbatim `RangePush` variant
  carries a `domain` handle. Non-domain CORE push/pop and the remaining event
  kinds are Phase 1 plan 04 scope (the objective wires "ONE representative event
  kind end-to-end").

## Threat Mitigations (from plan `<threat_model>`)

| Threat ID | Disposition | Status |
|-----------|-------------|--------|
| T-02-01 (panic across C ABI) | mitigate | `catch_unwind` wraps every callback body |
| T-02-02 (const char* UAF) | mitigate | message copied into owned `String` inside convert |
| T-02-03 (read past `size`) | mitigate | all attribute reads guarded by `nvtxEventAttributes_t.size` |
| T-02-04 (injected `.so` trust) | accept | opt-in `NVTX_INJECTION64_PATH`; documented boundary |
| T-02-05 (static strong-symbol) | accept | feature-gated `static-injection`; cdylib path is primary |
| T-02-SC (build-dep supply chain) | mitigate | NVTX pinned+allow-listed; bindgen/cc/cargo_metadata `[OK]`; `cargo deny` gate green |

## Scope Notes (not stubs)

Only push/pop are wired end-to-end at the callback layer — this is the plan's
explicit objective ("ONE representative event kind"). RangeStart/End, Mark,
Domain*, RegisterString, NameCategory/Thread, and Resource* callbacks are Phase 1
plan 04. `convert` already handles the full `NvtxEventAttributes` (category,
color, message, CORE payload union), so widening is additive.

## Commits

- ce20562 — feat(01-02): scaffold quent-nvtx-injection cdylib with hermetic committed bindings
- 9f8fb5b — feat(01-02): NVTX init entry, one-shot CORE2 table fill, sink-agnostic hook, push/pop end-to-end
- 2f99cfb — test(01-02): pure convert tests — verbatim payload, size-bounded reads, registered handle

## Self-Check: PASSED

- FOUND: integrations/nvtx/injection/Cargo.toml
- FOUND: integrations/nvtx/injection/build.rs
- FOUND: integrations/nvtx/injection/wrapper.h
- FOUND: integrations/nvtx/injection/c/symbol.c
- FOUND: integrations/nvtx/injection/src/bindings.rs
- FOUND: integrations/nvtx/injection/src/lib.rs
- FOUND: integrations/nvtx/injection/src/init.rs
- FOUND: integrations/nvtx/injection/src/callbacks.rs
- FOUND: integrations/nvtx/injection/src/convert.rs
- FOUND: commit ce20562
- FOUND: commit 9f8fb5b
- FOUND: commit 2f99cfb
