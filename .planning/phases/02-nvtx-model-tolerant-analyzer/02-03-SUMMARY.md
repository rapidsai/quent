---
phase: 02-nvtx-model-tolerant-analyzer
plan: 03
subsystem: analyzer
tags: [nvtx, analyzer, handle-resolution, placeholders, marks, domains, categories, rust]

# Dependency graph
requires:
  - phase: 02-nvtx-model-tolerant-analyzer
    plan: 02
    provides: nvtx-analyzer crate, NvtxSpan/SpanKind, two-pass NvtxModelBuilder, StartEndRanges
  - phase: 01-capture-foundation
    provides: nvtx-events registration variants (DomainCreate/RegisterString/NameCategory/NameThread)
provides:
  - "ResolutionTables — pass-1 handle-resolution tables built by one order-independent scan"
  - "Registered-string resolution keyed by (domain, handle) (ANA-01)"
  - "Category-name resolution keyed by (domain, category), never globally (ANA-02)"
  - "Domain and OS-thread name resolution"
  - "D-14 placeholder policy as pure functions of the raw id"
  - "NvtxMark / NvtxDomain / NvtxThread / NvtxCategory model-surface types (MOD-02)"
  - "NvtxModel accessors: marks(), domains(), threads(), categories(), category_name(), thread_name()"
affects: [02-04, 02-05, ANA-01, ANA-02, MOD-02]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pass 1 is a fold of the whole ordered stream into tables; every accumulation is an idempotent insert or a min/max, so it is order-independent by construction"
    - "Placeholders are pure functions of the raw id — captured strings are never interpolated into a format specifier"
    - "BTreeSet for the observed-id sets so record ordering is deterministic without an explicit sort"
    - "The model retains its ResolutionTables so names with no reconstructed entity still resolve on demand"

key-files:
  created:
    - "integrations/nvtx/analyzer/src/tables.rs"
    - "integrations/nvtx/analyzer/tests/resolution.rs"
  modified:
    - "integrations/nvtx/analyzer/src/span.rs"
    - "integrations/nvtx/analyzer/src/model.rs"
    - "integrations/nvtx/analyzer/src/ranges.rs"
    - "integrations/nvtx/analyzer/src/lib.rs"

key-decisions:
  - "Resolution happens at range *open* time, not at close: pass 1 is already complete when replay starts, so ranges.rs takes a resolved name and never needs the tables"
  - "The model retains its ResolutionTables, making category_name()/thread_name() answer for any id rather than only for ids attached to a reconstructed entity"
  - "Marks are NvtxMark instants, never zero-length NvtxSpans — a timeline consumer must be able to tell 'happened at' from 'lasted zero'"
  - "Domains referenced but never created still get a record, with created = first-seen timestamp; a domain created before capture began is still a real grouping key"
  - "Category 0 never enters the tables or the categories() view — it is NVTX's 'no category' sentinel, an absence rather than an unresolved reference"
  - "Added NvtxCategory beyond the three types the plan named, because the plan's 'categories view keyed (domain, category)' and the MOD-02 must_have both need a record type"

patterns-established:
  - "Two-pass shape completed: pass 1a orders (TimeOrderedCollector), pass 1b learns names (ResolutionTables), pass 2 replays and resolves"
  - "Placeholder helpers live as free functions next to the const labels in tables.rs, so the exact strings have one definition to assert against"

requirements-completed: [ANA-01, ANA-02, MOD-02]

# Metrics
duration: ~20 min
completed: 2026-07-28
---

# Phase 2 Plan 03: Handle Resolution and Model Surface Summary

**A pass-1 scan now learns every NVTX registration in the stream before replay begins, so registered strings resolve per `(domain, handle)`, category names per `(domain, category)`, and anything that never resolves gets a bracketed placeholder that is a pure function of its raw id — with marks, domains, threads, and categories now first-class on the model.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 2 (tables/types/RED, wiring/GREEN)
- **Files created:** 2; modified: 4
- **Tests:** 9 passing (5 from plan 02-02, 4 new)

## Accomplishments

