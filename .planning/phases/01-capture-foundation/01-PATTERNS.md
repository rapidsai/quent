# Phase 1: Capture Foundation - Pattern Map

**Mapped:** 2026-07-13
**Files analyzed:** 22 (new crates, build scripts, test app, harness, workspace/registry edits, doc updates)
**Analogs found:** 18 / 22 (4 have no in-repo analog — the FFI/bindgen/strong-symbol mechanism; PR #87 is the external reference)

> **Precedence note for the planner:** where CONTEXT.md's *locked* decisions
> (D-01..D-16) diverge from 01-RESEARCH.md's *recommendations*, the locked
> decisions win. Two divergences matter here:
> - **Headers:** D-13 LOCKS a **git-dep on NVIDIA/NVTX** (`build.rs` locates it
>   via `cargo_metadata`) + a `deny.toml allow-git` edit. Research recommended
>   vendored headers; that recommendation is **overridden** — plan for the git-dep.
> - **Bindings:** D-14 LOCKS **committing the generated `bindings.rs`** with
>   feature-gated regen (no `libclang` in CI). Follow D-14.

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `integrations/nvtx/events/src/lib.rs` | model (event vocab) | transform | `crates/events/src/lib.rs`, `crates/events/src/trace.rs` | exact |
| `integrations/nvtx/events/src/attributes.rs` | model | transform | `crates/events/src/trace.rs` | exact |
| `integrations/nvtx/events/src/payload.rs` | model | transform | `crates/events/src/trace.rs` | role-match |
| `integrations/nvtx/events/Cargo.toml` | config | — | `crates/events/Cargo.toml` | exact |
| `integrations/nvtx/injection/src/lib.rs` | provider (cdylib entry + `install_hook`) | event-driven | `crates/instrumentation/src/lib.rs` (module re-export shape only) | role-match (FFI body: NO analog) |
| `integrations/nvtx/injection/src/init.rs` | provider (`InitializeInjectionNvtx2`) | event-driven | **NO in-repo analog** — PR #87 `init.rs` + NVTX `sample-injection` | none (external ref) |
| `integrations/nvtx/injection/src/callbacks.rs` | provider (C-ABI callbacks) | event-driven | **NO in-repo analog** — PR #87 `callbacks.rs` | none (external ref) |
| `integrations/nvtx/injection/src/convert.rs` | utility (args → `NvtxEvent`) | transform | `crates/events/src/trace.rs` (shape target only) | partial |
| `integrations/nvtx/injection/src/bindings.rs` | generated FFI (committed, D-14) | — | none (generated artifact) | none |
| `integrations/nvtx/injection/build.rs` | build script (bindgen + `cargo_metadata`) | batch | `crates/collector/proto/build.rs` (external-codegen shape); `examples/cpp-integration/bridge/build.rs` (cc shape) | partial |
| `integrations/nvtx/injection/wrapper.h` / `c/symbol.c` | config / C shim | — | **NO in-repo analog** — PR #87 `wrapper.h`, `c/symbol.c` | none (external ref) |
| `integrations/nvtx/injection/Cargo.toml` | config (cdylib+rlib, feature-gated `cc`) | — | `examples/cpp-integration/bridge/Cargo.toml` (crate-type + build-deps) | role-match |
| `integrations/nvtx/instrumentation/src/lib.rs` | provider (bridge: ring + drain + `EventSender`) | event-driven / streaming | `crates/instrumentation/src/observer.rs` (`spawn_forwarder` drain pattern) | role-match |
| `integrations/nvtx/instrumentation/Cargo.toml` | config | — | `crates/instrumentation/Cargo.toml`, `crates/events/Cargo.toml` | role-match |
| test-app binary (`[[bin]]` under instrumentation, or `examples/nvtx/app`) | binary (deterministic emitter) | event-driven | `domains/query_engine/tests/fixed/src/{main,lib}.rs` | exact |
| `integrations/nvtx/instrumentation/tests/capture_e2e.rs` | test (subprocess harness) | request-response (spawn+assert file) | `crates/instrumentation/tests/collector_roundtrip.rs`; ndjson round-trip in `crates/instrumentation/src/lib.rs` tests | role-match |
| ring drop-count unit test | test | streaming | `crates/instrumentation/src/observer.rs` `#[cfg(test)]` | role-match |
| `NvtxEvent` serde round-trip unit test | test | transform | `crates/exporter/ndjson/src/lib.rs` `#[cfg(test)]` | exact |
| root `Cargo.toml` (members/default-members/workspace.deps) | config | — | root `Cargo.toml` | exact (self) |
| `deny.toml` (`allow-git` edit, D-13) | config | — | `deny.toml` | exact (self) |
| build-instructions README (regen doc, D-14) | docs | — | — | none |
| `REQUIREMENTS.md` CAP-03 + `ROADMAP.md` SC-2 (D-12 scope narrowing) | docs | — | — | none |

---

## Pattern Assignments

### `integrations/nvtx/events/src/{lib,attributes,payload}.rs` (model, transform)

**Analog:** `crates/events/src/trace.rs` (enum-of-structs serde vocabulary) + `crates/events/src/lib.rs` (`EntityEvent` trait + `Event<T>` wrapper).

**SPDX header + module doc** — mandatory on every file (`crates/events/src/trace.rs:1-4`):
```rust
// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Module containing events for run-time defined tracing
```

**Verbatim event vocabulary — enum of structs, derive `Deserialize, Serialize`** (`crates/events/src/trace.rs:10-55`). Mirror this exact shape for `NvtxEvent` (raw `u64` handles per D-01/specifics; `NvtxMessage::{String, RegisteredHandle}`):
```rust
pub type SpanId = u64;

#[derive(Debug, Deserialize, Serialize)]
pub struct SpanInit {
    pub id: SpanId,
    pub name: String,
    pub parent_id: Option<SpanId>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TraceEvent {
    /// Declare a trace entity.
    Init(TraceInit),
    /// Declare a span within the trace.
    Span(SpanInit),
    // ...
}
```

**`EntityEvent` impl** — the event stream type MUST implement this so it flows through `EventSender`/exporters (`crates/events/src/lib.rs:13-16`, impl example `crates/exporter/ndjson/src/lib.rs:143-145`):
```rust
pub trait EntityEvent {
    /// The name of the entity.
    const NAME: &'static str;
}
// impl:  impl EntityEvent for NvtxEventKind { const NAME: &'static str = "NvtxEvent"; }
```

**Cargo.toml** (`crates/events/Cargo.toml`) — zero Quent-internal deps beyond serde/uuid (D-03 keeps `events` sink-agnostic; it is the shared vocabulary). Use `.workspace = true` inheritance:
```toml
[package]
name = "quent-nvtx-events"
version.workspace = true
edition.workspace = true
publish.workspace = true

[dependencies]
serde.workspace = true
uuid = { workspace = true, features = ["serde"] }
```
> Note: `crates/events` depends on `quent-time`/`quent-attributes`. For strict D-03 separability of the events crate, keep those out unless a captured field needs them — the CORE payload union is plain scalars.

---

### `integrations/nvtx/injection/build.rs` (build script — bindgen + `cargo_metadata`, D-13/D-14)

**Analog (closest shape):** `crates/collector/proto/build.rs` (external-source codegen at build time) and `examples/cpp-integration/bridge/build.rs` (cc/codegen). **No repo analog uses `bindgen` or `cargo_metadata`** — those come from PR #87.

**`rerun-if-changed` + fallible `main` returning boxed error** (`crates/collector/proto/build.rs:4-8`):
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=../../../proto");
    tonic_prost_build::compile_protos("...")?;
    Ok(())
}
```
Apply: `println!("cargo::rerun-if-changed=wrapper.h")`, then per D-14 gate the bindgen call behind a `regenerate-bindings` feature (`#[cfg(feature = "regenerate-bindings")]` / `env::var("CARGO_FEATURE_...")`) that overwrites the committed `src/bindings.rs`. Default/CI builds skip bindgen and `include!` the committed file. Per D-13 the header path is derived from the NVIDIA/NVTX git-dep located via `cargo_metadata` (`<nvtx-repo>/c/include`).

