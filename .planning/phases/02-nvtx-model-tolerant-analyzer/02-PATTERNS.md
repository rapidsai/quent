# Phase 2: NVTX Model & Tolerant Analyzer - Pattern Map

**Mapped:** 2026-07-23
**Files analyzed:** 16 (5 Wave-0 capture-side modifications + 10 new-crate files + root Cargo.toml)
**Analogs found:** 16 / 16 (all in-repo; this is a copy-the-neighbor phase)

## Orientation

Two workstreams:

- **Wave 0 (capture-side, D-17):** narrow modification to the *existing* upstreamable
  NVTX capture crates to stamp `thread_id: u32` onto `RangePush`/`RangePop` (planner's
  call on `Mark`/`RangeStart`). Analogs are the surrounding lines of the very files
  being edited — copy the local convention exactly.
- **New crate `integrations/nvtx/analyzer` (D-06, framework-free):** a hand-written
  two-pass reconstruction core. **No analog crate exists** for the *reconstruction
  logic* (that is the point of D-06 — off `crates/analyzer`), so per-file analogs below
  map onto (a) the NVTX crates' *file-shape* conventions (SPDX, `//!` docs, `mod` +
  `pub use`, plain structs/enums with feature-gated derives, `thiserror` error enum) and
  (b) reusable runtime pieces (`Event<T>`, `TimeOrderedCollector`). The *algorithms*
  come from RESEARCH.md §"Two-Pass Handle Resolution" / §"Range Reconstruction
  Mechanics" / §"Tolerance by Construction", not from a code analog.

**Hard constraint (D-06 / Pitfall 3):** `crates/analyzer`, `crates/model`,
`quent-model`, `schema::Schema`, `RtFsmBuilder`, and the `model!`/`fsm!`/`entity!` DSL
must NOT appear in the new crate's `Cargo.toml` or `use` statements. `crates/analyzer`
is mapped below **only** as an anti-analog (what NOT to depend on) and as the source of
the one reusable, legacy-free type `TimeOrderedCollector` (which actually lives in
`crates/time`, verified).

## File Classification

### Wave 0 — capture-side modifications (existing crates)

| Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------|------|-----------|----------------|---------------|
| `integrations/nvtx/events/src/lib.rs` | model (vocabulary enum) | transform / event-driven | itself — `NameThread { thread_id: u32, .. }` variant already in this file | exact (same file) |
| `integrations/nvtx/injection/src/convert.rs` | utility (pure FFI→event conversion) | transform | `convert::range_pop` / `convert::name_thread` (same file) | exact (same file) |
| `integrations/nvtx/injection/src/callbacks.rs` | middleware (`extern "C"` FFI callback) | event-driven | `callbacks::on_domain_range_pop` / `on_name_os_thread_a` (same file) | exact (same file) |
| `integrations/nvtx/injection/src/init.rs` | utility (process/thread state) | request-response | `init::next_handle` + `RANGE_DEPTH` thread-local (same file) | exact (same file) |
| `integrations/nvtx/example/tests/capture.rs` | test | event-driven | itself (the coverage assertion) | exact (same file) |

### New crate — `integrations/nvtx/analyzer`

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `integrations/nvtx/analyzer/Cargo.toml` | config | — | `integrations/nvtx/bridge/Cargo.toml` (+ `events/Cargo.toml` for feature block) | role-match |
| `integrations/nvtx/analyzer/src/lib.rs` | module root | — | `integrations/nvtx/events/src/lib.rs` (`//!` + `mod` + `pub use`) | role-match |
| `integrations/nvtx/analyzer/src/error.rs` | error | — | `crates/analyzer/src/error.rs` (shape only) + `injection/src/init.rs` `InstallHookError` | role-match |
| `integrations/nvtx/analyzer/src/span.rs` | model (plain structs) | — | `integrations/nvtx/events/src/attributes.rs` (plain struct/enum + feature-gated derives) | role-match |
| `integrations/nvtx/analyzer/src/tables.rs` | service (pass-1 resolution) | transform / batch | `init.rs` `RANGE_DEPTH` HashMap idiom; algorithm from RESEARCH §Two-Pass | partial (algorithm from research) |
| `integrations/nvtx/analyzer/src/model.rs` | service (two-pass builder) | batch / transform | `NdjsonImporter` iterator + `capture.rs` replay loop | partial |
| `integrations/nvtx/analyzer/src/ranges.rs` | service (stack + handle match) | event-driven / transform | `init.rs::range_push_level`/`range_pop_level` per-`(thread,domain)` stack | role-match |
| `integrations/nvtx/analyzer/src/resource.rs` | service (lifespan + labels) | transform | `convert::read_resource` (field semantics); algorithm from RESEARCH §Resource | partial |
| `integrations/nvtx/analyzer/src/stats.rs` | service (aggregation) | batch / transform | none (straight fold); RESEARCH §Range Statistics | no code analog |
| `integrations/nvtx/analyzer/tests/*.rs` | test | event-driven | `integrations/nvtx/example/tests/capture.rs` | role-match |
| `Cargo.toml` (root, workspace) | config | — | root `Cargo.toml` NVTX `members` block (lines 27–31) | exact |

