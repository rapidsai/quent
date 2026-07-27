---
phase: 1
slug: capture-foundation
status: planned
nyquist_compliant: true
wave_0_complete: false
created: 2026-07-13
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from 01-RESEARCH.md §"Validation Architecture". The per-task map below is
> populated from the task IDs in 01-01..01-04-PLAN.md.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` (integration tests spawn the deterministic test app as a subprocess; assert over the real ndjson exporter output file) |
| **Config file** | none — workspace `Cargo.toml` members/default-members registration |
| **Quick run command** | `pixi run cargo test -p quent-nvtx-injection` |
| **Full suite command** | `pixi run cargo test --workspace --all-features --locked` |
| **Estimated runtime** | subprocess spawn dominates; GPU-less; expected < ~30 s |

---

## Sampling Rate

- **After every task commit:** Run the quick command for the touched crate
- **After every plan wave:** Run the full suite command
- **Before `/gsd:verify-work`:** Full suite must be green, plus `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo fmt --all -- --check`, and `cargo deny check`
- **Max feedback latency:** ~30 s (subprocess-bound integration test)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-T1 | 01-01 | 1 | CAP-02, CAP-03 | T-01-01 | Lossless verbatim serde round-trip; zero Quent-internal deps (D-03) | unit (TDD) | `pixi run cargo test -p quent-nvtx-events` | ❌ Wave 0 (creates crate + tests) | ⬜ pending |
| 01-01-T2 | 01-01 | 1 | CAP-03 | — | Doc re-scope (D-12): core union now, extension deferred | doc/grep | `grep -i extension .planning/REQUIREMENTS.md \| grep -ci defer` | ✅ (edits existing docs) | ⬜ pending |
| 01-02-T1 | 01-02 | 2 | CAP-01 | T-02-SC | Hermetic/offline default build; feature-gated NVTX git-dep + committed bindings (D-14); deny allow-git (D-13) | build/grep | `pixi run cargo build -p quent-nvtx-injection --locked` (+ `--offline`, `cargo deny check`) | ❌ Wave 0 (creates crate + bindings) | ⬜ pending |
| 01-02-T2 | 01-02 | 2 | CAP-01 | T-02-04, T-02-05 | Unmangled one-shot `InitializeInjectionNvtx2` (OnceLock); Linux-64 compile_error guard (D-04); static path feature-gated (D-15) | build/symbol | `pixi run cargo build -p quent-nvtx-injection --locked` (+ `nm -D` symbol check) | ❌ Wave 0 | ⬜ pending |
| 01-02-T3 | 01-02 | 2 | CAP-02, CAP-03 | T-02-01, T-02-02, T-02-03 | `catch_unwind` at C boundary; `const char*` copy-in; version/size bounds-check (ASVS V5); CORE payload union verbatim | unit (TDD) | `pixi run cargo test -p quent-nvtx-injection convert` | ❌ Wave 0 (creates convert tests) | ⬜ pending |
| 01-03-T1 | 01-03 | 3 | CAP-04, CAP-05 | T-03-01, T-03-02, T-03-SC | Bounded ArrayQueue ring drops-and-counts, producer never blocks (D-16/D-07/D-08); `Event::new_now` capture timestamp (CAP-04) | unit (TDD) | `pixi run cargo test -p quent-nvtx` | ❌ Wave 0 (creates ring drop-count test) | ⬜ pending |
| 01-03-T2 | 01-03 | 3 | CAP-01, CAP-04, VAL-01, VAL-02 | T-03-03 | Subprocess isolation (VAL-02) with `NVTX_INJECTION64_PATH`; deterministic GPU-less app (VAL-01); timestamped events via real ndjson exporter | integration (subprocess) | `pixi run cargo test -p quent-nvtx --test capture_e2e` | ❌ Wave 0 (creates test-app + harness) | ⬜ pending |
| 01-04-T1 | 01-04 | 4 | CAP-02, CAP-03 | T-04-01, T-04-02, T-04-03 | Full-kind `catch_unwind` callbacks; registered-string copy-at-registration; version/size guards; payload union | unit (TDD) | `pixi run cargo test -p quent-nvtx-injection convert` | ❌ Wave 0 (extends convert tests) | ⬜ pending |
| 01-04-T2 | 01-04 | 4 | CAP-02, CAP-03, VAL-01 | T-04-01 | Every core kind + payload union + cross-thread RangeStart/End asserted from ndjson; multi-threaded app (D-11) | integration (subprocess) | `pixi run cargo test -p quent-nvtx --test capture_e2e` | ❌ Wave 0 (extends test-app + harness) | ⬜ pending |
| 01-04-T3 | 01-04 | 4 | VAL-01 | — | Documents committed-bindings / feature-gated-regen workflow (D-14) | doc/grep | `grep -c regenerate-bindings integrations/nvtx/README.md` | ❌ (creates README) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Deterministic in-repo NVTX test-app binary (VAL-01) — thin push/pop app in 01-03-T2, widened to every event kind single + multi-threaded (D-11) in 01-04-T2
- [ ] Subprocess integration harness (VAL-02) — `integrations/nvtx/instrumentation/tests/capture_e2e.rs` created in 01-03-T2, assertions widened in 01-04-T2
- [ ] Drop-count correctness test (CAP-05 / D-08) — bounded ring fills → drop-counted, producer not blocked, in 01-03-T1
- [ ] `NvtxEvent` serde round-trip unit tests (pure, in-process) — 01-01-T1
- [ ] Pure convert unit tests (side-effect-free, Pitfall 6) — 01-02-T3 (push/pop), extended in 01-04-T1 (all kinds)

*Existing infrastructure (`domains/query_engine/tests/fixed/`, `crates/exporter/ndjson/`) provides the deterministic-emitter + ndjson-assertion patterns to mirror.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real-GPU-library capture (libcudf) | (Phase 5) | No GPU in CI; Phase 1 proves the mechanism GPU-less | Deferred to Phase 5 |
| High-frequency load/stress proof | CAP-05 (full) | Design-level only in Phase 1 (D-08) | Deferred to Phase 5 |

*Phase 1 behaviors otherwise have automated verification under GPU-less CI.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (every task above has an automated command)
- [x] Wave 0 covers all MISSING references (test-app, subprocess harness, drop-count test, serde/convert unit tests all scheduled)
- [x] No watch-mode flags (all commands are single-shot `cargo test`/`cargo build`)
- [x] Feedback latency threshold set (~30 s)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved (planning-time; re-verify Wave 0 files exist during execution)