- Built `tables.rs`: `ResolutionTables` folds the whole ordered stream into five lookup tables plus two observed-id sets, in one order-independent scan. Every accumulation is an idempotent insert or a `min`/`max`, so arrival order cannot change the result — which is exactly what makes a forward reference resolve.
- **ANA-01 proven, not just implemented.** `resolve_registered_string` registers handle `0xAB` in domain 1 *and* domain 2 with different strings, and puts both `RegisterString` events **after** the ranges that use them. Both resolve to their own domain's string. A bare-handle key would have collapsed them.
- **ANA-02 proven.** `category_namespaced_by_domain` names category `7` in two domains; a global table would have returned one name for both.
- Implemented the D-14 placeholder policy as `const`s and free functions in one place, with the exact strings asserted in tests: `"default domain"`, `"thread {id}"`, `"<domain 0x{X}>"`, `"<unregistered string 0x{X}>"`, `"<category {n} @ domain 0x{X}>"`. The stability claim is tested directly — two builds of the same stream compare equal across spans, domains, threads, and categories.
- Added the model surface (MOD-02): `NvtxMark`, `NvtxDomain`, `NvtxThread`, `NvtxCategory`, with `marks()`, `domains()`, `threads()`, `categories()`, `category_name()`, and `thread_name()` on `NvtxModel`. `Mark` events now reconstruct into instants rather than falling through the replay match arm.
- Plan 02-02's Start/End matching and all five tolerance tests are untouched and still green.

## Task Commits

Each task committed atomically (DCO sign-off, Conventional Commits):

1. **Task 1: Pass-1 tables + placeholder policy + model-surface types + failing tests (RED)** — `35cc21a` (test)
2. **Task 2: Wire pass-1 into the builder; resolve labels; populate the model surface (GREEN)** — `393845c` (feat)

## Files Created/Modified

- `integrations/nvtx/analyzer/src/tables.rs` *(created)* — `ResolutionTables` (`domain_names`, `domain_lifespans`, `registered_strings` keyed `(domain, handle)`, `category_names` keyed `(domain, category)`, `thread_names`, plus `categories_seen`/`threads_seen` as `BTreeSet`s); `build`/`observe` pass 1; the four `resolve_*` methods; `domain_records`/`thread_records`/`category_records`; the placeholder consts and helpers.
- `integrations/nvtx/analyzer/tests/resolution.rs` *(created)* — the four resolution tests plus local event constructors.
- `integrations/nvtx/analyzer/src/span.rs` — added `NvtxMark`, `NvtxDomain`, `NvtxThread`, `NvtxCategory`.
- `integrations/nvtx/analyzer/src/model.rs` — pass 1b calls `ResolutionTables::build`; pass 2 resolves labels and reconstructs marks; five new accessors; `NvtxModel` gained `marks`/`domains`/`threads`/`categories`/`tables` fields.
- `integrations/nvtx/analyzer/src/ranges.rs` — `OpenStartRange` carries a resolved `name`; `start()` takes it; the interim local `resolve_name` placeholder function was deleted.
- `integrations/nvtx/analyzer/src/lib.rs` — `mod tables`; re-exports the four new types; crate doc now states the two-pass naming contract.

## Decisions Made

- **Resolve at open, not at close.** `ranges.rs` could have taken `&ResolutionTables` and resolved inside `close()`. It does not need to: pass 1 finishes before replay starts, so the name is knowable the moment the `RangeStart` is seen. Resolving there keeps `ranges.rs` ignorant of the tables entirely and keeps `resolve_message` visible at the one call site in `model.rs` where the two passes meet.
- **The model retains its tables.** `category_name(domain, category)` and `thread_name(id)` answer for *any* id, not only ids that ended up on a reconstructed entity. This is what the plan's "unnamed threads render `thread {id}` on demand" requires, and it costs one moved struct.
- **Marks are instants, not zero-length spans.** Both are representable, but a consumer rendering a timeline needs to distinguish "happened at T" from "lasted zero ns starting at T". `unclosed_start_closed_synthetic` (from 02-02) already asserted marks are not spans; that assertion still holds and now means something stronger.
- **Referenced-but-uncreated domains still get records.** `created` falls back to the first timestamp the domain was seen at. A domain created before the capture attached is not an anomaly — it is the normal case for a library already running — and it is still a valid grouping key.
- **Category 0 is filtered at the table, not at the view.** `NameCategory { category: 0 }` is dropped during pass 1 rather than stored and hidden later, so there is exactly one place where the sentinel is understood.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `model.rs` and `ranges.rs` touched outside their listed task**