## Pattern Assignments

### Wave 0 file 1 — `integrations/nvtx/events/src/lib.rs` (model, MODIFY)

**Analog:** the same file — `NameThread` already carries the exact field to add.

**The id space to reuse** (`lib.rs:101-107`) — `NameThread` is the one existing variant
with a thread id; `RangePush`/`RangePop` must use the **same `u32` id space** (D-17: OS
`gettid()`), so named-thread grouping resolves against it:

```rust
/// `nvtxNameOsThread` — name an OS thread.
NameThread {
    /// Raw OS thread id.
    thread_id: u32,
    /// The thread's name.
    name: String,
},
```

**Variants to extend** (`lib.rs:36-47`) — add `thread_id: u32` with a doc line matching
the existing field-doc style (`/// Raw ...`). Keep field ordering `domain` then
`thread_id` then `attributes`:

```rust
/// `nvtxDomainRangePushEx` — open a nested (per-thread) range.
RangePush {
    /// Raw domain handle (`0` = default domain).
    domain: u64,
    /// Captured event attributes (message, color, category, payload).
    attributes: NvtxEventAttributes,
},
/// `nvtxDomainRangePop` — close the most recent push on this thread.
RangePop {
    /// Raw domain handle (`0` = default domain).
    domain: u64,
},
```

**Derive block to preserve** (`lib.rs:33-34`) — do not change; new field must impl the
same bounds:

```rust
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum NvtxEvent {
```

**Planner note (scope of the field):** RESEARCH OQ#1 / D-17 say `RangePush`/`RangePop`
are mandatory; `Mark`/`RangeStart` are optional (named-thread grouping for instants /
process-wide ranges). Adding the field is a breaking vocabulary change — every
`match`/construction site below must be updated in lockstep (compiler-enforced).

---

### Wave 0 file 2 — `integrations/nvtx/injection/src/convert.rs` (utility, MODIFY)

**Analog:** `convert::range_pop` (`convert.rs:33-36`) and `convert::name_thread`
(`convert.rs:129-133`), same file.

**Pattern to copy — the pure conversion fn** (`convert.rs:33-36`, the smallest one):

```rust
/// Convert a `DomainRangePop` call to a verbatim [`NvtxEvent::RangePop`].
pub(crate) fn range_pop(domain: u64) -> NvtxEvent {
    NvtxEvent::RangePop { domain }
}
```

These fns are **pure and side-effect-free by design** (`convert.rs` module doc:
"Everything here is deterministic and touches no global state"). Therefore the OS thread
id must be **passed in as a parameter**, not read here — the thread read belongs in the
callback (file 3). New signature shape: `range_pop(domain: u64, thread_id: u32)` →
`NvtxEvent::RangePop { domain, thread_id }`; same for `range_push`.

**Test convention in this file to extend** (`convert.rs:549-564` module header + the
per-variant `let NvtxEvent::X { .. } = ... else { panic!(...) }` assertion style at
`convert.rs:605-631`). Every convert fn has a matching `#[test]`; add/adjust the
`thread_id` assertion the same way.

---

### Wave 0 file 3 — `integrations/nvtx/injection/src/callbacks.rs` (middleware/FFI, MODIFY)

**Analog:** `callbacks::on_domain_range_pop` (`callbacks.rs:52-60`) — the exact callback
that must now read the thread id.

**Pattern to copy — callback body with the panic barrier** (`callbacks.rs:52-60`):

