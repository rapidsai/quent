# Phase 1: Capture Foundation - Context

**Gathered:** 2026-07-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Capture NVTX activity from a running application into Quent's standard event
pipeline as **raw, verbatim events** — driven by a deterministic in-repo NVTX
test app under GPU-less CI.

In scope: single-consumer injection, the raw NVTX event vocabulary, the
EventSender bridge, a deterministic test app, and a subprocess test harness.

Out of scope (later phases): handle→name resolution and tolerant reconstruction
(Phase 2), HTTP/UI (Phase 3), the fan-out mediator / coexistence with other
consumers (Phase 4), real-GPU-workload validation and load/stress proof
(Phase 5), payload **decode**/render (v2).
</domain>

<decisions>
## Implementation Decisions

### PR #87 Adoption & Crate Layout
- **D-01: Reference-only rebuild.** Treat PR #87 (Johan Peltenburg's WIP
  `feat: add Rust-based NVTX injection library`, branch `nvtx`, rapidsai/quent)
  as a **design reference** and re-implement Phase 1 from scratch in our own
  commits. Do NOT rebase or adopt its branch history. PR #87's design was
  reviewed and validated as sound: complete `NvtxEvent` vocabulary, bindgen +
  strong-symbol injection, `Event::new_now` EventSender bridge.
- **D-02: Crate layout — adopt `integrations/nvtx/` tree.** New top-level
  `integrations/nvtx/{events,injection,instrumentation}` (NOT `crates/` or
  `domains/`), signalling separable integration crates for later upstreaming.
  Package names follow PR #87: `quent-nvtx-events`, `quent-nvtx-injection`,
  `quent-nvtx` (the instrumentation/EventSender bridge). Register in workspace
  `members` (and `default-members` unless a zero-cost exclusion applies).