- **Found during:** Tasks 1 and 2
- **Issue:** Task 1's `<files>` omits `model.rs`, but its RED tests call `marks()`/`domains()`/`threads()`/`category_name()`/`thread_name()` — those symbols have no other home, and without them the tests fail to *compile* rather than fail on assertions, which is not RED. Symmetrically, Task 2's `<files>` lists only `model.rs` and `lib.rs`, but the span name is produced inside `ranges.rs::close()`, so leaving `ranges.rs` alone would have kept every `RangeStart` label at the interim `<unregistered string ...>` placeholder regardless of the tables.
- **Fix:** Added stub accessors to `model.rs` in Task 1 (empty vectors, `None`, empty string) and replaced them with the real implementation in Task 2. Changed `ranges.rs` in Task 2 so `OpenStartRange` carries an already-resolved `name` and `start()` accepts it, deleting the now-dead local `resolve_name`.
- **Files modified:** `integrations/nvtx/analyzer/src/model.rs`, `integrations/nvtx/analyzer/src/ranges.rs`
- **Commits:** `35cc21a` (stubs), `393845c` (implementation)

**2. [Rule 2 - Missing critical functionality] `NvtxCategory` added beyond the three named types**

- **Found during:** Task 1
- **Issue:** Task 1 names three model-surface types (`NvtxMark`, `NvtxDomain`, `NvtxThread`), but Task 2's action requires "a categories view keyed `(domain, category)`" and the plan's `must_haves.truths` requires categories to be "present in the built model". A view needs a record type; `category_name()` alone is a lookup, not a view, and would leave the MOD-02 truth unverifiable.
- **Fix:** Added `NvtxCategory { domain, category, name }` alongside the three named types and a `categories()` accessor. The `(domain, category)` pair *is* the identity, so the type makes the ANA-02 namespacing visible in the model rather than only inside the tables.
- **Files modified:** `integrations/nvtx/analyzer/src/span.rs`, `integrations/nvtx/analyzer/src/model.rs`, `integrations/nvtx/analyzer/src/tables.rs`
- **Commits:** `35cc21a`, `393845c`

**3. [Rule 1 - Bug] `needless_lifetimes` clippy failure in the test helper**

- **Found during:** Task 2 verification
- **Issue:** `fn domain_record<'a>(model: &'a NvtxModel, ..) -> &'a NvtxDomain` tripped `clippy::needless_lifetimes` under `-D warnings`, failing the plan's own acceptance criterion.
- **Fix:** Elided the lifetimes and imported `NvtxDomain` rather than path-qualifying it.
- **Files modified:** `integrations/nvtx/analyzer/tests/resolution.rs`
- **Commit:** `393845c`

### Environment deviation (pre-execution)

**Worktree spawned from the wrong base.** This executor's worktree (`worktree-agent-a00ceff0b7805eb17`) was created at `f31e60b`, a commit on `main`'s history, rather than the specified base `6c46b9e` — so `.planning/`, `CLAUDE.md`, and `integrations/nvtx/` were absent entirely. The branch is in the sanctioned `worktree-agent-*` namespace, the tree was clean, and the branch had zero unique commits (`worktree-agent-… --not nvtx-phase-2 main` empty), so it was aligned to the specified base with `git reset --hard 6c46b9e` — the sanctioned startup branch-check remedy on a per-agent branch. No commits were destroyed and no protected ref was touched. This is the same misconfiguration plan 02-02's executor hit.

---

**Total deviations:** 3 auto-fixed (1× Rule 3, 1× Rule 2, 1× Rule 1) + 1 pre-execution environment correction.
**Impact on plan:** No scope change, no architectural change. Deviation 1 is the mechanical consequence of the plan's per-task file lists; deviation 2 is required to satisfy a `must_haves` truth the plan already asserted; deviation 3 is a lint fix.

## Requirement Status

- **ANA-01** (resolve registered-string handles from the event stream): **COMPLETE.** Keyed `(domain, handle)`; forward references resolve; unregistered handles fall back to a placeholder rather than failing.
- **ANA-02** (resolve domain and category names with `(domain, categoryId)` namespacing): **COMPLETE.** Categories are never keyed globally, proven by a two-domain collision test. Domain names resolve, with domain 0 labelled `"default domain"`.
- **MOD-02** (domains/threads/categories first-class; marks are instants): **COMPLETE** for marks, domains, threads, and categories. Resources remain a later slice (02-05).

## Threat Model Compliance