```rust
pub(crate) extern "C" fn on_domain_range_pop(domain: nvtxDomainHandle_t) -> c_int {
    let domain = domain as usize as u64;
    let mut level: c_int = 0;
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        level = init::range_pop_level(domain);
        init::dispatch(convert::range_pop(domain));
    }));
    level
}
```

**Where the thread id is captured:** the callback runs on the app thread (RESEARCH:
"injection callbacks already run on the app thread"). Read the OS thread id here —
either *before* `catch_unwind` (like `range_id`/`handle` are synthesized before the
guard at `callbacks.rs:85`, `104`, `127` so they survive a panic) or inside it, and pass
it to `convert::range_pop(domain, thread_id)`. All Push/Pop callbacks to touch:
`on_domain_range_pop` (:52), `on_domain_range_push_ex` (:32), and the default-domain CORE
mirrors `on_range_pop` (:267), `on_range_push_ex` (:242), `on_range_push_a` (:255).

**Precedent — value synthesized outside the guard so it survives a panic**
(`callbacks.rs:81-93`, the `range_id` pattern; apply the same discipline to the thread
id if it must be returned/used post-panic):

```rust
let range_id = init::next_handle();
let _ = std::panic::catch_unwind(|| {
    let event = unsafe { convert::range_start(domain as usize as u64, range_id, attr) };
    init::dispatch(event);
});
range_id
```

---

### Wave 0 file 4 — `integrations/nvtx/injection/src/init.rs` (utility, MODIFY — add a thread-id helper)

**Analog:** `init::next_handle` (`init.rs:36-39`) — the shape for a small pub(crate)
process helper; and `RANGE_DEPTH` (`init.rs:41-51`), which proves the correct grain is
per-`(thread, domain)`.

**Pattern to copy — a tiny `pub(crate)` helper** (`init.rs:36-39`):

```rust
/// Return a fresh, process-unique, nonzero handle/id.
pub(crate) fn next_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}
```

**What to add:** a `current_thread_id() -> u32` returning the OS thread id in the same
space `NameThread` uses (Linux `gettid()`). `nvtx-injection`'s only deps are
`nvtx-events` + `thiserror` (`injection/Cargo.toml:15-17`) — no `libc` yet. Planner:
either add `libc` (workspace dep — verify it is in `[workspace.dependencies]` first) for
`libc::gettid`, or use a `syscall`/`std::thread` fallback. Keep it a single small helper
so the blast radius stays "2–4 variants + convert/callbacks" (D-17).

**Grain confirmation to honor** (`init.rs:41-51`) — the injection layer already keys
nesting by `(thread, domain)`; the analyzer's `ranges.rs` stacks must use the same grain:

```rust
thread_local! {
    /// Per-thread, per-domain count of currently-open push/pop ranges.
    static RANGE_DEPTH: std::cell::RefCell<std::collections::HashMap<u64, i32>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}
```

---

### New file — `integrations/nvtx/analyzer/Cargo.toml` (config, CREATE)

**Analog:** `integrations/nvtx/bridge/Cargo.toml` (path-dep style + SPDX header) and
`integrations/nvtx/events/Cargo.toml` (optional-serde `[features]` block).

**SPDX + package header to copy** (`bridge/Cargo.toml:1-8`):

```toml
# SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

[package]
name = "nvtx-analyzer"
version.workspace = true
edition.workspace = true
publish.workspace = true
```

**Dependency style to copy** (`bridge/Cargo.toml:10-15`) — path deps for in-repo crates,
`{ workspace = true }` for shared:

```toml
[dependencies]
nvtx-events = { path = "../events" }
nvtx-bridge = { path = "../bridge" }
quent-events = { path = "../../../crates/events" }
quent-time  = { path = "../../../crates/time" }
thiserror = { workspace = true }
tracing   = { workspace = true }
# uuid / serde / serde_json / rustc-hash as needed (all in [workspace.dependencies])
```

**Forbidden deps (D-06 / Pitfall 3):** no `quent-analyzer`, `quent-model`,
`quent-schema`, `quent-model-macros`. **Verified reusable, legacy-free:** `quent-time`
(`TimeOrderedCollector` lives there — see below), `quent-events`.

**`[dev-dependencies]` for the happy-path test** (mirror `example/Cargo.toml:27-29` and
the `capture.rs` needs): `nvtx-example` (real capture), `quent-instrumentation` with
`io-callback`, `uuid`. If reading back ndjson instead, add `quent-io-ndjson`.