**cc shim invocation for the `static-injection` feature** — model on the cc/codegen build (`examples/cpp-integration/bridge/build.rs:18-22`, uses `cxx_build`; for a raw C shim use `cc::Build::new().file("c/symbol.c").compile("...")`). Keep behind `[features] static-injection = ["dep:cc"]` (RESEARCH.md Standard Stack / D-15).

**Cargo.toml crate-type + split build-deps** — model on `examples/cpp-integration/bridge/Cargo.toml:7-18`:
```toml
[lib]
crate-type = ["cdylib", "rlib"]   # cdylib = NVTX_INJECTION64_PATH (D-15 primary); rlib = static/test path

[build-dependencies]
bindgen = "0.72"
cargo_metadata = "0.23"           # locate the NVTX git-dep header dir (D-13)
# cc only under [features] static-injection

[features]
static-injection = ["dep:cc"]     # link-time strong-symbol path; off by default
```

---

### `integrations/nvtx/injection/src/{lib,init,callbacks,convert}.rs` (provider — FFI cdylib)

**Analog for the module-layout / re-export skeleton ONLY:** `crates/instrumentation/src/lib.rs:10-16`:
```rust
//! Backing structures for generated instrumentation libraries.
mod context;
mod observer;
mod sidecar;

pub use context::Context;
pub use observer::{EventSender, Observer};
```
Apply the same small-`lib.rs`-declares-private-modules-and-re-exports shape:
`mod init; mod callbacks; mod convert; pub use init::install_hook;` plus the `#[unsafe(no_mangle)] pub extern "C" fn InitializeInjectionNvtx2(...)` entry point.

