# Phase 1: Capture Foundation - Research

**Researched:** 2026-07-13
**Domain:** NVTX v3 injection/consumer FFI in Rust + hot-path capture into Quent's `EventSender` pipeline, driven by a deterministic in-repo test app under GPU-less CI
**Confidence:** HIGH (attach mechanism and FFI vocabulary verified against NVIDIA/NVTX primary source; existing Quent code read directly; open questions R-01..R-04 resolved with prescriptive recommendations, three of which need user confirmation)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01: Reference-only rebuild.** Treat PR #87 as a *design reference*; re-implement Phase 1 from scratch in our own commits. Do NOT rebase or adopt its branch history. Its design (complete `NvtxEvent` vocabulary, bindgen + strong-symbol injection, `Event::new_now` bridge) was reviewed as sound.
- **D-02: Crate layout — adopt `integrations/nvtx/` tree.** New top-level `integrations/nvtx/{events,injection,instrumentation}` (NOT `crates/` or `domains/`). Package names: `quent-nvtx-events`, `quent-nvtx-injection`, `quent-nvtx` (the instrumentation/EventSender bridge). Register in workspace `members` (and `default-members` unless a zero-cost exclusion applies).
- **D-03: Separability is a hard constraint.** The injection crate must not depend on Quent internals — prefer a sink-agnostic injection (generic `Fn(NvtxEvent)` hook, as PR #87's `install_hook`) with Quent's `EventSender` wired in as one sink by the `quent-nvtx` bridge.
- **D-04: macOS out of scope for Phase 1 — Linux-only.** Drop PR #87's macOS codepaths (`-force_load`, `pthread_threadid_np`); keep the Windows/32-bit `compile_error!` guards.
- **D-05: Binding approach follows PR #87 in principle** — bindgen against NVTX C headers + a `cc`-compiled strong `InitializeInjectionNvtx2_fnptr` symbol overriding NVTX's weak definition. Header *sourcing* and *attach mechanism* are open research questions (R-02, R-03).
- **D-06: Bounded + non-blocking is a hard requirement (CAP-05).** The capture path must never block the app thread on locks/I-O and must not allocate unboundedly. Quent's `EventSender` is a **tokio unbounded mpsc** (`crates/instrumentation/src/observer.rs:59`), so a bounded stage/overflow policy is required in front of it.
- **D-07: Overflow-on-overload favors drop-and-count over blocking.**
- **D-08: CAP-05 proof is design-level for Phase 1.** Establish the bounded/hand-off design plus a basic correctness test (buffer fills → drop-counted, not blocked). Full high-frequency load/stress validation moves to Phase 5.
- **D-09: Subprocess + deterministic standalone app.** A dedicated test-app binary emits a fixed NVTX script; integration tests spawn it as a **subprocess** and assert on captured output (VAL-01, VAL-02). Mirror `domains/query_engine/tests/fixed/`.
- **D-10: Assert via the real ndjson exporter to a file.** Test app captures through Quent's actual ndjson exporter to a file; the parent harness reads it back and asserts.
- **D-11: Full event-kind coverage, single AND multi-threaded.** The deterministic app exercises every event kind AND spawns multiple threads (cross-thread RangeStart/End, per-thread naming) to de-risk Phase 2.

### Claude's Discretion
- Hot-path buffer **placement** (injection crate vs `quent-nvtx` bridge) — constrained only by D-03 (keep injection separable).
- Exact test-app/harness file structure (D-09).

### Deferred Ideas (OUT OF SCOPE)
- **macOS support** for the injection library (D-04) — later phase.
- **High-frequency load/stress validation** of the hot path (D-08) — Phase 5.
- **Payload-extension *decode* and rendering** — v2 (PAY-01/PAY-02). (Payload *capture* itself is R-01, resolved below.)
- Handle→name resolution and tolerant reconstruction (Phase 2), HTTP/UI (Phase 3), fan-out mediator (Phase 4), real-GPU validation (Phase 5).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CAP-01 | App emitting NVTX v3 captured without app code changes (attach via `NVTX_INJECTION64_PATH` or link-time) | R-03: deliver the runtime cdylib path (`NVTX_INJECTION64_PATH`) as primary; strong-symbol link-time path as secondary/test. Verified attach handshake against NVTX `sample-injection`. |
| CAP-02 | All core NVTX call types captured verbatim as raw events | Standard Stack + NVTX API Surface: CORE + CORE2 callback tables cover push/pop, RangeStart/End, marks, domain create/destroy, registered strings, category naming, thread naming, resource create/destroy. Mirror PR #87's `NvtxEvent` vocabulary. |
| CAP-03 | Payload-extension events captured & preserved (decode deferred to v2) | **R-01 (needs user confirmation):** distinguish the CORE `nvtxEventAttributes_t.payload` union (free to capture) from the payload *extension* module (`nvtxExtInit`/schemas/enums/binary — separate module, zero Rust tooling, unproven libcudf adoption). Recommend capturing the union now; deferring the extension module. Impacts success-criterion 2 and CAP-03 mapping. |
| CAP-04 | Captured events timestamped at capture, flow into standard pipeline (`EventSender` → exporters) | Verified: `Event::new_now` (`crates/events/src/lib.rs:38`) stamps `timestamp()` at construction; `EventSender::send`/`emit` (`observer.rs:59,69`) is the sink. Bridge wraps each `NvtxEvent` in `Event<T>`. |
| CAP-05 | Capture never blocks app threads on locks/I-O/unbounded allocation | R-04: bounded lock-free ring + drop-and-count in front of the unbounded `EventSender`; drain thread forwards. Design + basic correctness test only for Phase 1 (D-08). |
| VAL-01 | Deterministic in-repo NVTX test app exercises pipeline in CI without GPU | D-09/D-11 + Validation Architecture: standalone binary emitting a fixed NVTX script through the real capture path; no GPU. Mirror `tests/fixed/`. |
| VAL-02 | Injection/fan-out integration tests run in subprocesses (NVTX init is process-global one-shot) | Pitfall 11 + Validation Architecture: subprocess harness spawns the test-app binary, asserts on the ndjson file. In-process `cargo test` cannot re-install the one-shot hook. |
</phase_requirements>

## Summary

This phase is a **thin-FFI + hot-path-discipline problem**, not a framework problem. Quent already owns everything below `EventSender::emit`; Phase 1 adds one new *source* at the top of that spine. The three new crates (`quent-nvtx-events`, `quent-nvtx-injection`, `quent-nvtx`) plus a deterministic test-app binary and a subprocess integration test are the whole deliverable. The domain research (`.planning/research/*.md`, 2026-07-08, HIGH confidence, verified against NVTX `release-v3` primary source) already establishes the toolchain and pitfalls; this document resolves the four Phase-1-specific open questions and maps every requirement to a provable validation.

The attach mechanism (R-03) is confirmed directly against NVIDIA's own `tools/sample-injection`: NVTX loads a dynamic library named by `NVTX_INJECTION64_PATH` and calls its exported `int InitializeInjectionNvtx2(NvtxGetExportTableFunc_t)` lazily at the first NVTX call, which then fills per-module callback tables (`NVTX_CB_MODULE_CORE`, `CORE2`) using `NVTX_CBID_*` indices. This runtime-cdylib path is exactly what uninstrumented GPU libraries require (they cannot be relinked), so Phase 1 should deliver and prove it first — the subprocess harness sets `NVTX_INJECTION64_PATH` to the built `quent-nvtx-injection` cdylib, which de-risks Phase 5.

**Primary recommendation:** Build `quent-nvtx-events` (verbatim serde vocabulary mirroring PR #87), then `quent-nvtx-injection` as a **cdylib** exporting `InitializeInjectionNvtx2` with a sink-agnostic `Fn(NvtxEvent)` hook (bindgen over **vendored, pinned** NVTX headers — not a git dep), then the `quent-nvtx` bridge that puts a **bounded lock-free ring + drop-counter + drain thread** in front of the unbounded `EventSender`. Drive it all from a deterministic multi-threaded test-app binary asserted via the real ndjson exporter in a subprocess integration test. Confirm the three flagged scope recommendations (R-01 payload deferral, R-02 vendored headers, R-04 ring crate) with the user before locking.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Export `InitializeInjectionNvtx2` / own the injection slot | Injection cdylib (`quent-nvtx-injection`) | — | The C-ABI entry point NVTX calls; must be a `cdylib` with an unmangled strong symbol. Sink-agnostic per D-03. |
| bindgen FFI over NVTX headers | Injection crate `build.rs` + vendored headers | — | Headers are macro-heavy/versioned; hand-writing is untenable. Kept inside the injection crate (separable unit). |
| Callback args → owned `NvtxEvent` (stamp + copy strings) | Injection crate hot path | — | Runs on the app thread; must copy caller-owned `const char*` before returning (Pitfall 4). Verbatim, no interpretation. |
| Bounded buffering + drop-and-count overflow policy | `quent-nvtx` bridge | Injection crate (discretion) | Recommend the bridge (owns the `EventSender` hand-off); keeps the injection crate a pure `Fn(NvtxEvent)` hook per D-03. |
| Drain buffer → `EventSender::send` | `quent-nvtx` bridge drain thread | — | Off the app thread; the only crate that touches Quent internals. |
| Wrap `NvtxEvent` in `Event<T>` with capture timestamp | `quent-nvtx` bridge (`Event::new_now`) | — | Reuses existing Quent timestamp path (CAP-04). |
| Deterministic NVTX emission | Test-app binary (`integrations/nvtx/.../tests` or `examples/nvtx`) | — | Application tier; the "instrumented app" role. No GPU. |
| Subprocess spawn + ndjson assertion | Integration test (parent process) | — | Test-harness tier; required because NVTX init is process-global one-shot (VAL-02). |
| Handle→name resolution, span pairing, tolerance | **NONE — Phase 2** | — | Explicitly out of scope; capture stays verbatim (raw `u64` handles). |

## Resolved Open Questions (R-01..R-04)

### R-01 — Payload-extension capture scope (CAP-03) — **RECOMMENDATION NEEDS USER CONFIRMATION**

There are **two distinct things** both loosely called "payload," and conflating them is the trap:

1. **The CORE `nvtxEventAttributes_t.payload` union** — a single inline value (`u64`/`i64`/`f64`/pointer/size) plus `payloadType`, carried on ordinary marks/ranges through the CORE/CORE2 callback tables. This is **free to capture** — it is already a field in the attribute struct the CORE callbacks receive. `[CITED: nvidia.github.io/NVTX — "Optional extra fields... a category, a color, and a payload value"]`
2. **The NVTX payload *extension*** (`nvToolsExtPayload.h`: `nvtxPayloadSchemaRegister`, `nvtxPayloadEnumRegister`, binary `nvtxPayloadData_t`) — structured/binary blobs described by registered schemas. This is delivered through a **separate extension module** (`nvtxExtInit` / `NvtxExtModuleInfo`), **not** the CORE callback table, and has **no existing Rust tooling**. `[VERIFIED: NVIDIA/NVTX release-v3 c/include/nvtx3/nvtxDetail/nvtxExtInit.h, nvtxExtPayloadTypeInfo.h — confirmed present via gh api]`

**Adoption evidence:** libcudf's NVTX usage is via `nvtx3.hpp` standard ranges/domains; no evidence surfaced that libcudf currently emits payload-*extension* schemas broadly. `[ASSUMED — search inconclusive; nvtx3.hpp confirmed as libcudf's path, payload-extension adoption unproven]` PR #87 itself modeled the payload-extension types but **did not emit them** ("Phase 5 — not yet emitted"). `[CITED: PROJECT.md, CONTEXT.md]`

**Recommendation:**
- **Capture the CORE `payload` union verbatim in Phase 1** (cheap, already in the attribute struct) — this legitimately satisfies "payload preserved in the event stream" for the common case.
- **Defer the payload-*extension* module** (schema/enum registration, binary `nvtxPayloadData_t`) out of Phase 1. It is a separate ext-module handshake with zero Rust tooling and unproven target adoption — high cost, low Phase-1 value, and its *decode* is already v2 (PAY-01/PAY-02).
- Keep the payload-extension **event *types*** defined in `quent-nvtx-events` (verbatim vocabulary, as PR #87 does) so the stream can carry them later without a vocabulary change — but do not wire the ext-module callbacks.

**Consequence to flag:** This narrows success-criterion 2 ("schema registration, enum registration, binary payload data appear verbatim"). If accepted, **REQUIREMENTS.md CAP-03 and ROADMAP.md Phase-1 success-criterion 2 must be updated** to move payload-*extension* capture to a later phase (natural home: alongside PAY-01 decode, or a dedicated payload-capture phase). Confirm with the user before locking.

### R-02 — NVTX header sourcing — **RECOMMENDATION NEEDS USER CONFIRMATION**

**CORRECTION to a CONTEXT.md premise:** CONTEXT.md R-02 states "`deny.toml` already allow-lists the git source." It does **not** — `deny.toml:57` is `allow-git = []` (empty) and `unknown-git = "deny"` (`deny.toml:55`). `[VERIFIED: deny.toml read directly]` A git dependency on NVIDIA/NVTX would therefore **fail `cargo deny check`** (a CI gate) until `allow-git` is amended. This changes the tradeoff materially.

| Option | Reproducibility | Offline / GPU-less CI | cargo-deny impact | Separability |
|--------|-----------------|----------------------|-------------------|--------------|
| **git-pin on NVIDIA/NVTX via `nvtx-sys`** (PR #87) | Good (if pinned to a commit SHA, not the moving `release-v3` branch) | Requires network to fetch git dep on clean CI | **Requires editing `deny.toml` `allow-git`** — currently empty | Downstream inherits the git dep |
| **Vendored in-repo headers** (copy `c/include/nvtx3/**` at a pinned NVTX commit) | Excellent (bytes in-repo) | Fully offline | **No deny.toml change** (no external source) | Clean — headers travel with the crate |
| **crates.io `nvtx-sys`** | Good | Needs registry (already allow-listed) | OK | It's a *producer* -sys layer; injection/consumer headers are what we need — must confirm they ship the injection headers |

**Recommendation: vendor the NVTX C headers in-repo** (`integrations/nvtx/injection/vendor/nvtx3/…`), copied at a documented, pinned NVTX commit, and run bindgen against them at build time. Rationale: (a) fully reproducible and offline — best for GPU-less CI; (b) **no `deny.toml` change** (avoids widening the git allowlist for the whole workspace); (c) maximally separable/upstreamable (Apache-2.0 headers travel with the crate); (d) the header set is small and changes rarely — pin the commit, document it, bump deliberately.

**Optional CI hardening:** for a fully hermetic build with no `libclang` in CI, run `bindgen-cli` once and **check in the generated `bindings.rs`**, gating regeneration behind a `regen-bindings` feature. `libclang` is present on this dev machine (`/usr/lib/llvm-18/lib/libclang.so.1`) but **`pixi.toml` does not provision `clang`/`libclang`** — it only lists `cxx-compiler` (`pixi.toml:14`). If bindgen runs in CI, add `clang` to `pixi.toml` (the single most common bindgen CI failure per STACK.md); checking in `bindings.rs` sidesteps this entirely. Confirm the chosen path with the user.

### R-03 — Attach mechanism + build order — **RESOLVED (verified against primary source)**

Confirmed against NVIDIA's `tools/sample-injection/README.md` and `Source/NvtxSampleInjection.cpp`: `[VERIFIED: NVIDIA/NVTX dev branch tools/sample-injection — read via gh api]`
- NVTX loads the dynamic library named by **`NVTX_INJECTION64_PATH`** prior to the first NVTX call, then calls **`int InitializeInjectionNvtx2(NvtxGetExportTableFunc_t)`**.
- That function retrieves per-module callback tables via the export-table func and fills them with callback references, **using the corresponding `NVTX_CBID_*` indices per module** (`NVTX_CB_MODULE_CORE`, `NVTX_CB_MODULE_CORE2`).
- Init is **lazy and one-shot** (first NVTX touch) — no static-initializer work is needed on our side; do minimal idempotent work guarded by a `std::sync::OnceLock`.

**Recommendation — deliver BOTH paths, prioritize the runtime cdylib:**
1. **Primary: runtime cdylib via `NVTX_INJECTION64_PATH`.** `quent-nvtx-injection` is a `crate-type = ["cdylib"]` exporting `#[unsafe(no_mangle)] pub extern "C" fn InitializeInjectionNvtx2(...) -> i32`. This is the path uninstrumented GPU libs require (they can't be relinked). The subprocess harness sets `NVTX_INJECTION64_PATH` to the built `.so` — **no link games, proves the real GPU-lib path, de-risks Phase 5.** Make this the test-app's default attach mode.
2. **Secondary: link-time strong-symbol override.** A `#[used] #[no_mangle] pub static InitializeInjectionNvtx2_fnptr` (or PR #87's `cc` C shim) overriding NVTX's `__attribute__((weak))` default, for the statically-linked test scenario. Gate the `cc` shim behind a `static-injection` feature so `cc`/libclang don't tax the default build (STACK.md). **Validate empirically** whether the pure-Rust strong static beats the weak C symbol at link time; fall back to the `cc` shim if not (SUMMARY.md gap).

**Rust cdylib init timing / symbol-export needs:** the crate must be `cdylib`; the symbol must be exactly `InitializeInjectionNvtx2` (no mangling, `extern "C"`); returning `1` signals success. Because init is lazy, there is no static-init-order hazard on our side *as long as* the callback tables and the `OnceLock` sink are constructed inside `InitializeInjectionNvtx2` itself, not in a `#[ctor]`. `catch_unwind` at every callback boundary (a Rust panic across the C ABI is UB — Pitfall/robustness).

### R-04 — Hot-path overflow design — **RESOLVED (design recommendation)**

`EventSender` is a **tokio unbounded mpsc** (`observer.rs:16,59,151`) — non-blocking but unbounded, so feeding it directly violates CAP-05's "no unbounded allocation" clause under sustained overload. A bounded stage in front is required (D-06). Public NVTX/nsys internal overflow mechanics are not documented `[ASSUMED — searches inconclusive]`, but the standard profiler discipline is well established: stamp-and-hand-off on the app thread into a **bounded** per-thread or lock-free buffer, drain on a dedicated thread, and **drop-with-counter** rather than block under overload.

**Recommended concrete design (honors D-06/D-07/D-08):**
```
[app thread, NVTX callback]
   stamp Event::new_now timestamp  →  build owned NvtxEvent (copy immediate const char* strings)
   →  ring.push(ev)  (non-blocking; on Full: DROPPED.fetch_add(1, Relaxed), return)
[dedicated drain thread]
   ring.pop() loop  →  EventSender::send(Event<NvtxEventKind>)
[at shutdown]  emit a final "captured N, dropped M" summary event
```
- **Buffer:** a fixed-capacity lock-free MPMC/MPSC queue. **Primary recommendation: `crossbeam-queue::ArrayQueue`** (bounded, lock-free, `push` returns `Err(val)` on Full — perfect for drop-and-count). **std-only fallback:** `std::sync::mpsc::sync_channel(N)` with `try_send` (returns `TrySendError::Full` without blocking; drained by a std thread). Either satisfies Phase 1; `crossbeam-queue` is the cleaner lock-free choice and is a tiny, ubiquitous dep.
- **Placement (Claude's discretion, D-03):** put the ring + drain thread in the **`quent-nvtx` bridge**, not the injection crate. The injection crate exposes only `install_hook(Fn(NvtxEvent))`; the bridge's hook implementation is `move |ev| { stamp+push }`. This keeps the injection crate Quent-agnostic and upstreamable.
- **Drop policy:** drop-newest (simplest; the incoming event is discarded on Full) with an atomic counter. Drop-oldest is possible with `ArrayQueue::force_push` if preferred; drop-newest is the safe default.
- **Phase 1 proof (D-08):** a unit/integration test that fills the ring (tiny capacity, slow/paused drain) and asserts the producer **never blocks** and the **drop counter increments** — not a full high-frequency load test (that's Phase 5).

**Do NOT** feed NVTX capture into the collector client's *bounded blocking* mpsc (`crates/collector/client`) — its `send` awaits when full and would stall the instrumented app (Pitfall 3). Keep capture on the ring→`EventSender::send` (unbounded, non-blocking) side; let backpressure live at the exporter/collector, never at the callback.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `bindgen` (build-dep) | `0.72.x` | Generate Rust FFI for NVTX C headers (callback tables, CBID enums, `nvtxEventAttributes_t`) | De-facto C→Rust standard; what upstream `nvtx-sys/build.rs` uses. Headers are macro-heavy/versioned — hand-writing rots. `[VERIFIED: crates.io slopcheck OK]` `[CITED: STACK.md — 0.72.1 verified current 2026-05]` |
| `cc` (build-dep, **feature-gated**) | `1.2.x` | Compile the strong-`InitializeInjectionNvtx2_fnptr` C shim for the **static-injection** path only | Only needed for the link-time path; keep behind a `static-injection` feature so default (cdylib) builds stay shim-free. `[VERIFIED: crates.io slopcheck OK]` |
| `crossbeam-queue` | `0.3.x` | Bounded lock-free `ArrayQueue` for the hot-path ring (R-04) | Standard lock-free bounded queue; `push` returns `Err` on Full → clean drop-and-count. `[VERIFIED: crates.io slopcheck OK]` `[ASSUMED: exact 0.3.x — verify with pixi cargo]` |
| `uuid` | workspace (`v7`) | Per-session/entity IDs for `Event<T>` | Already a workspace dep; `Uuid::now_v7()`. `[VERIFIED: Cargo.toml, events code]` |
| `serde` / `serde_json` | workspace | `NvtxEvent` serde derive; ndjson round-trip in tests | Existing backbone; ndjson exporter uses `serde_json`. `[VERIFIED: crates/exporter/ndjson]` |
| `thiserror` | workspace `2.x` | Error types for bindgen/hook-install/mediator failures | Reuse workspace `thiserror` — no new dep. `[VERIFIED: Cargo.toml:126]` |
| `std::sync::OnceLock` | std | One-shot `install_hook` guard (NVTX init is once-per-process) | edition 2024 std; no external crate. `[VERIFIED: CLAUDE.md edition 2024]` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `cargo_metadata` (build-dep) | `0.23.x` | Locate NVTX headers from a git dep at build time | **Only if** the git-dep header path (R-02 option 1) is chosen. Not needed for vendored headers (recommended). `[VERIFIED: STACK.md]` |
| `libc` | `0.2` | Raw POSIX fallback (rarely needed) | Only if a raw symbol libloading/std doesn't cover. Not expected in Phase 1. |
| `quent-events` | path | `Event<T>`, `Event::new_now`, `EntityEvent` | The bridge wraps `NvtxEvent` here (CAP-04). `[VERIFIED: crates/events/src/lib.rs]` |
| `quent-instrumentation` | path | `EventSender` sink | The bridge's drain target. `[VERIFIED: observer.rs]` |
| `quent-exporter` (ndjson feature) | path | Test-app capture format (D-10) | Test app writes ndjson; harness reads back. `[VERIFIED: crates/exporter/ndjson]` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `crossbeam-queue::ArrayQueue` | `std::sync::mpsc::sync_channel` + `try_send` | std-only, zero new dep; slightly less ergonomic, channel-not-queue. Fine for Phase 1's design-level proof. |
| Vendored headers | git-pin `nvtx-sys` (PR #87) | Needs `deny.toml allow-git` edit + network in CI; less separable. |
| Vendored headers | Checked-in generated `bindings.rs` | Drops `libclang` from CI entirely (most hermetic); manual regen when headers change. Strong secondary option. |
| `cc` C shim | Pure-Rust `#[used] #[no_mangle] static` | Prefer pure Rust; validate it beats NVTX's weak symbol at link time, else fall back to shim. |
| `libloading` (Phase 4) | — | **Not needed in Phase 1** — that's the fan-out mediator's dlopen dep. Do not pull it in yet. |

**Installation (indicative — verify with `pixi run cargo add` before locking):**
```toml
# integrations/nvtx/injection/Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]   # cdylib for NVTX_INJECTION64_PATH; rlib for the static/test path

[dependencies]
thiserror = { workspace = true }

[build-dependencies]
bindgen = "0.72"
# cc only under [features] static-injection

[features]
static-injection = ["dep:cc"]     # link-time strong-symbol path; off by default
```

## Package Legitimacy Audit

slopcheck 0.6.1 ran against crates.io; all four packages returned `[OK]` (the trailing `cargo not found` error is a PATH artifact of the bare shell — cargo is provided via `pixi run` — and does not affect the registry legitimacy scan, which completed: "scanned 4 packages, 4 OK").

| Package | Registry | Age / Standing | Source Repo | slopcheck | Disposition |
|---------|----------|----------------|-------------|-----------|-------------|
| `bindgen` | crates.io | rust-lang project, ~9 yrs, millions/mo | github.com/rust-lang/rust-bindgen | [OK] | Approved (build-dep) |
| `cc` | crates.io | rust-lang project, ubiquitous | github.com/rust-lang/cc-rs | [OK] | Approved (feature-gated build-dep) |
| `crossbeam-queue` | crates.io | crossbeam-rs, ubiquitous | github.com/crossbeam-rs/crossbeam | [OK] | Approved (runtime) |
| `crossbeam-channel` | crates.io | crossbeam-rs, ubiquitous | github.com/crossbeam-rs/crossbeam | [OK] | Alternative (not required if ArrayQueue chosen) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*NVTX C headers are vendored source (recommended, R-02), not a package. If the git-dep path is chosen instead, `cargo-deny` `allow-git` must be amended and the pinned NVTX commit treated as a dependency.*

## Architecture Patterns

### System Architecture Diagram
```
[ Deterministic NVTX test-app binary ]  (Rust; emits fixed NVTX script; multi-threaded; NO GPU)
      │  NVTX v3 calls (nvtxRangePush/Pop, RangeStart/End, Mark, Domain*, RegisterString,
      │                 NameCategory, NameThread, Resource*, + CORE payload union)
      ▼  NVTX runtime dispatch (tables installed at InitializeInjectionNvtx2)
┌──────────────────────────────────────────────────────────────────────────┐
│ quent-nvtx-injection  (cdylib; loaded via NVTX_INJECTION64_PATH)           │
│  - #[no_mangle] extern "C" InitializeInjectionNvtx2 → fill CORE/CORE2 tables│
│  - catch_unwind at every callback boundary                                 │
│  - callback: stamp? NO — copy immediate strings, build raw NvtxEvent,      │
│              invoke the sink-agnostic Fn(NvtxEvent) hook, return fast       │
└───────────────────────────────┬──────────────────────────────────────────┘
                                 │ NvtxEvent (verbatim; raw u64 handles)
                                 ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ quent-nvtx  (bridge; the only crate touching Quent internals)             │
│  hook = |ev| { let e = Event::new_now(id, ev.into());                      │
│                if ring.push(e).is_err() { DROPPED.fetch_add(1) } }         │ ← BOUNDED, non-blocking
│  [drain thread]  ring.pop() → EventSender::send(...)                        │
└───────────────────────────────┬──────────────────────────────────────────┘
                                 │ Event<NvtxEventKind>  ── EXISTING QUENT SPINE ──
                                 ▼
                    EventSender (unbounded tokio mpsc) → ndjson exporter → file
                                 ▲
[ Parent integration test ]  spawns the app as a SUBPROCESS with NVTX_INJECTION64_PATH set,
                             then reads the .ndjson file and asserts the event stream.
```

### Recommended Project Structure
```
integrations/nvtx/               # separable, upstreamable capture layer (NEW top-level, D-02)
├── events/                      # quent-nvtx-events: verbatim NvtxEvent serde vocabulary (zero Quent deps)
├── injection/                   # quent-nvtx-injection: cdylib, bindgen build.rs, callbacks, Fn(NvtxEvent) hook
│   ├── build.rs                 # bindgen over vendored headers
│   ├── vendor/nvtx3/            # pinned NVTX C headers (R-02 recommendation)
│   └── src/{lib,init,callbacks,convert}.rs
└── instrumentation/             # quent-nvtx: ring + drain thread + EventSender bridge (only crate w/ Quent deps)

examples/nvtx/ OR integrations/nvtx/instrumentation/tests/
├── app/                         # deterministic multi-threaded NVTX emitter binary (VAL-01, D-09/D-11)
└── tests/                       # subprocess integration test: spawn app, assert ndjson (VAL-02, D-10)
```
*(Note: CONTEXT.md D-02 names three crates `{events,injection,instrumentation}`; the domain research's `sys`/`mediator` split is deferred — `sys` folds into `injection` for Phase 1, `mediator` is Phase 4.)*

### Pattern 1: Verbatim capture, interpret later
**What:** Injection emits NVTX calls verbatim — raw `u64` handles, `NvtxMessage::{String, RegisteredHandle}`, domain/category ids — with **no** name resolution. **When:** every handle-bearing event. All interpretation is Phase 2. **Why:** hot path does zero map lookups; the injection crate stays dumb and upstreamable; robust to lazy registration. `[CITED: ARCHITECTURE.md Pattern 3; CONTEXT.md specifics]`

### Pattern 2: Stamp-and-hand-off with a bounded ring
**What:** copy caller-owned strings inside the callback, build an owned `NvtxEvent`, push to a bounded lock-free ring; drain thread forwards to `EventSender`. **When:** always, at the capture→bridge boundary. See R-04. **Why:** app thread never blocks/allocates unboundedly (CAP-05).

### Pattern 3: cdylib injection via `NVTX_INJECTION64_PATH`
**What:** ship the injection as a `cdylib` exporting `InitializeInjectionNvtx2`; the harness/app points `NVTX_INJECTION64_PATH` at it. **When:** the primary attach path (CAP-01). See R-03.

### Anti-Patterns to Avoid
- **Interpreting handles on the hot path** (locked map lookup in the callback) — kills upstreamability and adds latency. Resolve in Phase 2's analyzer.
- **Blocking/awaiting in the callback** (bounded blocking channel, `EventSender` is fine but the collector client is not) — stalls the instrumented app.
- **Storing the raw `const char*`** for later drain-thread serialization — use-after-free crashing the *app* (Pitfall 4). Copy immediately.
- **Static-initializer (`#[ctor]`) capture-state setup** — races NVTX's lazy init; construct state inside `InitializeInjectionNvtx2` guarded by `OnceLock` (Pitfall 6).
- **Building on the dead preliminary crates** (`crates/fsm`, `crates/schema`, etc. — issue #191). Use `crates/events` + `crates/instrumentation`. `[VERIFIED: CLAUDE.md, Cargo.toml:54-62]`
- **In-process `cargo test` for the hook** — one-shot process-global state contaminates tests. Subprocess only (Pitfall 11, VAL-02).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| NVTX C FFI bindings | Hand-written structs/enums for the full surface | `bindgen` over vendored headers | Macro-heavy, versioned headers; hand-maintenance rots against NVTX releases. |
| Bounded lock-free hand-off | Custom ring with `unsafe` atomics | `crossbeam-queue::ArrayQueue` (or std `sync_channel`) | Lock-free bounded queues are notoriously easy to get subtly wrong; these are battle-tested. |
| Capture timestamp | New clock | `Event::new_now` / `crates/time` | Reuses Quent's timestamp path; binning/timeline in later phases come free. |
| Event pipeline / exporter | New sink/format | `EventSender` + ndjson exporter | Existing spine; D-04/D-10 mandate reuse. |
| Panic-across-FFI safety | Manual guards | `std::panic::catch_unwind` at each callback | A Rust panic unwinding into C is UB; `catch_unwind` is the standard containment. |

**Key insight:** Phase 1's only genuinely new code is the FFI vocabulary, the callback→`NvtxEvent` conversion, and the bounded hand-off. Everything else is reuse.

## Runtime State Inventory

*Greenfield capture layer — no rename/refactor/migration. This section is N/A. No existing stored data, live-service config, OS-registered state, secrets, or build artifacts carry a value this phase changes.* One near-adjacent item worth noting: `NVTX_INJECTION64_PATH` is an **environment variable the test harness sets** (not persisted state) — it must point at the built `.so` path, which is a build-artifact path the harness resolves at runtime (e.g. via `CARGO_BIN_*`/`env!` or `cargo`'s artifact dir).

## Common Pitfalls

### Pitfall 1: Events emitted before the hook is installed are silently dropped
**What goes wrong:** NVTX resolves its table once, lazily, at the first NVTX call; anything before `InitializeInjectionNvtx2` (e.g. static-init registrations in a linked GPU lib) vanishes. **Avoid:** ensure `NVTX_INJECTION64_PATH` is set before the app's first NVTX touch; make init idempotent and minimal. For Phase 1's controlled test app this is easy — set the env var in the harness before spawn. **Warning signs:** ranges citing handles whose registration event was never captured.

### Pitfall 2: Doing real work in the callback distorts the measured app
**What goes wrong:** allocation/serialization/locking in the callback inflates the very ranges being measured. **Avoid:** callback does only stamp + copy immediate strings + build `NvtxEvent` + non-blocking ring push. Serialization happens on the drain thread. **Warning signs:** range durations change materially attached vs detached.

### Pitfall 3: C-string lifetime — use-after-free from the callback
**What goes wrong:** `const char*` message args are valid only for the call's duration; stashing the pointer for later drain-thread use is UAF crashing the *app*. **Avoid:** copy the bytes into owned storage **inside** the callback. Registered strings are the opposite — capture the string once at registration, store only the handle thereafter. **Warning signs:** intermittent garbled/empty messages; ASAN UAF in the drain path.

### Pitfall 4: Struct version/size ABI drift
**What goes wrong:** `nvtxEventAttributes_t` carries explicit `version`/`size`; reading fixed offsets past `size` reads foreign memory when the app was built against a different NVTX version. **Avoid:** branch on `version`/`size`; only read members `size` says are present; pin+document the vendored header version as the minimum contract; forward-tolerate. **Warning signs:** nonsensical category/color/payload only for certain apps.

### Pitfall 5: Symbol/linkage & init-order (link-time path)
**What goes wrong:** our strong symbol must beat NVTX's weak `InitializeInjectionNvtx2`; if ours is also weak → silent no injection; duplicate strong (CUPTI linked) → link error/UB. **Avoid:** prefer the runtime cdylib path (linkage-order irrelevant); for the static path force strong via `cc` shim or validated `#[used] #[no_mangle]`; keep init side effects lazy/guarded. **Warning signs:** "works when linked this way"; no events in some builds.

### Pitfall 6: Process-global one-shot state breaks in-process `cargo test`
**What goes wrong:** the hook + NVTX one-shot table fire once per process; `cargo test`'s multi-threaded single-process harness can't re-install and contaminates across tests. **Avoid:** subprocess integration tests (VAL-02); keep pure logic (conversion) in side-effect-free functions unit-testable in-process. **Warning signs:** tests pass alone, fail in-suite.

## Code Examples

### CORE callback → verbatim NvtxEvent (shape)
```rust
// integrations/nvtx/injection/src/callbacks.rs
// Source pattern: NVTX release-v3 sample-injection + PR #87 convert.rs (reference-only)
extern "C" fn on_domain_range_push_ex(
    domain: nvtxDomainHandle_t,
    attr: *const nvtxEventAttributes_t,
) -> i32 {
    let r = std::panic::catch_unwind(|| {
        // SAFETY: NVTX guarantees `attr` valid for the call's duration.
        let a = unsafe { &*attr };
        // Branch on a.version / a.size before reading later members (Pitfall 4).
        let msg = copy_message(a);            // copies immediate const char*; keeps handle for registered
        HOOK.get().map(|h| h(NvtxEvent::RangePush {
            domain: domain as u64,            // raw handle, verbatim (resolve in Phase 2)
            category: a.category,
            message: msg,
            payload: copy_core_payload(a),    // CORE payload union (R-01 recommendation)
            // ... color, thread id ...
        }));
    });
    let _ = r; // a panic here must not unwind into C (UB)
    0
}
```

### Bridge: bounded ring hand-off (shape)
```rust
// integrations/nvtx/instrumentation/src/lib.rs
static DROPPED: AtomicU64 = AtomicU64::new(0);

pub fn install(sender: EventSender<NvtxEventKind>, session: Uuid) {
    let ring = Arc::new(ArrayQueue::<Event<NvtxEventKind>>::new(RING_CAPACITY));
    // drain thread
    { let ring = ring.clone();
      std::thread::spawn(move || while let Some(ev) = ring.pop() { sender.send(ev); }); }
    // sink-agnostic hook installed into the injection crate
    quent_nvtx_injection::install_hook(move |ev: NvtxEvent| {
        let e = Event::new_now(session, NvtxEventKind::from(ev));   // CAP-04 timestamp
        if ring.push(e).is_err() { DROPPED.fetch_add(1, Ordering::Relaxed); } // D-07 drop-and-count
    });
}
```
*(Spin the drain on a `park`/`recv` primitive in real code rather than a busy `pop` loop; shown minimal for clarity.)*

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `InitializeInjectionNvtx2` (older `...Nvtx` v1) | Prefer the v2 entry point; verify returned table version | NVTX v3 | Use `Nvtx2`; check table version. |
| git-dep NVTX headers (PR #87) | Vendored/pinned headers or checked-in bindings | This research (R-02) | Avoids `deny.toml allow-git` edit; hermetic CI. |
| Direct unbounded `EventSender::emit` from callback (PR #87 shape) | Bounded ring + drop-count + drain thread | This research (R-04) | Satisfies CAP-05's no-unbounded-allocation clause. |

**Deprecated/outdated:** none blocking; PR #87 predates Quent's exporter/serde-bounds refactors (#324) — another reason for the reference-only rebuild (D-01).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | libcudf/cuCascade do not broadly emit the payload *extension* (schemas/binary) today | R-01 | If they do, deferring extension capture loses fidelity on real workloads — but decode is v2 regardless; capture could be added later without a vocabulary change. |
| A2 | Deferring payload-*extension* capture from Phase 1 is acceptable to the user | R-01 | Changes success-criterion 2 and CAP-03 mapping; **needs user confirmation** before locking. |
| A3 | Vendoring headers (vs git-dep) is the preferred sourcing | R-02 | Low — all three options are viable; vendoring is strictly the most hermetic. Needs user confirmation. |
| A4 | `crossbeam-queue::ArrayQueue` is an acceptable new runtime dep (vs std-only `sync_channel`) | R-04, Stack | Low — std fallback exists; either meets D-06/D-07. |
| A5 | Pure-Rust `#[used] #[no_mangle]` static *may* override NVTX's weak symbol without the `cc` shim | R-03 | Must be validated empirically at link time; `cc` shim is the fallback. |
| A6 | Exact crate versions (bindgen 0.72.x, crossbeam-queue 0.3.x, cc 1.2.x) | Stack | Low — verify with `pixi run cargo add`; domain research verified currency 2026-07-08. |

## Open Questions (RESOLVED)

All four resolved at a user planning checkpoint (2026-07-13); see CONTEXT.md D-12..D-16 for the locked decisions.

1. **Payload-extension scope (R-01/A2) — RESOLVED (D-12):** Defer the payload-extension module; capture the CORE payload union now. Plan includes a task updating REQUIREMENTS.md CAP-03 + ROADMAP SC-2. (Evidence: libcudf emits zero payload-extension events.)
2. **Header sourcing (R-02/A3) — RESOLVED (D-13):** Git-dep on NVIDIA/NVTX (overrides this doc's vendoring recommendation), pinned `rev`, `deny.toml allow-git` amended, `build.rs` locates headers via `cargo_metadata`.
3. **CI `libclang` (R-02) — RESOLVED (D-14):** Check in generated `bindings.rs`; gate regen behind the `regenerate-bindings` feature. No `libclang` in CI; the NVTX git-dep is optional/dev-only so default builds stay hermetic.
4. **Strong-symbol override (R-03/A5) — RESOLVED (D-15):** Keep the `cc` shim + whole-archive override; the static path is feature-gated (`static-injection`). Runtime `NVTX_INJECTION64_PATH` cdylib is the primary Phase-1 deliverable.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust / cargo (via pixi) | all crates | ✓ (via `pixi run`) | edition 2024, ≥1.93 | — |
| `libclang` | bindgen at build time | ✓ on dev machine; ✗ in `pixi.toml` | llvm-18 (`/usr/lib/llvm-18/lib/libclang.so.1`) | Check in generated `bindings.rs` (drops the CI requirement) |
| `cc` / C toolchain | static-injection shim (feature-gated) | ✓ (`cxx-compiler` in pixi) | — | Pure-Rust `#[no_mangle]` static |
| GPU / CUDA | **nothing in Phase 1** | ✗ (by design) | — | Deterministic in-repo test app (VAL-01) — no GPU needed |
| NVTX C headers | bindgen | vendored (recommended) | pinned NVTX `release-v3` commit | git-dep + `allow-git` edit |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** `libclang` not in `pixi.toml` — fall back to checked-in `bindings.rs`, or add `clang` to `pixi.toml`.

## Validation Architecture

Nyquist validation is **enabled** (`config.json workflow.nyquist_validation: true`). Every success criterion is provable under GPU-less CI via the subprocess harness + ndjson-file assertions.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` (integration tests spawning subprocesses) |
| Config file | none — Cargo convention (`tests/` dir + `[[test]]`); test-app is a `[[bin]]` |
| Quick run command | `pixi run cargo test -p quent-nvtx` (unit: conversion, ring drop-count) |
| Full suite command | `pixi run cargo test -p quent-nvtx-events -p quent-nvtx-injection -p quent-nvtx` (incl. subprocess integration test) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CAP-01 | cdylib attaches via `NVTX_INJECTION64_PATH`, no app code change | integration (subprocess) | `cargo test -p quent-nvtx --test capture_e2e` | ❌ Wave 0 |
| CAP-02 | All CORE/CORE2 kinds appear in the ndjson stream | integration (subprocess) → assert every kind present | same harness, per-kind assertions | ❌ Wave 0 |
| CAP-03 | CORE payload union preserved verbatim (extension deferred per R-01) | integration | assert payload field round-trips | ❌ Wave 0 |
| CAP-04 | Every event has a capture timestamp; reaches ndjson via `EventSender` | integration | assert `timestamp` present & monotone-ish; file non-empty | ❌ Wave 0 |
| CAP-05 | Ring fills → producer never blocks, drop counter increments | unit | `cargo test -p quent-nvtx ring_drops_when_full` | ❌ Wave 0 |
| VAL-01 | Deterministic multi-threaded app emits fixed script, no GPU | integration (the app itself + harness) | `cargo test -p quent-nvtx --test capture_e2e` | ❌ Wave 0 |
| VAL-02 | Hook installed once per process → subprocess isolation | integration (spawn design) | harness uses `std::process::Command` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `pixi run cargo test -p <crate-under-edit>` + `pixi run cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` (CI gate) + `pixi run cargo fmt --all -- --check`.
- **Per wave merge:** full Phase-1 crate test set incl. the subprocess integration test.
- **Phase gate:** full suite green + `cargo deny check` clean before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `integrations/nvtx/instrumentation/tests/capture_e2e.rs` — subprocess harness: set `NVTX_INJECTION64_PATH`, spawn the test-app, read ndjson, assert (CAP-01/02/03/04, VAL-01/02).
- [ ] Test-app binary (`examples/nvtx/app` or a `[[bin]]`) — deterministic multi-threaded NVTX emitter covering every kind (D-11).
- [ ] `integrations/nvtx/instrumentation/src` unit test — ring fills → drop-count, no block (CAP-05).
- [ ] `quent-nvtx-events` unit tests — `NvtxEvent` serde round-trip (pure, in-process).
- [ ] Confirm `cargo` binary path in CI is via `pixi run` (bare shell has no `cargo` on PATH).

*(The test-app should also script the nasty cases — unclosed ranges, cross-thread start/end, multi-domain handle reuse — so Phase 2's tolerant analyzer inherits ready-made fixtures, per Pitfall 6 / SUMMARY.md.)*

## Security Domain

`security_enforcement` is not set in `config.json` (absent = treat as enabled). This is in-process instrumentation loaded into the target's address space; "security" here is largely **process integrity**, not a network surface.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth surface in the capture layer |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | Validate `nvtxEventAttributes_t` `version`/`size` before reading members; bounds-check before any offset read (Pitfall 4). NVTX callback args are effectively untrusted foreign input. |
| V6 Cryptography | no | None in this phase |

### Known Threat Patterns for Rust FFI injection
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Rust panic unwinding across the C ABI into the app | Denial of Service (app crash / UB) | `std::panic::catch_unwind` at every callback boundary |
| Use-after-free of caller-owned `const char*` | Tampering / DoS | Copy immediate strings inside the callback before returning |
| Over-read past `size` on a foreign-version attribute struct | Info disclosure / DoS | Honor `version`/`size`; only read declared members |
| Injecting our `.so` into an arbitrary process via `NVTX_INJECTION64_PATH` | Elevation of trust boundary | Document the trust boundary — the injector is loaded only into opt-in processes; no elevated behavior |
| Unbounded allocation under high-frequency emission (OOM the app) | DoS | Bounded ring + drop-and-count (CAP-05) |

## Sources

### Primary (HIGH confidence)
- NVIDIA/NVTX `dev` — `tools/sample-injection/README.md` + `Source/` (attach handshake: `NVTX_INJECTION64_PATH` → `InitializeInjectionNvtx2(NvtxGetExportTableFunc_t)` → `NVTX_CB_MODULE_CORE/CORE2` tables, `NVTX_CBID_*` indices) — read via `gh api`.
- NVIDIA/NVTX `release-v3` — `c/include/nvtx3/nvtxDetail/{nvtxInit.h,nvtxTypes.h,nvtxExtInit.h,nvtxExtPayloadTypeInfo.h}` present (payload extension is a separate module) — confirmed via `gh api`.
- Quent codebase (read directly): `crates/instrumentation/src/observer.rs` (unbounded `EventSender`), `crates/events/src/lib.rs` (`Event::new_now`), `crates/exporter/ndjson/src/lib.rs`, `domains/query_engine/tests/fixed/{Cargo.toml,src}`, root `Cargo.toml` (members/default-members), `deny.toml` (`allow-git = []`), `pixi.toml` (no `clang`).
- `.planning/research/{SUMMARY,STACK,ARCHITECTURE,PITFALLS}.md` (2026-07-08, HIGH; verified against NVTX release-v3 primary source).

### Secondary (MEDIUM confidence)
- CUPTI NVTX tutorial (eunomia), Nsight Systems User Guide, NVTX docs (nvidia.github.io/NVTX) — attach mechanism, payload union as an optional field — via WebSearch summaries.

### Tertiary (LOW confidence)
- nsys/profiler internal overflow mechanics (ring/drop policies) — not publicly documented; R-04 design rests on established profiler discipline + Quent's `EventSender` constraints, not on a cited nsys implementation. `[ASSUMED]`
- libcudf payload-extension adoption level — search inconclusive. `[ASSUMED]`
- Exact current crate versions — verify with `pixi run cargo add`. `[ASSUMED]`

## Metadata

**Confidence breakdown:**
- Attach mechanism (R-03): HIGH — verified against NVTX sample-injection primary source.
- Standard stack: HIGH — packages verified via slopcheck; versions from recent domain research.
- Hot-path/overflow design (R-04): MEDIUM-HIGH — `EventSender` constraint verified in code; nsys internals assumed.
- Payload scope (R-01): MEDIUM — extension module structure verified; libcudf adoption assumed; recommendation needs user confirmation.
- Header sourcing (R-02): HIGH on the `deny.toml`/`pixi.toml` facts (read directly); recommendation is a judgment call.
- Pitfalls: HIGH — Quent hazards from code + CONCERNS; FFI details MEDIUM.

**Research date:** 2026-07-13
**Valid until:** ~2026-08-13 (stable domain; re-verify crate versions and NVTX header commit before locking the plan).