---

### New file — `integrations/nvtx/analyzer/src/lib.rs` (module root, CREATE)

**Analog:** `integrations/nvtx/events/src/lib.rs:1-26` — SPDX, crate `//!` doc, private
`mod`s, and a single `pub use` re-export surface.

**Shape to copy** (`events/lib.rs:1-23`):

```rust
// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! <crate-level //! doc: hand-written, framework-free NVTX reconstruction core>

mod error;
mod model;
mod ranges;
mod resource;
mod span;
mod stats;
mod tables;

pub use error::{NvtxModelError, NvtxModelResult};
pub use model::{NvtxModel, NvtxModelBuilder};
pub use span::{NvtxMark, NvtxSpan, SpanKind};
// ... re-export the query surface (stats, domains/threads/categories)
```

Every `lib.rs` in the repo carries a crate-level `//!` doc (CLAUDE.md "Module Design").

---

### New file — `integrations/nvtx/analyzer/src/error.rs` (error, CREATE)

**Analog:** `crates/analyzer/src/error.rs:1-33` for the `thiserror` enum *shape* (import
map onto only — do NOT depend on that crate), and `injection/src/init.rs:84-90` for the
local minimal-enum idiom.

**Pattern to copy — `thiserror` enum + `#[from]` + inline messages** (`crates/analyzer/src/error.rs:1-14`):

```rust
// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnalyzerError {
    #[error("importer error: {0}")]
    Importer(#[from] quent_model::io::ImporterError),
    #[error("validation error: {0}")]
    Validation(String),
    // ...
}
```

**Required additions (CLAUDE.md "Error Handling"):** add the per-crate result alias
`pub type NvtxModelResult<T> = std::result::Result<T, NvtxModelError>;`. Note: tolerance
is **by construction** (D-12) — most anomalies are `tracing::warn!`-and-continue, NOT
`Err`, so this enum will be small (e.g. an importer/deserialize error for the test-replay
path). Do not model tolerated anomalies as error variants.

---

### New file — `integrations/nvtx/analyzer/src/span.rs` (model, CREATE)

**Analog:** `integrations/nvtx/events/src/attributes.rs:21-57` — plain structs/enums with
doc comments and feature-gated derives; this is exactly the "own plain span type" D-08
mandates, in the same house style.

