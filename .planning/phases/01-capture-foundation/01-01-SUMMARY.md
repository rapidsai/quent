---
phase: 01-capture-foundation
plan: 01
subsystem: nvtx-vocabulary
tags: [nvtx, events, serde, vocabulary, workspace]
requires: []
provides:
  - quent-nvtx-events crate (NvtxEvent, NvtxMessage, NvtxEventKind vocabulary)
  - CORE nvtxEventAttributes payload union (verbatim)
  - deferred payload-extension vocabulary (defined, unwired)
  - local EntityEvent contract for the NVTX stream
affects:
  - Cargo.toml (workspace members + default-members)
  - .planning/REQUIREMENTS.md (CAP-03 re-scope)
tech-stack:
  added: []
  patterns:
    - enum-of-structs serde vocabulary (mirrors crates/events/src/trace.rs)
    - transparent newtype carrying an entity name
    - zero Quent-internal deps for upstream separability (D-03)
key-files:
  created:
    - integrations/nvtx/events/Cargo.toml
    - integrations/nvtx/events/src/lib.rs
    - integrations/nvtx/events/src/attributes.rs
    - integrations/nvtx/events/src/payload.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - .planning/REQUIREMENTS.md
decisions:
  - Local EntityEvent trait instead of quent_events::EntityEvent to keep the crate Quent-agnostic (D-03) and satisfy the grep-clean Cargo.toml acceptance criterion
  - CORE payload union modeled as payload_type tag + NvtxPayloadValue enum (verbatim, undecoded)
  - Payload-extension vocabulary defined but not wired into NvtxEvent (D-12)
metrics:
  duration: ~15m
  completed: 2026-07-13
  tasks: 2
  files: 7
---

# Phase 1 Plan 01: quent-nvtx-events Vocabulary Summary

Established `quent-nvtx-events` — the verbatim, Quent-agnostic NVTX event
vocabulary (enum-of-structs with raw `u64`/`u32` handles) that every downstream
NVTX crate speaks — plus workspace registration and the D-12 payload re-scope.

## What Was Built

- **`quent-nvtx-events` crate** under `integrations/nvtx/events/`:
  - `NvtxEvent`: an enum-of-structs covering every core NVTX call kind
    (`RangePush`, `RangePop`, `RangeStart`, `RangeEnd`, `Mark`, `DomainCreate`,
    `DomainDestroy`, `RegisterString`, `NameCategory`, `NameThread`,
    `ResourceCreate`, `ResourceDestroy`) with raw handles and no capture-time
    resolution (D-01).
  - `NvtxMessage::{String, RegisteredHandle}` — registered messages keep only
    their raw handle.
  - `NvtxColor`, `NvtxEventAttributes` (captured subset of
    `nvtxEventAttributes_t`).
  - `NvtxPayload` / `NvtxPayloadValue` — the CORE `nvtxEventAttributes` payload
    **union** captured verbatim (raw `payload_type` tag + scalar value).
  - `PayloadExtensionEvent` — payload-extension vocabulary (schema/enum
    registration, binary blob) **defined but not wired** into `NvtxEvent`, so the
    stream can carry it later without a vocabulary-breaking change (D-12).
  - `NvtxEventKind` — a `#[serde(transparent)]` newtype over `NvtxEvent`
    carrying the entity name, with `From<NvtxEvent>` and an `EntityEvent` impl.
  - Six `#[cfg(test)]` serde round-trip tests (every variant, CORE payload each
    scalar, `NvtxMessage`, `NvtxEventKind`, deferred extension vocabulary).
- **Workspace registration**: `integrations/nvtx/events` added to root
  `Cargo.toml` `members` and `default-members` under a new
  `# NVTX integration crates` comment block.
- **D-12 re-scope**: `REQUIREMENTS.md` CAP-03 narrowed to "CORE payload union
  captured now; payload-extension module deferred." ROADMAP success-criterion 2
  already carried this wording from planning, so no ROADMAP change was needed
  (and the worktree may not modify ROADMAP.md).