**The FFI body has NO in-repo analog.** `InitializeInjectionNvtx2`, the CORE/CORE2 `NVTX_CBID_*` callback-table fill, `catch_unwind` at every C-ABI boundary, `const char*` copy-in-callback, `OnceLock` one-shot guard, and the `#[used] #[no_mangle] static` / `cc` strong-symbol override are all **new**. The reference is:
- **PR #87** (`rapidsai/quent`, branch `nvtx`) — `injection/src/{init,callbacks,convert}.rs`, `build.rs`, `c/symbol.c`, `wrapper.h` (reference-only, D-01; do not adopt commits).
- **NVIDIA/NVTX `tools/sample-injection`** — the attach handshake (verified in RESEARCH.md R-03).
- Shape sketch in `01-RESEARCH.md` "Code Examples → CORE callback → verbatim NvtxEvent" (lines 301-325).

Sink-agnostic contract (D-03): the injection crate exposes only `install_hook(hook: impl Fn(NvtxEvent) + Send + Sync + 'static)` guarded by `std::sync::OnceLock`; it must **not** depend on any `quent-*` crate except `quent-nvtx-events`.

---

### `integrations/nvtx/instrumentation/src/lib.rs` (provider — bridge: bounded ring + drain thread → `EventSender`)

**Analog:** `crates/instrumentation/src/observer.rs` — the `spawn_forwarder` drain-loop pattern (lines 142-190) and the `EventSender::send`/`emit` sink (lines 59-72). This is the ONLY Phase-1 crate that touches Quent internals (D-03).

**Sink `send` — non-blocking, unbounded, error-log-once** (`observer.rs:59-72`). The drain thread forwards through exactly this:
```rust
pub fn send(&self, event: Event<T>) {
    if let Some(tx) = &self.tx
        && tx.send(event).is_err()
        && !self.disable_error_log.swap(true, Ordering::Relaxed)
    {
        tracing::error!("unable to send event, suppressing further errors");
    }
}

pub fn emit(&self, id: Uuid, event: impl Into<T>) {
    self.send(Event::new_now(id, event.into()));
}
```