**Pattern to copy — plain enum + struct with feature-gated serde** (`attributes.rs:21-57`):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum NvtxMessage {
    String(String),
    RegisteredHandle(u64),
}

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct NvtxEventAttributes {
    pub category: u32,
    pub color: Option<NvtxColor>,
    pub message: Option<NvtxMessage>,
    pub payload: Option<NvtxPayload>,
}
```

**Target shape (from RESEARCH §Model Representation, lines 311-329)** — `NvtxSpan`,
`NvtxMark`, `NvtxDomain`, `NvtxThread`, `SpanKind { PushPop, StartEnd, Resource }`. Reuse
verbatim vocabulary types by **re-exporting** `nvtx_events::{NvtxColor, NvtxPayload}` on
span fields (D-03: carry `color`/`payload` verbatim; carry `thread_id: Option<u32>`,
`domain`, precise `start`/`end`, `synthetic_end: bool` per D-13). Timestamps are
`quent_time::TimeUnixNanoSec` (= `u64`, `crates/time/src/lib.rs:34`).

---

### New file — `integrations/nvtx/analyzer/src/tables.rs` (service — pass 1, CREATE)

**Analog (idiom):** `init.rs:41-63` `HashMap` keyed accumulation. **Algorithm:**
RESEARCH §"Two-Pass Handle Resolution" table (lines 234-264) — no code analog, this is
new logic.

**Keying rules to implement exactly (Pitfall 2 & 4):**
- domain names: key `domain: u64` ← `DomainCreate`
- registered strings: key `(domain: u64, handle: u64)` ← `RegisterString` (**never bare
  handle**)
- categories: key `(domain: u64, category: u32)` ← `NameCategory` (**never global**)
- thread names: key `thread_id: u32` ← `NameThread`
- resources: key `handle: u64` alone ← `ResourceCreate` (**not `(domain, handle)`** —
  `ResourceDestroy` carries no domain, `lib.rs:121-125`)

**Placeholder policy (D-14, success criterion 2) — RESEARCH lines 252-264:** pure
functions of the raw id, no counters/timestamps. Implement as `const`/format helpers and
assert exact strings in tests: `"default domain"`, `"thread {id}"`,
`"<domain 0x{domain:X}>"`, `"<unregistered string 0x{h:X}>"`,
`"<category {category} @ domain 0x{domain:X}>"`.

---

### New file — `integrations/nvtx/analyzer/src/model.rs` (service — two-pass builder, CREATE)

**Analog (replay/consume shape):** the `capture.rs` collect-and-match loop
(`example/tests/capture.rs:36-54`) and `NdjsonImporter`'s `Iterator<Item = Event<T>>`
(`crates/io/ndjson/src/lib.rs:135-161`) — both are the *input contract* the builder
consumes.

**Input element to consume** (`capture.rs:42-46`) — how a captured event is pulled apart;
the builder reads `event.timestamp` + `event.data.0` (the inner `NvtxEvent`):

```rust
EventCallback::new(move |recorded| {
    if let Some(event) = recorded.event.downcast_ref::<Event<NvtxEventEntity>>() {
        collected.lock().unwrap().push(event.data.0.clone());
    }
})
```

**Builder API shape (CLAUDE.md conventions):** fallible constructor `try_new`, options
struct not long param lists, `NvtxModelResult` return. Two-pass (D-15): `fn build(events:
impl IntoIterator<Item = Event<NvtxEventEntity>>) -> NvtxModelResult<NvtxModel>` — pass 1
fills `tables.rs`, pass 2 replays timestamp-sorted (see `stats`/`ranges`). Keep the
`Event<T>` envelope (`crates/events/src/lib.rs:19-46`) as the boundary type; `Event: 
Timestamp` (`:48-52`) is what feeds the collector below.

---

### New file — `integrations/nvtx/analyzer/src/ranges.rs` (service — stacks + matching, CREATE)

**Analog:** `init.rs::range_push_level` / `range_pop_level` (`init.rs:56-82`) — the
per-`(thread, domain)` push/pop nesting model, the exact grain ANA-03 must reconstruct.

**Pattern to copy — per-`(thread,domain)` stack discipline** (`init.rs:56-82`):

```rust
pub(crate) fn range_push_level(domain: u64) -> c_int {
    RANGE_DEPTH.with(|depth| {
        let mut map = depth.borrow_mut();
        let level = map.entry(domain).or_insert(0);
        let started = *level;
        *level += 1;
        started
    })
}
```

The analyzer version keys a `Vec<OpenSpan>` stack by `(thread_id, domain)` (thread_id now
present post-Wave-0), pushing on `RangePush`, popping innermost on `RangePop`, recording
`parent` at pop (RESEARCH §Nested-range representation, lines 291-303).

**RangeStart/End (ANA-04, no gap)** — `HashMap<u64 range_id, OpenStartRange>`: insert on
`RangeStart`, remove+form span on `RangeEnd` (RESEARCH lines 284-289). Orphan `End` →
`warn!` + skip. Match by `range_id` alone (globally unique, `init.rs:34-39`).

---

### New file — `integrations/nvtx/analyzer/src/resource.rs` (service, CREATE)

**Analog (field semantics):** `convert::read_resource` (`convert.rs:459-494`) and the
`ResourceCreate`/`ResourceDestroy` variants (`lib.rs:108-125`) — shows the raw fields the
analyzer must interpret (`identifier_type: i32`, `identifier: u64`, `message`).

**Rules (D-09/D-10/D-11, RESEARCH lines 336-349):** match `Create→Destroy` by `handle`
alone (Pitfall 4); resolve `message` via the string table; `match identifier_type` →
static `&str` for the **core/generic** set, pass unknown through as
`"<identifier_type {n}>"`. Do NOT fabricate capacity/occupancy (D-10). A `NvtxResource`
is structurally an `NvtxSpan { kind: Resource, .. }`.

**Open item (OQ#3 / A1):** exact core `identifier_type` numeric values are `[ASSUMED]` —
planner confirms against the vendored NVTX headers used by
`integrations/nvtx/injection/build.rs` (bindgen) before hardcoding labels.

---

### New file — `integrations/nvtx/analyzer/src/stats.rs` (service — aggregation, CREATE)

**Analog:** none (no aggregation analog in the NVTX vertical); straight fold per RESEARCH
§"Range Statistics" (lines 388-398).

**Rules:** group completed spans by `(resolved name, domain, category)`; per group
`count`, `total_duration`, `avg = total/count` (guard `count == 0`), `min`, `max`.
`duration = end - start` (`u64` ns); zero-duration spans contribute `0` (clamp `end =
max(end, start)` — RESEARCH lines 366-368). Include Push/Pop + Start/End spans; exclude
marks; resources reported separately or excluded. Keep the `synthetic_end` contribution
distinguishable (OQ#2).

---

### New file — `integrations/nvtx/analyzer/tests/*.rs` (test, CREATE)

**Analog:** `integrations/nvtx/example/tests/capture.rs:1-68` — the integration-test
fixture and coverage-assertion pattern, entirely reusable.

**Happy-path pattern to copy** (`capture.rs:36-67`) — reuse `nvtx_example::run_capture`
with a collecting `EventCallback`, then assert reconstructed labels:

```rust
let collected: Arc<Mutex<Vec<NvtxEvent>>> = Arc::new(Mutex::new(Vec::new()));
let sink = { /* EventCallback that downcasts Event<NvtxEventEntity> and pushes data.0 */ };
nvtx_example::run_capture(Uuid::now_v7(), sink).expect("capture");
// feed collected events (as Event<NvtxEventEntity>) into NvtxModelBuilder, assert model
```

**Synthetic-fixture pattern (malformed cases)** — build `Vec<Event<NvtxEventEntity>>`
with `Event::new(id, ts, NvtxEventEntity(..))` (`crates/events/src/lib.rs:38-45`) to
control timestamps exactly. Cases from RESEARCH §Test Fixture Strategy (lines 448-458):
unclosed Push, orphan Pop, unmatched Start/orphan End, out-of-order, duplicate
timestamps, referenced-but-unresolved placeholders, forward references. Multi-thread
Push/Pop fixtures become authorable only after Wave 0 lands the `thread_id` field.

**Run command (crate is `members`-only, not `default-members`):** `cargo test -p
nvtx-analyzer` — bare `cargo test` skips it (RESEARCH §Validation).

---

### New file — root `Cargo.toml` (config, MODIFY)

**Analog:** the existing NVTX `members` block, root `Cargo.toml:27-31`.

**Pattern to copy** (root `Cargo.toml:27-31`):

```toml
    # NVTX integration crates
    "integrations/nvtx/events",
    "integrations/nvtx/injection",
    "integrations/nvtx/bridge",
    "integrations/nvtx/example",