| Threat ID | Disposition | Implementation |
|-----------|-------------|----------------|
| T-02-04 | mitigate | Every placeholder is `format!` over a raw integer only (`{domain:X}`, `{handle:X}`, `{category}`, `{thread_id}`). No captured string is ever interpolated into a placeholder, so a stream cannot inject a label that mimics a real one. Unresolved names stay bracketed (`<...>`); the two *legitimately* unnamed cases (`"default domain"`, `"thread {id}"`) are deliberately unbracketed so the two categories are visually distinguishable. |
| T-02-01 | mitigate | Resolution never `unwrap`s: every lookup is `get(..).cloned().unwrap_or_else(placeholder)`. A missing registration is a label, not an error. Verified: `grep -rn "unwrap()\|expect(\|panic!" src` → 0 matches across all 6 files. |
| T-02-05 | accept | Table growth is bounded by the finite captured session, as planned. Unchanged. |

## Known Stubs

Intentional and scoped by later plans in this phase:

| Stub | File | Resolution |
|------|------|------------|
| `RangePush`/`RangePop` contribute only their domain and thread id to pass 1; they build no spans | `src/model.rs` | Plan 02-04 |
| `NvtxMark::thread_id` is always `None` — `nvtxDomainMarkEx` carries no thread id in the vocabulary | `src/model.rs` | By design (field kept per D-03) |
| `ResourceCreate` contributes only its domain; resource records and `identifier_type` labels absent | `src/tables.rs` | Plan 02-05 |
| `SpanKind::PushPop` / `SpanKind::Resource` still unconstructed; `NvtxSpan::parent` always `None` | `src/span.rs` | Plans 02-04 / 02-05 |

None prevent this plan's goal (per-domain handle resolution with stable placeholders, plus the mark/domain/thread/category surface) from being achieved.

## Verification Evidence

- `cargo test -p nvtx-analyzer` — ok, **9 passed, 0 failed** (5 reconstruction from 02-02 + 4 resolution: `resolve_registered_string`, `category_namespaced_by_domain`, `placeholder_stable`, `model_surface_present`).
- RED confirmed before implementation: all 4 new tests failed on assertions (`assertion left == right failed: registered strings resolve per (domain, handle)…`, `…the Mark became an instant`, `no domain record for 0x2A`), while the 5 pre-existing tests stayed green.
- `cargo clippy -p nvtx-analyzer --all-targets --all-features -- -D warnings` — clean.
- `cargo fmt --all -- --check` — clean (exit 0).
- `cargo doc -p nvtx-analyzer --no-deps` — no warnings (intra-doc links resolve).
- `grep -q "resolve_message" src/model.rs` → present (labels resolved via the tables in pass 2).
- `grep -rc "quent_analyzer\|quent_model\|RtFsm" src` → **0** for all 6 files (framework-free contract holds).
- `grep -q "default domain"` / `"unregistered string"` in `src/tables.rs` → both present; `grep -q "struct NvtxMark" src/span.rs` → present.
- `grep -rn "unwrap()\|expect(\|panic!" src` → no matches (`expect` appears only in test code).
- No file deletions in either commit (`git diff --diff-filter=D HEAD~2 HEAD` empty); no untracked files left behind.

## TDD Gate Compliance

Gate sequence satisfied: `test(02-03)` RED commit `35cc21a` precedes `feat(02-03)` GREEN commit `393845c`. No REFACTOR commit was needed.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 02-04 (push/pop per-thread stacks) has everything it needs: `thread_id` is already collected into `threads_seen` during pass 1 and resolves via `thread_name()`, `SpanKind::PushPop` and `NvtxSpan::parent` are defined, and `resolve_message` is the same call a push will make.
- Plan 02-05 (resources) slots into `tables.rs` the same way: `ResourceCreate` already registers its domain, and `resolve_message` handles the `message` field's `RegisteredHandle` case unchanged.
- No blockers introduced.

## Self-Check: PASSED

- Files verified present: `integrations/nvtx/analyzer/src/{tables.rs,span.rs,model.rs,ranges.rs,lib.rs}`, `integrations/nvtx/analyzer/tests/resolution.rs` — all FOUND.
- Commits verified present: `35cc21a` (FOUND), `393845c` (FOUND).

---
*Phase: 02-nvtx-model-tolerant-analyzer*
*Completed: 2026-07-28*