**Capture timestamp (CAP-04)** — wrap each `NvtxEvent` at ingest via `Event::new_now` (`crates/events/src/lib.rs:38-45`). This is the `inline(always)` stamp path the bridge reuses:
```rust
#[inline(always)]
pub fn new_now(id: Uuid, data: T) -> Self {
    Self { id, timestamp: timestamp(), data }
}
```

**Drain-loop + shutdown-drain discipline** — model the bridge's drain thread on `spawn_forwarder`'s select/drain-on-cancel/flush-once structure (`observer.rs:153-179`): loop popping the ring, forward each via `sender.send`, and on shutdown drain the remainder before exiting. Replace the tokio `select!` with a std `thread` + `park`/`recv` (RESEARCH.md R-04 note: don't busy-`pop`).

**D-16 bounded stage (NO in-repo analog — new):** front the unbounded `EventSender` with a bounded lock-free `crossbeam_queue::ArrayQueue<Event<NvtxEventKind>>` + `static DROPPED: AtomicU64`; on `ring.push(e).is_err()` do `DROPPED.fetch_add(1, Ordering::Relaxed)` (drop-and-count, D-07). Shape in `01-RESEARCH.md` lines 327-344. The injection hook installed here is `move |ev| { let e = Event::new_now(session, ev.into()); if ring.push(e).is_err() { DROPPED... } }`.

**Cargo.toml deps:** `quent-nvtx-events` (path), `quent-events` (path), `quent-instrumentation` (path), `crossbeam-queue = "0.3"`, `uuid = { workspace = true }`, `thiserror = { workspace = true }`.

---

### test-app binary (deterministic NVTX emitter) — `domains/query_engine/tests/fixed/` template

**Analog:** `domains/query_engine/tests/fixed/src/{main.rs,lib.rs}` + its `Cargo.toml`.

**Binary wrapper delegating to a library `emit(&ctx)`** (`fixed/src/main.rs:20-25`) — keep the deterministic script in `lib.rs`, thin `main.rs`:
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let ctx = SimulatorContext::try_new(args.exporter.into_options())?;
    emit(&ctx);
    Ok(())
}
```

**Clap `ExporterArgs` flatten** (`fixed/src/main.rs:7-18`, group def `crates/exporter/src/clap.rs:47-64`) — the test-app selects the ndjson exporter via `--exporter ndjson --output-dir <dir>` (D-10). Flatten `#[command(flatten)] exporter: ExporterArgs` and call `.into_options()`.

**Fixed/greppable IDs + module doc describing the script** (`fixed/src/lib.rs:4-59`) — hardcode `uuid!(...)` constants and document the timeline in `//!`. Mirror for the NVTX script (push/pop, RangeStart/End, marks, domain create/destroy, registered strings, category/thread naming, resource create/destroy — D-11), and spawn multiple threads for cross-thread RangeStart/End + per-thread naming.

**Cargo.toml — `__test-clock-override` + default-members exclusion** (`fixed/Cargo.toml:9` + root `Cargo.toml:50-53,65-67`). IF the test app pins deterministic timestamps it must add `quent-time = { path = "...", features = ["__test-clock-override"] }` and be **excluded from `default-members`** (zero-cost guarantee). NOTE: NVTX capture timestamps come from `Event::new_now` in the bridge, not the app — decide during planning whether determinism needs the clock override at all (the harness asserts on kinds/handles/payload, not absolute times).

---

### `integrations/nvtx/instrumentation/tests/capture_e2e.rs` (test — subprocess harness, VAL-02)

**Analog:** `crates/instrumentation/tests/collector_roundtrip.rs` (out-of-the-unit-test integration harness with a poll-until-deadline assert) + the ndjson read-back assertion in `crates/instrumentation/src/lib.rs` tests (lines 47-91).