```

Add `"integrations/nvtx/analyzer"` to `[workspace] members` **only**. Do NOT add it to
`default-members` (root `Cargo.toml:79-115`) — the NVTX crates are deliberately absent
there, preserving the zero-cost-default guarantee (RESEARCH: "VERIFIED root
`Cargo.toml:28-31,79-115`"). No external `cargo add`; all deps are path crates or
existing `[workspace.dependencies]`.

## Shared Patterns

### SPDX license header (mandatory, every source file)
**Source:** every `.rs` / `.toml` in the repo, e.g. `integrations/nvtx/events/src/lib.rs:1-2`
**Apply to:** all new crate files.
```rust
// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0
```
(`.toml` files use `#` comment form — `bridge/Cargo.toml:1-2`.)

### Timestamp-ordered replay (tolerance, D-12 — ANA-05)
**Source:** `crates/time/src/lib.rs:121-165` `TimeOrderedCollector<T>` — **VERIFIED it
lives in `crates/time`, not `crates/analyzer`**, so it is importable under D-06 (resolves
RESEARCH Assumption A3).
**Apply to:** `model.rs` pass-2 replay ordering.
```rust
pub struct TimeOrderedCollector<T>(Vec<T>);
impl<T> TimeOrderedCollector<T> where T: Timestamp {
    pub fn push(&mut self, state: T) {
        if let Some(last) = self.0.last() && last.timestamp() <= state.timestamp() {
            self.0.push(state);                     // O(1) in-order fast path
        } else {
            let pos = self.0.partition_point(|s| s.timestamp() < state.timestamp());
            self.0.insert(pos, state);              // stable insert for late arrivals
        }
    }
    pub fn into_inner(self) -> Vec<T> { self.0 }
}
```
`Event<T>: Timestamp` already (`crates/events/src/lib.rs:48-52`), so `Event<NvtxEventEntity>`
plugs straight in. Duplicate timestamps preserve arrival order (partition_point on `<`),
satisfying the "deterministic, no panic" requirement without a hand-rolled sort.

