---
phase: 02-nvtx-model-tolerant-analyzer
plan: 04
subsystem: analyzer
tags: [nvtx, analyzer, push-pop, per-thread-stacks, nesting, tolerance, rust]

# Dependency graph
requires:
  - phase: 02-nvtx-model-tolerant-analyzer
    plan: 01
    provides: thread_id on RangePush/RangePop end-to-end (D-17)
  - phase: 02-nvtx-model-tolerant-analyzer
    plan: 02
    provides: nvtx-analyzer crate, NvtxSpan/SpanKind/SpanId, two-pass NvtxModelBuilder, StartEndRanges
  - phase: 02-nvtx-model-tolerant-analyzer
    plan: 03
    provides: ResolutionTables::resolve_message, model surface, marks
provides:
  - "PushPopRanges — one Vec<OpenPushSpan> stack per (thread_id, domain) (ANA-03)"
  - "Explicit parent capture at pop time, so NvtxSpan::parent is populated for nested ranges"
  - "Span slot reservation: a SpanId is handed out at push time, making a still-open parent referenceable"
  - "Synthetic close of leaked pushes at trace end, with nesting preserved"
  - "Orphan-pop tolerance (warn + skip) and a whole-stream no-panic proof"
affects: [02-05, 03, ANA-03, ANA-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Slot reservation: push reserves an index in a Vec<Option<NvtxSpan>>, pop fills it — the only way a child can name a parent that has not closed yet while SpanId stays an index"
    - "Stack key is the (thread_id, domain) tuple, the analyzer-side view of the capture layer's thread-local-keyed-by-domain RANGE_DEPTH"
    - "Parent is read from the stack after the pop — the range the pop uncovered — never tracked separately"
    - "Every tolerance path is a total function: Vec::pop, get_mut, get_mut(id) — no unwrap, so malformed input degrades instead of aborting"

key-files:
  created:
    - "integrations/nvtx/analyzer/tests/pushpop.rs"
  modified:
    - "integrations/nvtx/analyzer/src/ranges.rs"
    - "integrations/nvtx/analyzer/src/model.rs"
    - "integrations/nvtx/analyzer/tests/fixtures.rs"

key-decisions:
  - "SpanIds are reserved at push time, not assigned at pop time: a child pops before its parent, so the parent's id must exist before the parent's span does"
  - "spans() ordering is now by opening rather than by completion — the necessary consequence of slot reservation, and the doc comment says so"
  - "Parent is derived from the live stack (stack.last() after the pop) rather than stored on the open record — one source of truth for nesting"
  - "Leaked pushes keep their nesting: the stack's shape at trace end is exactly the parent chain, so a synthetic close is not a flattening"
  - "Empty stacks are removed from the map on pop, mirroring range_pop_level's own cleanup, so the map tracks open ranges rather than every thread ever seen"

patterns-established:
  - "Two range reconstructions, two match keys, one module: correlate-by-id (StartEnd) and correlate-by-stack-position (PushPop) documented side by side so the distinction is unmissable"

requirements-completed: [ANA-03, ANA-05]

# Metrics
duration: ~15 min
completed: 2026-07-28
---

# Phase 2 Plan 04: Per-Thread Push/Pop Stacks Summary

**`RangePush`/`RangePop` now reconstruct as genuine nested stacks keyed by `(thread_id, domain)` — a pop closes the innermost push on its own thread and nothing else, records its parent from the range the pop uncovered, and every unbalanced case (leaked push, orphan pop, cross-thread pop) degrades to a logged warning rather than a panic.**

## Performance

- **Duration:** ~15 min
- **Tasks:** 2 (RED tests, GREEN implementation)
- **Files created:** 1; modified: 3
- **Tests:** 14 passing (9 from plans 02-02/02-03, 5 new)

## Accomplishments

- **ANA-03 satisfied at the correct grain, and proven at it.** `PushPopRanges` holds a `HashMap<(u32, u64), Vec<OpenPushSpan>>`. The grain is not inferred — it is the analyzer-side view of the injection layer's `RANGE_DEPTH`, a `thread_local!` keyed by domain, so capture and reconstruction agree by construction.
- **The interleaved test is the load-bearing one.** `pushpop_nested_per_thread` runs thread 1 pushing `a`, thread 2 pushing `b`, thread 1 pushing `c`, then pops `c`, `a`, `b`. A global stack reconstructs this stream *successfully* into wrong nesting — `b` would close on thread 1's first pop. The test pins `b.end == 150` and `b.parent == None`, so that failure mode is now caught rather than shipped. This is exactly RESEARCH Pitfall 1.
- **Solved the parent-before-close ordering problem.** `SpanId` is documented as an index into `NvtxModel::spans()`, but a child pops *before* its parent, so at the moment a child needs `parent` the parent has no span yet. Pass 2 now reserves a slot (`Vec<Option<NvtxSpan>>`) at push time and fills it at pop time, so the parent's id is knowable while the parent is still open — without weakening `SpanId` into an opaque handle needing a side lookup.
- **Leaked pushes keep their nesting.** The drain at trace end walks each stack innermost-first, deriving each open push's parent from the entry below it. `unclosed_closed_at_trace_end` asserts the inner leaked push still points at the outer one, so a truncated capture yields a nested tree rather than a flat pile.
- **Whole-core no-panic proof (success criterion 3).** `malformed_stream_completes` runs one stream containing an out-of-order end, an orphan `RangeEnd`, an unmatched `RangeStart`, duplicate timestamps across two threads, an orphan pop, a leaked push, a cross-thread pop, a mark, and a push in an unrelated domain. It builds, produces exactly the 6 expected spans, holds `start <= end` on every one, and rebuilds identically.
- All 9 prior tests are untouched and still green.

## Task Commits

Each task committed atomically (DCO sign-off, Conventional Commits):

1. **Task 1: Failing per-thread Push/Pop + tolerance tests (RED)** — `3cec89f` (test)
2. **Task 2: Per-(thread_id, domain) stacks + synthetic close + orphan skip (GREEN)** — `93e7f4d` (feat)

## Files Created/Modified

- `integrations/nvtx/analyzer/tests/pushpop.rs` *(created)* — the five tests plus a `span_id` helper that reads a `SpanId` back as a position in `spans()`, which is what makes the `parent` assertions meaningful.
- `integrations/nvtx/analyzer/src/ranges.rs` — added `StackKey`, `OpenPushSpan` (with `close` returning `(SpanId, NvtxSpan)`), and `PushPopRanges` with `push`/`pop`/`close_at_trace_end`. Module doc rewritten to contrast the two match keys.
- `integrations/nvtx/analyzer/src/model.rs` — pass 2 dispatches `RangePush`/`RangePop`; span accumulation became `Vec<Option<NvtxSpan>>` slots with a bounds-checked `fill` helper; `spans()` doc updated for the new ordering contract.
- `integrations/nvtx/analyzer/tests/fixtures.rs` — added `range_push` / `range_pop` constructors carrying `thread_id`.

## Decisions Made

- **Reserve the slot at push, do not assign the id at pop.** The alternative was making `SpanId` an opaque counter with a side map from id to index, which would have changed a public type's meaning and pushed a lookup onto every consumer. Reservation keeps `SpanId` an index and costs one `Option` per span during the build only.
- **`spans()` is now ordered by opening, not completion.** This is forced by reservation and is stated in the doc comment. It is arguably the better contract anyway: a parent now always precedes its children, so a consumer walking the list builds the tree in one forward pass. Determinism is unaffected (it is still a pure function of the timestamp-ordered stream), and the rebuild-equality assertions in both `duplicate_timestamps_no_panic` and `malformed_stream_completes` cover it.
- **Parent comes from the live stack, not from the open record.** `stack.last()` *after* the pop is the enclosing range by definition. Storing a parent on `OpenPushSpan` at push time would duplicate that information and let the two disagree under an unbalanced stream.
- **Empty stacks are removed on pop.** Not required for correctness — `Vec::pop` on an empty stack is already `None` — but it mirrors `range_pop_level`'s own `map.remove(&domain)` and keeps the map proportional to *open* ranges rather than to every `(thread, domain)` the process ever touched, which narrows T-02-06.
- **The pop path is bounds-checked twice over.** `self.stacks.get_mut(&key).and_then(Vec::pop)` covers both "no stack for this key" and "stack exists but is empty" in one expression, and `fill` uses `slots.get_mut` rather than indexing. Neither can panic on adversarial input.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `SpanId` could not be assigned at pop time as the plan's action implies**

- **Found during:** Task 2
- **Issue:** The plan says to "capture `parent` from the new top-of-stack (its `SpanId`)" and separately to "assign `SpanId`s so `parent` references resolve". Those two cannot both hold if ids are assigned when a span is appended: `SpanId` is documented as an index into `spans()`, a child pops before its parent, and so at child-pop time the parent has no index. Implementing it literally would have produced `parent` references that were off by the number of spans closing between the child and the parent — wrong nesting that still looks plausible.
- **Fix:** Reserve the span's slot at push time (`Vec<Option<NvtxSpan>>`), hand the reserved `SpanId` to the open record, and fill the slot at pop or at the trace-end drain. `SpanId` keeps its documented meaning, and every reserved slot is filled by construction so flattening preserves indices.
- **Files modified:** `integrations/nvtx/analyzer/src/model.rs`, `integrations/nvtx/analyzer/src/ranges.rs`
- **Commit:** `93e7f4d`

**2. [Rule 2 - Missing critical functionality] `spans()` ordering doc was left stale by the fix above**

- **Found during:** Task 2
- **Issue:** The existing doc promised "ordered by completion: spans whose close was observed come first… followed by any synthetically closed at trace end". Slot reservation makes that false for push/pop spans. A stale ordering contract on a public accessor is worse than none — a Phase 3 consumer would rely on it.
- **Fix:** Rewrote the doc to state the actual contract (index *is* the `SpanId`; ordering is by opening) and why it matters.
- **Files modified:** `integrations/nvtx/analyzer/src/model.rs`
- **Commit:** `93e7f4d`

**3. [Rule 3 - Blocking] `tests/fixtures.rs` modified, though Task 1's `<files>` lists only `tests/pushpop.rs`**

- **Found during:** Task 1
- **Issue:** No `RangePush`/`RangePop` fixture constructors existed.
- **Fix:** Added `range_push`/`range_pop` to `fixtures.rs`. Task 1's `<action>` explicitly sanctions this ("Extend the fixture helper (if needed)"); only the `<files>` list omitted it.
- **Files modified:** `integrations/nvtx/analyzer/tests/fixtures.rs`
- **Commit:** `3cec89f`

### Environment deviation (pre-execution)

**Worktree spawned from the wrong base — same misconfiguration as plans 02-02 and 02-03.** This executor's worktree (`worktree-agent-a161831d73ff8bfc5`) was created at `f31e60b`, a commit on `main`'s history, rather than the specified base `8f9db73`, so `.planning/`, `CLAUDE.md`, and `integrations/nvtx/` were absent. The branch is in the sanctioned `worktree-agent-*` namespace, the tree was clean, and `8f9db73..HEAD` was empty while `HEAD..8f9db73` was not — i.e. HEAD was strictly an ancestor with zero unique commits — so it was aligned with `git reset --hard 8f9db73`, the sanctioned startup branch-check remedy on a per-agent branch. No commits were destroyed and no protected ref was touched.

---

**Total deviations:** 3 auto-fixed (2× Rule 3, 1× Rule 2) + 1 pre-execution environment correction.
**Impact on plan:** No scope change. Deviation 1 is a correctness fix to the plan's own mechanism (the plan's stated goal — resolvable `parent` references — is met; only the id-assignment timing changed); deviation 2 follows from it; deviation 3 is a file-list omission the plan's action text already permitted.