## Verification

- `pixi run cargo test -p quent-nvtx-events` — 6 passed, 0 failed.
- `pixi run cargo clippy -p quent-nvtx-events --all-targets --all-features --locked -- -D warnings` — clean.
- `pixi run cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — clean (Finished profile; only unrelated pnpm/vite build stdout).
- `pixi run cargo fmt --all -- --check` — clean.
- `grep -v '^//' src/lib.rs | grep -c "impl EntityEvent"` → 1.
- `grep quent-events|quent-instrumentation Cargo.toml` → no match (D-03 separability).
- SPDX header on every new `.rs`.
- CAP-03 extension+defer grep → 1; CAP-03 still in the Phase 1 traceability row.

## Deviations from Plan

### Design reconciliation (Rule 3 — resolved a blocking contradiction)

**1. [Rule 3 - Blocking] Local `EntityEvent` trait instead of `quent_events::EntityEvent`**
- **Found during:** Task 1.
- **Issue:** The plan required BOTH `impl EntityEvent for NvtxEventKind` in
  `lib.rs` AND a Cargo.toml with zero `quent-events` dependency (grep-clean,
  D-03 separability). Using the real `quent_events::EntityEvent` would force a
  `quent-events` dependency; Rust's orphan rule also blocks the plan-03 bridge
  from adding that impl for a foreign type. These two acceptance criteria are
  mutually exclusive with the real trait.
- **Fix:** Defined a local, structurally identical `EntityEvent` trait in the
  vocabulary crate and impl'd it for `NvtxEventKind`. This keeps the crate fully
  Quent-agnostic (zero Quent deps) and satisfies both grep acceptance criteria.
- **Plan-03 reconciliation:** the bridge (plan 03) needs `NvtxEventKind` to
  satisfy the pipeline's `quent_events::EntityEvent` bound (`spawn_forwarder`).
  Because of the orphan rule, plan 03 must either (a) add a feature-gated
  optional `quent-events` dep + real impl to this crate, or (b) wrap
  `NvtxEventKind` in a bridge-local newtype that impls the real trait. Flagging
  for plan 03; no action needed now.
- **Files modified:** `integrations/nvtx/events/src/lib.rs`.
- **Commit:** 9a9bf99.

### TDD Gate Compliance

Task 1 is marked `tdd="true"`, but the plan type is `execute` (no plan-level TDD
gate). RED/GREEN were collapsed into a single `feat` commit: this is a pure data
vocabulary with no behavioral logic, so a compile-failing RED commit (types not
yet defined) carries no behavioral signal and would leave a broken intermediate
commit registered in `default-members`. Tests were written alongside the types
and confirmed green before committing. No separate `test(...)` RED commit exists.

### ROADMAP.md not modified

Plan Task 2 lists ROADMAP.md, but its success-criterion 2 already carried the
D-12 narrowing (set during planning), and worktree agents must not modify
ROADMAP.md. No edit was required or made.

## Known Stubs

None. `PayloadExtensionEvent` is intentionally defined-but-unwired per D-12 (a
planned future-vocabulary placeholder, not a data stub); it is documented in
`payload.rs` and CAP-03/SC-2 as deferred, with PAY-01 as its resolving home.

## Commits

- 9a9bf99 — feat(01-01): add quent-nvtx-events verbatim vocabulary crate
- 9178c9f — docs(01-01): re-scope CAP-03 to core-union-now / extension-deferred (D-12)

## Self-Check: PASSED

- FOUND: integrations/nvtx/events/Cargo.toml
- FOUND: integrations/nvtx/events/src/lib.rs
- FOUND: integrations/nvtx/events/src/attributes.rs
- FOUND: integrations/nvtx/events/src/payload.rs
- FOUND: commit 9a9bf99
- FOUND: commit 9178c9f