### `thiserror` error enum + `Result` alias (per-crate)
**Source:** `crates/analyzer/src/error.rs:7-33` (shape) + CLAUDE.md "Error Handling".
**Apply to:** `error.rs`. `#[derive(Debug, Error)]`, `#[error("...")]` inline messages,
`#[from]` for wrapped errors, plus `pub type NvtxModelResult<T> = Result<T, NvtxModelError>;`.

### `tracing::warn!` for tolerated anomalies (not errors)
**Source:** CLAUDE.md "Logging" (inline capture `{e}`); `crates/io/ndjson/src/lib.rs:94,150`
`warn!`/`error!` in a non-propagating path.
**Apply to:** `ranges.rs`, `resource.rs`, `model.rs` — orphan pop/end/destroy, unclosed
range synthetic-close (D-13). Anomalies are logged-and-continued, never returned as `Err`
(D-12). Note the capture-side cdylib uses `eprintln!` (no subscriber, `convert.rs:439-448`);
the analyzer is a normal library and should use `tracing`.

### Feature-gated serde derives on model types
**Source:** `integrations/nvtx/events/src/lib.rs:33-34`, `attributes.rs:46-47`,
`events/Cargo.toml:10-15`.
**Apply to:** `span.rs` model types if they need to serialize (planner's call; the model
is consumed in-process by Phase 3, so serde may be unnecessary — but the input
`NvtxEventEntity` requires the `nvtx-events` default `serde` feature to deserialize test
files).
```rust
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
```

### Fallible constructor `try_new` + options struct
**Source:** `crates/io/ndjson/src/lib.rs:45,123` `try_new`; `NdjsonExporterOptions` /
`Context::try_new` (CLAUDE.md "Function Design").
**Apply to:** `model.rs` builder entry point and any resource-owning type.

## No Analog Found

No file is left without at least a shape-analog. The items below have **no code analog
for their core logic** — they are new algorithms specified by RESEARCH, and the planner
must lift the algorithm from the cited research section, not from a neighbor file:

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/tables.rs` | service | batch | Two-pass handle-resolution tables are new; only the `HashMap` idiom (`init.rs`) is analogous. Algorithm: RESEARCH lines 234-264. |
| `src/stats.rs` | service | batch | No aggregation exists in the NVTX vertical. Straight fold: RESEARCH lines 388-398. |
| `src/resource.rs` | service | transform | `Create→Destroy` lifespan matching + `identifier_type` labeling is new logic; only field semantics (`convert::read_resource`) are analogous. |

## Anti-Analogs (map onto for structure ONLY, never depend on)

| File | Why referenced | Prohibition |
|------|----------------|-------------|
| `crates/analyzer/src/error.rs` | `thiserror` enum *shape* | Do NOT add `quent-analyzer`/`quent-model` deps (its variants wrap `quent_model::io::ImporterError`, `quent_time::TimeError`). |
| `crates/analyzer/src/fsm/runtime.rs` | Panic paths D-12 avoids (`RtFsmBuilder::try_build` `unwrap`; `RtFsmsBuilder` `?`-abort) | Never construct `RtFsmBuilder`/`RtFsm`; never `use quent_analyzer::*`. Own span type instead (D-08). |
| `domains/query_engine/*` | Directory shape (`model`/`analyzer`/`server`/`ui` quartet) | Do NOT mirror — it is built on the rejected `model!`/`fsm!`/`entity!` DSL + `crates/analyzer` (D-05/D-06). Crate goes in `integrations/nvtx/analyzer`, not `domains/nvtx/`. |

## Metadata

**Analog search scope:** `integrations/nvtx/{events,bridge,injection,example}`,
`crates/{events,time,io/ndjson,analyzer}`, root `Cargo.toml`.
**Files scanned (read in full or targeted):** 13 source/config files.
**Pattern extraction date:** 2026-07-23
**Key verifications performed this session:**
- `TimeOrderedCollector` is in `crates/time` (importable under D-06) — resolves RESEARCH A3.
- Root `Cargo.toml`: NVTX crates in `members` (27–31), absent from `default-members` (79–115).
- `nvtx-injection` deps are only `nvtx-events` + `thiserror` — a thread-id syscall helper
  needs a new dep decision (planner).
</content>
</invoke>