## Requirement Status

- **ANA-03** (per-thread nested Push/Pop stacks; a Pop matches the most recent Push on the same thread): **COMPLETE.** Keyed `(thread_id, domain)`, matching the capture layer's grain. Proven against interleaved threads, three-deep single-thread nesting, and a cross-thread pop that must not match.
- **ANA-05** (tolerant of malformed streams): **COMPLETE** for the push/pop path. Orphan pops skip, leaked pushes close synthetically with nesting intact, and the combined malformed stream builds to completion.

## Threat Model Compliance

| Threat ID | Disposition | Implementation |
|-----------|-------------|----------------|
| T-02-01 | mitigate | The pop is `self.stacks.get_mut(&key).and_then(Vec::pop)` — a missing stack and an empty stack both yield `None`, warn, and skip. Slot writes go through `slots.get_mut(id.0)`, not indexing. `grep -rn "unwrap()\|expect(\|panic!" src` → **0 matches** across all 6 source files. `malformed_stream_completes` exercises the unbalanced paths end to end. |
| T-02-03 | mitigate | `end.max(self.start)` in `OpenPushSpan::close`, matching `OpenStartRange::close`. `malformed_stream_completes` asserts `start <= end` on every span, and `NvtxSpan::duration` is saturating on top of that. |
| T-02-06 | accept | Stack growth is bounded by the finite captured session, as planned. Narrowed slightly: empty stacks are removed on pop, so the map size tracks currently-open ranges rather than every `(thread, domain)` ever observed. A depth cap remains the documented answer if streaming ingestion is added later. |