- **D-03: Separability is a hard constraint.** The injection crate must not
  depend on Quent internals — prefer a sink-agnostic injection (generic
  `Fn(NvtxEvent)` hook, as PR #87's `install_hook`) with Quent's `EventSender`
  wired in as one sink by the `quent-nvtx` bridge.

### NVTX Bindings & Injection Mechanism
- **D-04: macOS out of scope for Phase 1 — Linux-only.** Drop PR #87's macOS
  codepaths (`-force_load`, `pthread_threadid_np`) from the rebuild; keep the
  Windows/32-bit `compile_error!` guards. Revisit macOS later. Consistent with
  "(and possibly macOS)" being optional in PROJECT constraints. Linux is the
  real deployment platform for the target GPU libraries.
- **D-05: Binding approach follows PR #87 in principle** — bindgen against NVTX
  C headers + a `cc`-compiled strong `InitializeInjectionNvtx2_fnptr` symbol
  overriding NVTX's weak definition (whole-archive link). Header *sourcing* and
  *attach mechanism* are open research questions (see below).

### Hot-Path Hand-off & Overflow
- **D-06: Bounded + non-blocking is a hard requirement (CAP-05).** The capture
  path must never block the app thread on locks/I-O and must not allocate
  unboundedly. NOTE: Quent's `EventSender` is a **tokio unbounded mpsc**
  (`crates/instrumentation/src/observer.rs:59`) — non-blocking but unbounded,
  so a bounded stage/overflow policy is required in front of it to satisfy
  "no unbounded allocation". The specific mechanism is deferred to research.
- **D-07: Overflow-on-overload favors drop-and-count over blocking.** Under
  sustained overload the design should drop events (and account for the drops)
  rather than park the producer — standard profiler behavior. Exact policy
  (ring buffer, per-thread buffers, etc.) → research.
- **D-08: CAP-05 proof is design-level for Phase 1.** Establish the
  bounded/hand-off design plus a basic correctness test (buffer fills →
  drop-counted, not blocked). Full high-frequency load/stress validation moves
  to **Phase 5**. This intentionally softens Phase 1 success-criterion 5
  ("stamp-and-hand-off demonstrated under high-frequency emission").

### Test App & Harness Shape
- **D-09: Subprocess + deterministic standalone app.** A dedicated test-app
  binary emits a fixed NVTX script; integration tests spawn it as a
  **subprocess** and assert on captured output. Satisfies VAL-01 (deterministic
  app) and VAL-02 (subprocess — NVTX init is process-global one-shot, so the
  injection can only be installed once per process). Mirrors the existing
  `domains/query_engine/tests/fixed/` deterministic-emitter pattern. Exact
  binary/harness structure → planning.
- **D-10: Assert via the real ndjson exporter to a file.** The test app captures
  through Quent's actual ndjson exporter to a file; the parent harness reads it
  back and asserts on the event stream. Exercises the real CAP-04 pipeline
  (`EventSender` → exporter) end-to-end and stays human-readable/debuggable.
- **D-11: Full event-kind coverage, single AND multi-threaded.** The
  deterministic app exercises every event kind in success-criterion 1
  (push/pop, RangeStart/End, marks, domain create/destroy, registered strings,
  category naming, thread naming, resource create/destroy) AND spawns multiple
  threads so cross-thread RangeStart/End and per-thread naming are exercised.
  The multi-threaded coverage de-risks Phase 2 reconstruction.

### Open Questions for Research (raised during discussion)
- **R-01: Payload-extension capture scope (CAP-03).** Assess how much
  payload-extension surface real targets (libcudf, cuCascade) actually emit,
  and the cost of wiring the NVTX payload-extension callbacks, before deciding
  whether verbatim payload **capture** stays in Phase 1 or is deferred. PR #87
  models the payload event types but does not emit them. If deferred,
  REQUIREMENTS.md/ROADMAP.md CAP-03 mapping must be updated.
- **R-02: NVTX header sourcing.** Compare git-pin on NVIDIA/NVTX (as PR #87, via
  `nvtx-sys`; `deny.toml` already allow-lists the git source) vs vendored
  in-repo headers vs a crates.io `nvtx-sys`, judged on reproducibility,
  offline/GPU-less CI, and cargo-deny policy. Recommend one.
- **R-03: Attach mechanism + build order.** Confirm how nsys/AON and libcudf
  attach; whether Phase 1 delivers the runtime `NVTX_INJECTION64_PATH` cdylib
  path, the link-time strong-symbol path, or both; and a Rust cdylib's init
  timing / symbol-export needs. Runtime cdylib is what uninstrumented GPU libs
  require (they can't be relinked), so proving it early de-risks Phase 5.
- **R-04: Hot-path overflow design.** Survey how nsys and other injection
  profilers handle overload (ring buffers, per-thread buffers, drop policies)
  and recommend a concrete bounded, non-blocking design honoring D-06/D-07.

### Locked Research Resolutions (post-research, 2026-07-13)
Resolutions of R-01..R-04 after the Phase 1 research pass (01-RESEARCH.md) and a
user planning checkpoint. These OVERRIDE the open questions above and are locked
user decisions for planning.

- **D-12 (R-01 RESOLVED): Defer payload extension; capture core union.** Phase 1
  captures the CORE `nvtxEventAttributes` payload **union** verbatim (raw value,
  undecoded). The NVTX payload-**extension** module (schema/enum registration,
  structured binary blobs; `nvToolsExtPayload.h` / `nvtxExtImplPayload_v1.h`) is
  DEFERRED to a later phase. Evidence: libcudf (primary, verifiable target)
  emits **zero** payload-extension events — `cpp/include/cudf/detail/nvtx/ranges.hpp`
  attaches only message + domain, and `nvtxPayload` = 0 hits across rapidsai/cudf.
  cuCascade usage is unverified (non-public) — record as a revisit trigger.
  **The plan MUST include a task updating REQUIREMENTS.md CAP-03 and ROADMAP
  §Phase 1 success-criterion 2** to reflect the narrowed scope (core union
  captured now; payload-extension capture deferred).

- **D-13 (R-02 RESOLVED): Git-dep on NVIDIA/NVTX for headers.** Adopt PR #87's
  approach: `nvidia-nvtx = { git = "https://github.com/NVIDIA/NVTX", ... }` as the
  header source (a dev/optional dependency; `build.rs` locates the `nvtx-sys`
  crate via `cargo_metadata` and derives `<nvtx-repo>/c/include`). Add
  `"https://github.com/NVIDIA/NVTX"` to `deny.toml` `allow-git` (it is a TOML
  list, currently `[]`). **Refinement over PR #87: pin a `rev = "<sha>"`, NOT
  `branch = "release-v3"`**, to keep the dependency reproducible. **Pinned rev:
  `7d113f290f89eeeae9c957011c497101f3948d9e` (NVTX tag `v3.5.0`)** — do NOT pin
  the `release-v3` branch HEAD (currently a dependabot CI-bump commit). (the base
  `allow-git = []` is a generic supply-chain default — no project-specific
  rationale is documented anywhere, so relaxing it for NVIDIA's official repo is
  an accepted trade-off, but pinning a rev preserves the hygiene it guards).

- **D-14 (Bindgen/CI RESOLVED): Commit `bindings.rs`; feature-gated regen;
  document in build README.** Do NOT run bindgen on every build (avoids adding
  `libclang` to `pixi.toml` — `pixi` currently lacks it — and keeps CI hermetic).
  Instead: run bindgen once, **commit the generated `src/bindings.rs`**, and gate
  regeneration behind a cargo feature (e.g. `regenerate-bindings`) checked in
  `build.rs` that overwrites the committed file. Normal/CI builds `include!` the
  committed file — no libclang, no network for header codegen. **The plan MUST
  include a task documenting the regen command (and when to run it — on an NVTX
  `rev` bump) in the build-instructions README.**

- **D-15 (R-03 adopted from research): Runtime cdylib attach path first.** Phase 1
  ships the `NVTX_INJECTION64_PATH` runtime-cdylib injection (what uninstrumented
  GPU libs require; de-risks Phase 5). The link-time strong-symbol path is
  secondary / feature-gated. Keep PR #87's `cc`-compiled `c/symbol.c` +
  whole-archive strong-symbol override (consistent with D-05).

- **D-16 (R-04 adopted from research): Bounded, non-blocking hot-path stage in the
  bridge.** Front Quent's unbounded `EventSender` mpsc with a bounded lock-free
  ring (e.g. `crossbeam-queue::ArrayQueue`) + drop-and-count + a drain thread,
  placed in the `quent-nvtx` bridge so the injection crate stays sink-agnostic
  (D-03). Satisfies CAP-05's no-unbounded-allocation clause; full load/stress
  proof remains Phase 5 (D-08).

### Delivery / PR Grouping (decided 2026-07-13)
- **D-17: Single PR for Phase 1, reviewed in 3 crate-seam groups.** Ship the whole
  phase as ONE PR (not stacked PRs). Because waves execute in dependency order,
  the commit history is already segmented into the review groups:
  Group 1 = 01-01 (`quent-nvtx-events` + CAP-03 re-scope);
  Group 2 = 01-02 (`quent-nvtx-injection` cdylib/FFI — the separable, upstreamable
  crate); Group 3 = 01-03 + 01-04 (`quent-nvtx` bridge + test app + harness +
  README). `/gsd:ship` should reflect these three groups in the PR description.

### Claude's Discretion
- Hot-path buffer **placement** (injection crate vs `quent-nvtx` bridge) is left
  to researcher/planner, constrained only by D-03 (keep injection separable).
- Exact test-app/harness file structure (D-09) is left to planning.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Planning Source-of-Truth
- `.planning/ROADMAP.md` §"Phase 1: Capture Foundation" — goal + 5 success criteria
- `.planning/REQUIREMENTS.md` — CAP-01..05, VAL-01, VAL-02 (Phase 1 requirements);
  note the "Out of Scope" table (capture-time handle resolution, Windows, etc.)
- `.planning/PROJECT.md` — locked constraints (Rust-first, separability,
  Linux/macOS 64-bit, one injection slot per process)

### Design Reference (external)
- PR #87 `rapidsai/quent`, branch `nvtx` — `feat: add Rust-based NVTX injection
  library` (Johan Peltenburg, OPEN/WIP). Reference for: `NvtxEvent` vocabulary
  (`integrations/nvtx/events/src/{lib,attributes,payload}.rs`), injection
  mechanism (`integrations/nvtx/injection/src/{init,callbacks,convert}.rs`,
  `build.rs`, `c/symbol.c`, `wrapper.h`), and the EventSender bridge
  (`integrations/nvtx/instrumentation/src/lib.rs`). Local diff snapshot was
  reviewed during discussion. **Reference-only — do not adopt its commits.**

### Existing Code To Mirror / Integrate
- `crates/instrumentation/src/observer.rs` — `EventSender` (unbounded tokio
  mpsc; `send` at line 59) the bridge forwards through; relevant to D-06
- `crates/events/src/lib.rs` — `Event<T>` envelope + `Event::new_now` (capture
  timestamp, satisfies CAP-04's timestamp)
- `crates/exporter/ndjson/` — ndjson exporter used by the test harness (D-10)
- `domains/query_engine/tests/fixed/src/main.rs` — deterministic-emitter binary
  pattern to mirror for the NVTX test app (D-09); note it activates
  `quent-time/__test-clock-override` and is excluded from `default-members`
- `.planning/codebase/STRUCTURE.md` §"Where to Add New Code" — workspace
  registration conventions

### NVTX Domain Background
- `.planning/research/*.md` (ARCHITECTURE, FEATURES, PITFALLS, STACK, SUMMARY)
  — NVTX ingestion domain research from roadmap creation
- NVIDIA/NVTX injection sample: https://github.com/NVIDIA/NVTX/tree/dev/tools/sample-injection
  (referenced in parent issue #76)
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Event<T>` / `Event::new_now` (`crates/events`): supplies the capture-time
  timestamp for CAP-04 — the bridge wraps each `NvtxEvent` in an `Event`.
- `EventSender` (`crates/instrumentation/src/observer.rs`): the standard sink
  the bridge targets; but it is **unbounded**, so it does not by itself satisfy
  CAP-05's no-unbounded-allocation clause (see D-06).
- ndjson exporter (`crates/exporter/ndjson/`): the harness capture format (D-10).
- `domains/query_engine/tests/fixed/`: template for a deterministic emitter
  binary run out-of-process (D-09).

### Established Patterns
- Application-agnostic crates are `quent-*` and registered in root `Cargo.toml`
  `members`/`default-members`; shared deps pinned in `[workspace.dependencies]`.
- SPDX headers on every source file (`SPDX-License-Identifier: Apache-2.0`).
- Crates activating `quent-time/__test-clock-override` are excluded from
  `default-members` (zero-cost guarantee) — relevant if the test app uses it.
- `cargo clippy -D warnings` and `cargo fmt --check` are CI gates; FFI code
  needs careful `unsafe`/allow scoping (see PR #87's `callbacks.rs`).

### Integration Points
- `quent-nvtx` bridge → `EventSender` (one sink; injection stays sink-agnostic).
- Test harness → ndjson exporter output file → assertions.
- Workspace root `Cargo.toml` + `deny.toml` (git-source allow-list) if an NVTX
  git dependency is chosen (R-02).
</code_context>

<specifics>
## Specific Ideas

- PR #87 is the concrete "I want it like X" reference for the event vocabulary
  and injection mechanism — the rebuild should track its `NvtxEvent` shape
  (verbatim handle IDs, `NvtxMessage::{String,RegisteredHandle}`) closely, since
  Phase 2's analyzer will consume that vocabulary.
- Capture stays strictly verbatim: handles are captured as raw `u64`, names are
  NOT resolved at capture time (locked; resolution is Phase 2).
</specifics>

<deferred>
## Deferred Ideas

- **macOS support for the injection library** — carried out of Phase 1 (D-04);
  revisit in a later phase if a macOS target materializes.
- **High-frequency load/stress validation of the hot path** — moved to Phase 5
  (D-08); Phase 1 proves the design only.
- **Payload-extension *decode* and rendering** — v2 (PAY-01/PAY-02), unchanged.
  (Payload *capture* itself is R-01, still under research for Phase 1.)

None of the above are scope creep — they are explicit carry-forwards.
</deferred>

---

*Phase: 1-capture-foundation*
*Context gathered: 2026-07-13*
