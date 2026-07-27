---
phase: 01-capture-foundation
verified: 2026-07-14T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
deferred:
  - truth: "Fan-out integration tests run green in subprocesses (fan-out half of success-criterion 4)"
    addressed_in: "Phase 4"
    evidence: "REQUIREMENTS.md traceability: FAN-01/FAN-02/FAN-03 -> Phase 4. Phase 1 requirement set is CAP-01..05, VAL-01, VAL-02 (no FAN). The injection half of SC4 is fully verified now."
notes:
  - "Phase mode is `mvp` but the ROADMAP goal is a descriptive technical goal, not a User Story ('As a..., I want..., so that...'). Standard goal-backward verification was applied against the 5 concrete ROADMAP success criteria (the non-negotiable contract), which are fully testable. Recommend the goal be reworded to a User Story or the mode be cleared, but this did not block verification."
---

# Phase 1: Capture Foundation Verification Report

**Phase Goal:** A running NVTX-emitting application is captured by Quent's injection library with raw events flowing into the standard pipeline, driven by a deterministic in-repo test app under GPU-less CI.
**Verified:** 2026-07-14
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

All five ROADMAP success criteria were verified against the actual codebase by
reading the implementation AND executing the test suites (not by trusting the
SUMMARYs). The end-to-end subprocess test — the single strongest piece of
evidence — was reproduced green with no GPU present.

### Observable Truths