**`tests/` integration file with SPDX + `mod common`** (`collector_roundtrip.rs:1-9`). Use `std::process::Command` to spawn the built test-app binary (env `CARGO_BIN_EXE_<name>` gives its path) with `NVTX_INJECTION64_PATH` set to the built injection `.so` — see RESEARCH.md "Runtime State Inventory" (line 277) on resolving the cdylib artifact path.

**Poll-until-deadline pattern for async delivery** (`collector_roundtrip.rs:107-116`) — after the subprocess exits, read the ndjson file and assert. Adapt the deadline/sleep loop if waiting on flush.

**ndjson read-back + per-entity dir + `.ndjson` filter** (`crates/instrumentation/src/lib.rs:80-90`):
```rust
let ndjson_files: Vec<_> = std::fs::read_dir(context_dir.join("TestEvent"))
    .unwrap()
    .filter_map(Result::ok)
    .map(|e| e.path())
    .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("ndjson"))
    .collect();
```
Read events back with `NdjsonImporter` (`crates/exporter/ndjson/src/lib.rs:96-135`) — it yields `Event<T>` via `Iterator`; assert every CAP-02 kind present, CORE payload round-trips (CAP-03), and `timestamp` populated (CAP-04).

---

### ring drop-count unit test (CAP-05) & `NvtxEvent` serde round-trip test

**Analogs:** `crates/instrumentation/src/observer.rs:192-208` (`#[cfg(test)] mod tests`) for the ring test placement; `crates/exporter/ndjson/src/lib.rs:137-174` (`#[tokio::test]`, `tempfile::tempdir()`, push→read-back) for serde round-trip. Fill a tiny-capacity ring with a paused drain, assert producer never blocks and `DROPPED` increments (D-08). Keep the pure conversion (`convert.rs`) side-effect-free so it is unit-testable in-process (Pitfall 6 / VAL-02).

---

### root `Cargo.toml` (workspace registration)

**Analog (self):** root `Cargo.toml:3-104`. Add the three crates to `members` under a new `# NVTX integration crates` comment block (lines 3-63 style). Add `quent-nvtx-events`, `quent-nvtx-injection`, `quent-nvtx` to `default-members` (lines 68-104) **unless** a crate activates `quent-time/__test-clock-override` (only the test-app might — mirror the `tests/fixed` exclusion comment at lines 50-53/65-67). Add `crossbeam-queue`, `bindgen`, `cargo_metadata`, `cc` to `[workspace.dependencies]` (lines 112-135) if pinning centrally.

### `deny.toml` (D-13 git-dep allow-list)