## Known Stubs

Intentional and scoped by later plans in this phase:

| Stub | File | Resolution |
|------|------|------------|
| `ResourceCreate` contributes only its domain; resource records and `identifier_type` labels absent | `src/tables.rs` | Plan 02-05 |
| `SpanKind::Resource` still unconstructed | `src/span.rs` | Plan 02-05 |
| `NvtxMark::thread_id` is always `None` — `nvtxDomainMarkEx` carries no thread id in the vocabulary | `src/model.rs` | By design (field kept per D-03) |

`SpanKind::PushPop` and `NvtxSpan::parent` are no longer stubs — both are constructed and asserted by this plan. No remaining stub prevents this plan's goal from being achieved.

## Verification Evidence

- `cargo test -p nvtx-analyzer` — ok, **14 passed, 0 failed** (5 reconstruction + 4 resolution + 5 new pushpop). Zero doc-test failures.
- RED confirmed before implementation: all 5 new tests failed on *assertions*, not missing symbols — e.g. `assertion left == right failed: one span per matched push/pop pair, left: 0, right: 3` and `…every open range became a span, left: 2, right: 6` — while the 9 pre-existing tests stayed green.
- `cargo clippy -p nvtx-analyzer --all-targets --all-features -- -D warnings` — clean.
- `cargo fmt --all -- --check` — clean (exit 0).
- `cargo doc -p nvtx-analyzer --no-deps` — no warnings; intra-doc links (`[SpanId]`, `[NvtxSpan::parent]`, `[NvtxModel::spans]`) resolve.
- `grep -c "warn!" src/ranges.rs` → **5** (plan requires >= 2: orphan pop, synthetic push close, plus the three pre-existing start/end paths).
- `grep -Eq "thread_id.*domain|\(thread" src/ranges.rs` → present (`type StackKey = (u32, u64)`, `HashMap<StackKey, Vec<OpenPushSpan>>`, `entry((thread_id, domain))`).
- `grep -rc "quent_analyzer\|quent_model\|RtFsm" src` → **0** for all 6 files (framework-free contract holds).
- `grep -rn "unwrap()\|expect(\|panic!" src` → no matches (`expect`/`panic!` appear only in test code).
- `grep -rn "nvtx-analyzer" --include=Cargo.toml` → only its own manifest; no other crate depends on it, so the `spans()` ordering change has no downstream consumer to break.
- No file deletions across either commit (`git diff --diff-filter=D --name-only 3cec89f~1 HEAD` empty); no untracked files left behind (`git status --short` empty).

