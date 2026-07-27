---
phase: 2
slug: nvtx-model-tolerant-analyzer
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-07-23
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `02-RESEARCH.md` §Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` (workspace convention; CLAUDE.md) |
| **Config file** | none — `#[cfg(test)] mod tests` per source file + `tests/` integration dir (mirrors `integrations/nvtx/example/tests/capture.rs`) |
| **Quick run command** | `cargo test -p nvtx-analyzer` |
| **Full suite command** | `cargo test -p nvtx-analyzer --all-features` |
| **Estimated runtime** | ~30 seconds (pure in-process reconstruction; no GPU/network) |

> **NVTX crates are `members`-only, not `default-members`** (root `Cargo.toml:28-31,79-115`). Bare
> `cargo test` skips them — local runs and CI MUST pass `-p nvtx-analyzer` (or `--workspace`).

---

## Sampling Rate

- **After every task commit:** `cargo test -p nvtx-analyzer` + `cargo clippy -p nvtx-analyzer --all-targets --all-features -- -D warnings`
- **After every plan wave:** `cargo test -p nvtx-analyzer --all-features` + `cargo fmt --check`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

> Test names are the researcher's proposed handles; the planner may rename. Every phase requirement
> has an automated command. `thread_id` capture addition (D-17, Wave 0) unblocks the multi-thread
> ANA-03 case.

| Requirement | Behavior | Test Type | Automated Command | File Exists | Status |
|-------------|----------|-----------|-------------------|-------------|--------|
| D-17 (Wave 0) | `RangePush`/`RangePop` carry `thread_id: u32` end-to-end (capture → event → importer) | unit + integration | `cargo test -p nvtx-events thread_id_on_pushpop` | ❌ W0 | ⬜ pending |
| ANA-01 | Registered handle → string, keyed by `(domain,handle)` | unit | `cargo test -p nvtx-analyzer resolve_registered_string` | ❌ W0 | ⬜ pending |
| ANA-02 | Domain + category names, `(domain,category)` namespacing (no global collision) | unit | `cargo test -p nvtx-analyzer category_namespaced_by_domain` | ❌ W0 | ⬜ pending |
| ANA-02 / D-14 | Stable placeholders for unresolved domain/string/category (exact strings) | unit | `cargo test -p nvtx-analyzer placeholder_stable` | ❌ W0 | ⬜ pending |
| ANA-03 | Per-thread nested Push/Pop stacks (Pop matches most recent Push on same thread) | unit | `cargo test -p nvtx-analyzer pushpop_nested_per_thread` | ❌ W0 (unblocked by D-17) | ⬜ pending |
| ANA-04 | RangeStart/End matched by handle across threads | unit | `cargo test -p nvtx-analyzer startend_match_by_handle` | ❌ W0 | ⬜ pending |
| ANA-05 | Unclosed range → synthetic close at trace-end + flag; no panic | unit | `cargo test -p nvtx-analyzer unclosed_closed_at_trace_end` | ❌ W0 | ⬜ pending |
| ANA-05 | Out-of-order events → correct sorted reconstruction | unit | `cargo test -p nvtx-analyzer out_of_order_sorted` | ❌ W0 | ⬜ pending |
| ANA-05 | Duplicate timestamps → deterministic, no panic | unit | `cargo test -p nvtx-analyzer duplicate_timestamps_no_panic` | ❌ W0 | ⬜ pending |
| ANA-06 | count/total/avg/min/max per `(name,domain,category)` | unit | `cargo test -p nvtx-analyzer range_statistics` | ❌ W0 | ⬜ pending |
| MOD-01 | Range materializes as an `NvtxSpan` (start/end interval), own span type (no `RtFsm`) | unit | `cargo test -p nvtx-analyzer range_is_span` | ❌ W0 | ⬜ pending |
| MOD-02 | Marks/domains/threads/categories present in model | unit | `cargo test -p nvtx-analyzer model_surface_present` | ❌ W0 | ⬜ pending |
| D-09 | Resource lifespan (`Create→Destroy` by handle) + `identifier_type` label | unit | `cargo test -p nvtx-analyzer resource_lifespan` | ❌ W0 | ⬜ pending |
| Criterion 3 | Full malformed stream analyzes to completion, no panic/abort | integration | `cargo test -p nvtx-analyzer malformed_stream_completes` | ❌ W0 | ⬜ pending |
| Happy path | Real `nvtx-example` capture reconstructs expected labels | integration | `cargo test -p nvtx-analyzer example_capture_roundtrip` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] **Capture vocabulary (D-17):** add `thread_id: u32` to `RangePush`/`RangePop` in
      `integrations/nvtx/events` + stamp OS thread id in the injection callbacks
      (`init.rs`/`convert.rs`/`callbacks.rs`), same id space as `NameThread`. Prerequisite for ANA-03.
- [ ] New crate `integrations/nvtx/analyzer` (`Cargo.toml` + `src/lib.rs`) — registered in workspace
      `members` only (not `default-members`).
- [ ] `tests/` integration file mirroring `example/tests/capture.rs` (real-capture happy path).
- [ ] Synthetic-fixture helper: build `Vec<Event<NvtxEventEntity>>` with explicit timestamps via
      `Event::new(id, ts, NvtxEventEntity(..))` — needed for every malformed/tolerance test.
- [ ] Placeholder-string constants + exact-match assertions (Success Criterion 2 stability).

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real GPU-library (libcudf/cuCascade) multi-threaded capture reconstructs correctly | ANA-03 | Requires GPU hardware; CI is GPU-free (VAL-01). Synthetic multi-thread fixtures cover the logic. | Deferred to Phase 5 (real-GPU-workload validation). |

*All Phase-2 core behaviors have automated verification via synthetic fixtures; only real-GPU
confirmation is manual and out of Phase-2 scope.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (thread_id addition, new crate, fixtures, placeholders)
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-07-23 (plan-checker: 0 blockers)