**Analog (self):** `deny.toml:49-57`. `allow-git` is currently `[]` with `unknown-git = "deny"`. Add the pinned NVIDIA/NVTX source:
```toml
allow-git = ["https://github.com/NVIDIA/NVTX"]
```
Pin a `rev = "<sha>"` in the Cargo dependency (D-13 refinement over PR #87's `branch`). `cargo deny check` is a phase-gate (RESEARCH.md line 413).

### Doc updates (D-12, D-14)

- `REQUIREMENTS.md` CAP-03 + `ROADMAP.md` §Phase 1 success-criterion 2: narrow to "CORE `nvtxEventAttributes` payload union captured verbatim; payload-**extension** module deferred" (D-12). Mandatory plan task.
- Build-instructions README: document the `regenerate-bindings` command and when to run it (on an NVTX `rev` bump) (D-14). Mandatory plan task.

---

## Shared Patterns

### SPDX license header (mandatory, every source file)
**Source:** every `.rs` in the repo, e.g. `crates/events/src/lib.rs:1-2`
**Apply to:** all new `.rs`, `build.rs`, and `c/*.c` files
```rust
// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0
```

### Crate-level `//!` doc + small-`lib.rs` re-export
**Source:** `crates/instrumentation/src/lib.rs:4-16`, `crates/events/src/lib.rs:4`
**Apply to:** every new crate's `lib.rs` (declare private modules, re-export public surface, `//!` header).

### `Event<T>` capture-timestamp wrapping (CAP-04)
**Source:** `crates/events/src/lib.rs:38-45` (`Event::new_now`)
**Apply to:** the `quent-nvtx` bridge (wrap each `NvtxEvent` before ring push).

### `EventSender::send` non-blocking sink + error-log-once
**Source:** `crates/instrumentation/src/observer.rs:59-72`
**Apply to:** the bridge drain thread (forward each ring-popped `Event`).

### Cargo `.workspace = true` inheritance + workspace-pinned deps
**Source:** `crates/events/Cargo.toml:1-13`, root `Cargo.toml:106-135`
**Apply to:** every new `Cargo.toml` (`version/edition/publish.workspace = true`; `serde.workspace`, `uuid`/`thiserror = { workspace = true }`).

### `#[cfg(test)]` with `tempfile::tempdir()` + read-back assertions
**Source:** `crates/exporter/ndjson/src/lib.rs:137-174`, `crates/instrumentation/src/lib.rs:47-91`
**Apply to:** serde round-trip unit test, ring drop-count test, subprocess harness.

### Error handling — `thiserror` enum + `Result` alias, `try_new` constructors
**Source:** CLAUDE.md conventions; `crates/exporter/ndjson/src/lib.rs:43` (`try_new`), `crates/exporter/types` (per-crate `Result`)
**Apply to:** `install_hook`/bridge/build failures — `thiserror`-derived enums, fallible constructors named `try_new`, `/// # Errors` doc sections.

---

## No Analog Found

Files with no close in-repo match — the planner must lean on PR #87 (external, reference-only) and NVTX `sample-injection`, not force a weak local analog:

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `integrations/nvtx/injection/src/init.rs` | provider (`InitializeInjectionNvtx2`, `OnceLock` one-shot, table fill) | event-driven | No C-ABI injection entry point or NVTX callback-table wiring exists anywhere in the repo. Ref: PR #87 `init.rs` + NVTX `sample-injection`. |
| `integrations/nvtx/injection/src/callbacks.rs` | provider (per-CBID `extern "C"` callbacks, `catch_unwind`, string copy-in) | event-driven | No FFI callback code, no `catch_unwind`-across-C-ABI pattern, no `const char*` copy discipline in the repo. Ref: PR #87 `callbacks.rs`. |
| `integrations/nvtx/injection/src/bindings.rs` | generated FFI | — | Generated bindgen output; committed per D-14. No hand-written analog. |
| `integrations/nvtx/injection/wrapper.h` + `c/symbol.c` | C shim / bindgen umbrella header | — | Repo's only C interop is `cxx`-generated (`examples/cpp-integration`); no raw `wrapper.h` or hand-written strong-symbol `.c`. Ref: PR #87 `wrapper.h`, `c/symbol.c`. |
| build-instructions README (regen doc) | docs | — | New doc (D-14). |

The **bounded ring + drop-and-count** (D-16) also has no in-repo analog for the ring itself (`crossbeam_queue::ArrayQueue`) — only the drain/`EventSender` half is analogous to `spawn_forwarder`. Follow `01-RESEARCH.md` R-04.

---

## Metadata

**Analog search scope:** `crates/{events,instrumentation,exporter/ndjson,collector/proto}`, `domains/query_engine/tests/fixed`, `examples/cpp-integration/bridge`, root `Cargo.toml`, `deny.toml`; `find` for all `build.rs` and `tests/` dirs.
**Files scanned:** 14 read in full/part; ~11 `build.rs` + 10 `tests/` dirs enumerated.
**External reference (not in this tree):** PR #87 `rapidsai/quent` branch `nvtx`; NVIDIA/NVTX `tools/sample-injection`.
**Pattern extraction date:** 2026-07-13
</content>
</invoke>