## TDD Gate Compliance

Gate sequence satisfied: `test(02-04)` RED commit `3cec89f` precedes `feat(02-04)` GREEN commit `93e7f4d`. RED was verified by running the suite and observing assertion failures before any source change. No REFACTOR commit was needed.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 02-05 (resources) is unblocked and untouched by this plan: it adds a third reconstruction alongside `StartEndRanges` and `PushPopRanges`, and `slots.push(Some(span))` is the same append `RangeEnd` already uses (resource lifespans do not nest, so they need no reservation).
- Phase 3 (UI) gets what RESEARCH §Nested-range representation recommended: explicit `parent` *and* flat `start`/`end`, so swim-lane rendering (UI-02) can use either. Parents now always precede children in `spans()`, so tree construction is a single forward pass.
- No blockers introduced.

## Self-Check: PASSED

- Files verified present: `integrations/nvtx/analyzer/tests/pushpop.rs` (FOUND), `integrations/nvtx/analyzer/tests/fixtures.rs` (FOUND), `integrations/nvtx/analyzer/src/ranges.rs` (FOUND), `integrations/nvtx/analyzer/src/model.rs` (FOUND).
- Commits verified present: `3cec89f` (FOUND), `93e7f4d` (FOUND).

---
*Phase: 02-nvtx-model-tolerant-analyzer*
*Completed: 2026-07-28*