| # | Truth (ROADMAP success criterion) | Status | Evidence |
|---|-----------------------------------|--------|----------|
| 1 | Test app attached via `NVTX_INJECTION64_PATH` (no app code changes) produces raw NvtxEvents covering every core kind (push/pop, start/end, marks, domain create/destroy, registered strings, category naming, thread naming, resource create/destroy) | ✓ VERIFIED | `capture_e2e.rs` subprocess test asserts all 12 kinds present and **passed** when run. `convert.rs` has converters for all 12 variants; `init.rs` registers 12 CBID slots (CORE2 + CORE `NameOsThreadA`). Test-app links nothing Quent; capture is purely env-driven. |
| 2 | CORE `nvtxEventAttributes` payload union captured verbatim; payload-EXTENSION deferred (D-12) | ✓ VERIFIED | e2e asserts the mark's payload round-trips as `NvtxPayload { payload_type: 1, UnsignedInt64(0xCAFEF00D) }`. No `PayloadSchema/EnumRegister` wiring (grep empty). REQUIREMENTS.md CAP-03 re-scoped to "CORE now / extension DEFERRED". |
| 3 | Every captured event carries a capture-time timestamp and reaches a standard exporter through `EventSender` | ✓ VERIFIED | e2e asserts `event.timestamp > 0` for every event. `install()` hook stamps via `Event::new_now` (lib.rs:211); drain thread forwards through `observer.send` → `EventSender::send` (lib.rs:121,127). Unit test `drain_forwards_events_in_order_to_ndjson` confirms real ndjson exporter round-trip. |
| 4 | Injection (and fan-out) integration tests run green in subprocesses under CI with no GPU | ✓ VERIFIED (injection scope) | `capture_e2e.rs` uses `std::process::Command` subprocess isolation (VAL-02) and passed with no GPU (VAL-01). **Fan-out** half is DEFERRED to Phase 4 (FAN-01/02/03 → Phase 4; not in Phase 1's requirement set). |
| 5 | Test app runs under high-frequency emission with no capture-path blocking on locks/IO/unbounded allocation (stamp-and-hand-off) | ✓ VERIFIED | `ring_drops_and_counts_when_full` proves a full bounded `ArrayQueue` drops-and-counts and the producer returns effectively instantly (asserts `< 100ms`, `DROPPED` increments). Hot-path is lock-free `push_or_drop` (non-blocking), drain runs off the app thread. Mechanism proven by the overflow test rather than a sustained load benchmark. |

**Score:** 5/5 truths verified

Plan-frontmatter truths (01-01…01-04) were also cross-checked and all hold:
verbatim vocabulary + serde round-trip (6 tests green), one-shot
`InitializeInjectionNvtx2` + `catch_unwind` at every C boundary (14 boundaries),
committed hermetic bindings (offline build green, no libclang), Linux-64-only
`compile_error!` guard, bounded ring + drop-count, and the multi-threaded
cross-thread RangeStart/End + per-thread naming coverage (all asserted by the
passing e2e test).

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | Fan-out integration tests (the "and fan-out" clause of success-criterion 4) | Phase 4 | REQUIREMENTS.md traceability maps FAN-01/02/03 to Phase 4; Phase 1's declared requirements are CAP + VAL only. Not an actionable Phase-1 gap. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `integrations/nvtx/events/src/lib.rs` | NvtxEvent enum (12 kinds), NvtxMessage, NvtxEventKind + EntityEvent | ✓ VERIFIED | All 12 variants present; 6 serde tests green |
| `integrations/nvtx/events/src/payload.rs` | CORE payload union + deferred extension vocab | ✓ VERIFIED | NvtxPayload/NvtxPayloadValue + unwired PayloadExtensionEvent |
| `integrations/nvtx/injection/src/init.rs` | InitializeInjectionNvtx2, OnceLock one-shot, table fill | ✓ VERIFIED | Unmangled symbol exported (nm=1); 12 CBID subscribes; `next_handle` synthesis |
| `integrations/nvtx/injection/src/callbacks.rs` | extern C callbacks w/ catch_unwind | ✓ VERIFIED | 14 catch_unwind boundaries |
| `integrations/nvtx/injection/src/convert.rs` | pure args→NvtxEvent, size-bounded reads | ✓ VERIFIED | 10 convert tests green; size-bounded `read_present` |
| `integrations/nvtx/injection/src/bindings.rs` | committed bindgen output (D-14) | ✓ VERIFIED | Offline build green, no libclang |
| `integrations/nvtx/instrumentation/src/lib.rs` | bounded ArrayQueue ring + drop-count + drain + Event::new_now | ✓ VERIFIED | 2 unit tests green |
| `integrations/nvtx/instrumentation/tests/capture_e2e.rs` | subprocess harness | ✓ VERIFIED | Full-coverage test passed GPU-less |
| `integrations/nvtx/instrumentation/src/bin/nvtx_test_app.rs` | deterministic multi-thread emitter | ✓ VERIFIED | Links nothing Quent; e2e drives it |
| `integrations/nvtx/README.md` | regenerate-bindings doc (D-14) | ✓ VERIFIED | Contains regeneration command |
| `deny.toml` | allow-git NVIDIA/NVTX (D-13) | ✓ VERIFIED | Entry present; git-dep pinned rev 7d113f290f89 (v3.5.0), optional |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| injection hook | `quent_nvtx_events::NvtxEvent` | convert + hook invocation | ✓ WIRED | callbacks.rs → convert → hook, all 12 kinds |
| bridge `install` | `install_hook` | hook pushes Event::new_now onto ring | ✓ WIRED | lib.rs:210-212 |
| drain thread | `EventSender::send` | `observer.send` forwards ring-popped events | ✓ WIRED | lib.rs:121,127 |
| e2e harness | built cdylib | `NVTX_INJECTION64_PATH` env on subprocess | ✓ WIRED | capture_e2e.rs:92; test passed |
| workspace Cargo.toml | 3 nvtx crates | members + default-members | ✓ WIRED | all three registered |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full-coverage capture e2e (SC 1-4) | `cargo test -p quent-nvtx --features e2e --test capture_e2e` | 1 passed | ✓ PASS |
| Bridge ring/drain (SC 5, CAP-04) | `cargo test -p quent-nvtx --features e2e --lib` | 2 passed | ✓ PASS |
| Verbatim convert, all kinds (CAP-02/03) | `cargo test -p quent-nvtx-injection` | 10 passed | ✓ PASS |
| Vocabulary serde round-trip | `cargo test -p quent-nvtx-events` | 6 passed | ✓ PASS |
| Unmangled injection entry exported | `nm -D libquent_nvtx.so \| grep -c InitializeInjectionNvtx2` | 1 | ✓ PASS |
| Hermetic offline build (no libclang, no NVTX fetch) | `cargo build -p quent-nvtx-injection --locked --offline` | Finished | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CAP-01 | 01-02, 01-03 | Attach via NVTX_INJECTION64_PATH, no app changes | ✓ SATISFIED | e2e test-app links nothing Quent; env-driven capture; test green |
| CAP-02 | 01-01, 01-02, 01-04 | All core NVTX kinds captured verbatim | ✓ SATISFIED | e2e asserts all 12 kinds; convert+init cover all |
| CAP-03 | 01-01, 01-02, 01-04 | CORE payload union verbatim; extension deferred | ✓ SATISFIED | e2e payload round-trip; REQUIREMENTS re-scoped |
| CAP-04 | 01-03 | Timestamped, flows through EventSender | ✓ SATISFIED | Event::new_now + observer.send; timestamp asserted |
| CAP-05 | 01-03 | Never blocks (bounded ring, drop-count) | ✓ SATISFIED | ring_drops_and_counts_when_full |
| VAL-01 | 01-03, 01-04 | Deterministic in-repo test app, GPU-less | ✓ SATISFIED (capture leg) | nvtx_test_app + e2e green no GPU. (VAL-01's model→analyzer→endpoint legs belong to Phases 2/3.) |
| VAL-02 | 01-03 | Subprocess isolation | ✓ SATISFIED | std::process::Command subprocess |

All 7 declared Phase-1 requirement IDs are accounted for and satisfied for their
Phase-1 scope. No orphaned requirements (REQUIREMENTS.md maps exactly CAP-01..05,
VAL-01, VAL-02 to Phase 1, all claimed by the plans).

### Anti-Patterns Found

Debt-marker scan (TBD/FIXME/XXX/todo!/unimplemented!) across all nvtx sources:
**clean** (no markers). No stubs — all "Known Stubs" sections report None; the
unwired `PayloadExtensionEvent` is a documented D-12 deferral, not a data stub.

The following are soundness/correctness edges carried from `01-REVIEW.md`
(status: issues_found, 0 critical, 4 warning). None block the Phase-1 goal
(capture works end-to-end), but they should be tracked into later phases:

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| instrumentation/src/lib.rs | 190-217 | Drain thread/runtime/exporter leak if `install_hook` returns AlreadyInstalled (spawn precedes hook install) | ⚠️ Warning (WR-01) | Leak only on the one-shot already-installed error path; does not affect the happy path proven by tests |
| injection/src/convert.rs | 306-307, 396-397 | 8-byte `u64` read of a union whose active member may be 32-bit → reads possibly-uninit padding (technically UB) | ⚠️ Warning (WR-02) | Does not fire in e2e (emit.c zero-inits); real clients using standard init macros may not zero the full union. Fix before GPU-lib validation (Phase 5) |
| injection/src/callbacks.rs | 22-44, 84-93 | RangePush/RangePop return constant 0, not NVTX nesting depth | ⚠️ Warning (WR-04) | Silent semantic change to an instrumented app relying on push/pop return; relevant to the "don't break the app" / coexistence constraint (Phase 4/5) |
| instrumentation/tests/capture_e2e.rs | 76 | e2e test references `CARGO_BIN_EXE_nvtx_test_app` without a `#![cfg(feature="e2e")]` guard | ℹ️ Info (WR-03) | Did NOT reproduce on this toolchain — `cargo test -p quent-nvtx --no-run` (default features) compiled the test target cleanly. Cosmetic robustness only |

### Human Verification Required

None. Every success criterion is verifiable and was verified by automated tests
executed during this review (no visual/UX/real-time/external-service surface in
this phase).

### Gaps Summary

No gaps. All five ROADMAP success criteria are observably true in the codebase and
proven by green tests reproduced during verification. The one clause not satisfied
now — "fan-out integration tests" in success-criterion 4 — is legitimately
deferred to Phase 4 (FAN-01/02/03), consistent with the REQUIREMENTS traceability
and the Phase-1 requirement set, so it is recorded as a deferred item rather than a
gap.

The four review warnings (WR-01, WR-02, WR-04 and the cosmetic WR-03) are
non-blocking soundness/coexistence edges. WR-02 (uninit union read) and WR-04
(push/pop return contract) are the most worth carrying forward, as both bear on the
"observe without breaking the application" project constraint that Phases 4-5 must
honor.

---

_Verified: 2026-07-14_
_Verifier: Claude (gsd-verifier)_
