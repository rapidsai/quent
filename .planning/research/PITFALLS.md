# Pitfalls Research

**Domain:** NVTX injection/consumer libraries + fan-out mediation feeding an existing telemetry pipeline (Quent)
**Researched:** 2026-07-08
**Confidence:** MEDIUM-HIGH (NVTX injection semantics verified against NVIDIA docs; Quent-specific hazards sourced from `.planning/codebase/CONCERNS.md` at HIGH confidence; some FFI/lifetime details are training-data + header-inference at MEDIUM)

This document catalogs domain-specific mistakes for building an NVTX injection library, a multi-consumer fan-out mediator, and the model/analyzer/endpoint/UI slice that turns captured NVTX into Quent traces. Phase names below are thematic; they map to the natural build sequence in `PROJECT.md` (injection base → payload capture → fan-out → model/pipeline → analyzer → endpoint/UI → test app/validation).

## Critical Pitfalls

### Pitfall 1: Events emitted before the hook is installed are silently dropped

**What goes wrong:**
NVTX resolves its function table exactly once, lazily, at the first NVTX call (or at `nvtxInitialize`). The injection library's `InitializeInjectionNvtx2` is invoked at that moment. Any NVTX call the application makes *before* NVTX initializes with our injection present — or any registered-string registration, domain creation, or category naming done during static initialization of a linked-in GPU library — happens before our callback wiring is live, or worse, gets bound to the no-op default implementation. Those events vanish with no error. Downstream the analyzer then sees ranges referencing domain/string handles whose *registration events were never captured*, producing "unknown handle" gaps that look like analyzer bugs.

**Why it happens:**
Developers assume "install the hook, then all events flow." But NVTX's one-shot table resolution means install order relative to the app's first NVTX touch is everything. Static-initializer NVTX calls in a dependency (libcudf registering domains/strings at load) run before `main`, before any Rust-side setup the app author controls.

**How to avoid:**
- Ensure the injection library is discoverable *before the first NVTX call*: set `NVTX_INJECTION64_PATH` (or the strong-symbol override linked in) so NVTX finds us at its own init, not after app startup logic runs.
- Treat handle *registration* events as potentially-missing by design: the analyzer must resolve handles from whatever registration events it saw and degrade gracefully (render the raw handle id / "unregistered string #N") rather than panic or drop the range.
- In the injection init path, do the absolute minimum and make it idempotent; never assume "install" happens at a point you control.

**Warning signs:**
Ranges with domain/string handle ids that have no corresponding registration event; counts of captured "register string" events far lower than distinct handles referenced; events appearing only after a certain wall-clock point in every run.

**Phase to address:** Injection base (capture correctness); Analyzer reconstruction (graceful unknown-handle handling)

---

### Pitfall 2: Doing real work in the callback distorts the very application being measured

**What goes wrong:**
NVTX callbacks execute *synchronously on the application's own threads* — NVIDIA's own guidance is explicit that once a tool is present, every NVTX call "jumps directly into that tool's implementation, with overhead entirely determined by what the tool does." If our callback allocates, takes a lock, serializes to msgpack/postcard, or (worst) does blocking I/O or an `await` that parks, we inflate the timing of the exact ranges we are measuring. The measurement becomes self-referential noise: a 2µs range reads as 40µs because our callback ran inside it.

**Why it happens:**
The instinct is to reuse Quent's existing `EventSender`/exporter path directly from the callback. That path was built for an app that *chooses* when to emit; here we are on the hot path of an unwitting third-party library at whatever call frequency it emits (libcudf can emit thousands of ranges per operator).

**How to avoid:**
- Callback does only: capture raw fields into a pre-sized, lock-free/SPSC or per-thread buffer, and return. No allocation on the steady-state path (reuse thread-local buffers / arenas), no string copying beyond what's unavoidable, no serialization, no locks that can contend.
- Move serialization + `EventSender` handoff to a dedicated drain thread, decoupled from app threads.
- Copy `const char*` message payloads *only when the message is not a registered handle* (see Pitfall 4) — and do that copy cheaply.
- Benchmark callback cost with a tight NVTX push/pop microbenchmark and hold it to a low fixed budget (target sub-microsecond steady state).

**Warning signs:**
Measured range durations change materially when Quent is attached vs. not; per-thread CPU rises under Quent; flamegraph shows time in our callback/allocator/serializer; range durations correlate with emission rate.

**Phase to address:** Hot-path capture (injection base)

---

### Pitfall 3: Unbounded buffering vs. backpressure — both corrupt the measurement, differently

**What goes wrong:**
Two failure modes at the capture→pipeline boundary:
1. **Backpressure into the app.** If the drain path uses a bounded channel that blocks when full, a slow consumer (collector, disk) stalls the application's threads at NVTX-call time. Quent's *existing* collector client already has exactly this hazard: bounded mpsc(1024) where `Client::send` awaits when the collector is slow (`crates/collector/client/src/lib.rs:95-101`), plus a fixed 42×1s connect-retry loop (`:75-92`). Feeding NVTX capture straight into that path means the collector being slow silently freezes libcudf.
2. **Unbounded buffering.** Switching to an unbounded channel to avoid stalling instead lets memory grow without limit under bursty high-rate emission, eventually OOMing the instrumented app — a worse outage than dropped data.

**Why it happens:**
"Just make the channel bigger / unbounded" is the reflexive fix for backpressure, trading a stall for an OOM. The collector client's own TODO literally says "consider unbounded" without resolving the policy.

**How to avoid:**
- Choose an explicit drop policy for the capture buffer: bounded ring with **drop-newest or drop-oldest and a dropped-event counter** emitted as telemetry, so data loss is visible and quantified rather than a hang.
- Never let the app thread block on downstream slowness. The capture→drain boundary must be non-blocking on the producer side.
- Keep the drain→`EventSender`→collector path off the app threads entirely, and size the hop-to-hop buffers with backpressure absorbed by *dropping*, not *blocking*, on the NVTX side.
- Emit a "captured N, dropped M" summary at flush/exit so runs with loss are self-documenting.

**Warning signs:**
App hangs or stutters when the collector/disk is slow; RSS of the instrumented process climbs monotonically under load; no visibility into whether events were dropped.

**Phase to address:** Hot-path capture (injection base) — decide drop policy before wiring into `EventSender`; revisit the collector-client backpressure TODO as part of this.

---

### Pitfall 4: C-string and registered-string pointer lifetime — use-after-free from the callback

**What goes wrong:**
The `const char*`/`const wchar_t*` message in `nvtxEventAttributes_t` (and args to `nvtxMarkA`, `nvtxRangePushA`, etc.) is owned by the *caller* and is only guaranteed valid *for the duration of the call*. If the callback stashes the raw pointer into a buffer to serialize later on the drain thread, the app may free or reuse that memory before the drain runs → garbage strings, or a segfault *inside the instrumented application* that looks like an app bug. The reverse mistake also bites: **registered strings** (`nvtxDomainRegisterStringA` → handle) are the opposite — the tool is meant to record the string *once at registration* and thereafter receive only the *handle*; copying the string on every range that uses a handle is wasted hot-path work and defeats the purpose registered strings exist for.

**Why it happens:**
FFI captures store pointers naively; the "valid only during the call" contract isn't enforced by the type system across the C boundary. And the registered-string indirection (handle now, string earlier) is easy to miss, leading to either dereferencing a handle as if it were a `char*` or redundant copying.

**How to avoid:**
- For **immediate** (non-registered) message strings: copy the bytes into owned storage *inside the callback, before returning*. Never store the raw C pointer for later use.
- For **registered** strings: capture the *registration event* (handle ↔ string, plus its domain) as its own Quent event, and on ranges store only the handle id. Resolve handle→string in the analyzer.
- Handle `wchar_t` explicitly: on Linux `wchar_t` is 4 bytes (UTF-32); the Unicode (`...W`) APIs differ from ASCII (`...A`). Decide whether to support the wide APIs and convert deterministically, or document them as unsupported (Windows is already out of scope, which removes the 2-byte-`wchar_t` case, but the `...W` entry points still exist on Linux).
- Validate the `version`/`size` fields of `nvtxEventAttributes_t` before reading later members; a struct authored against a newer NVTX version may carry fields we must not read, and an older one may be shorter than we expect.

**Warning signs:**
Intermittent garbled message text; crashes originating inside the instrumented app only when Quent is attached; strings that "sometimes" come through empty; sanitizer (ASAN) use-after-free reports in the drain path.

**Phase to address:** Injection base (immediate-string copy, ABI/version guarding); Payload/handle capture (registration events); Analyzer reconstruction (handle resolution)

---

### Pitfall 5: Struct-version / ABI drift across NVTX versions

**What goes wrong:**
`nvtxEventAttributes_t`, payload-schema structs, and the injection interface (`InitializeInjectionNvtx2` vs the older `...Nvtx`) all carry explicit `version`/`size` fields precisely because the layout evolves. If we bindgen against one NVTX header version and the instrumented app is compiled against another, blindly reading fixed offsets past `size` reads uninitialized or foreign memory. The injection init function also has a versioned contract (v1 vs v2) and thread-safety guarantees that changed across releases.

**Why it happens:**
Bindgen freezes one header snapshot; developers treat the generated struct as canonical and read all fields unconditionally. The header we bind (via the `nvtx-sys` git dep in PR #87) may not match the runtime NVTX the app statically embeds.

**How to avoid:**
- Always branch on the incoming `version`/`size` fields; only read members that `size` says are present.
- Pin and document the NVTX header version bindgen runs against; treat it as the *minimum* contract and forward-tolerate newer.
- Prefer the `Nvtx2` injection entry point; verify the returned function-table version.
- Add a compatibility test that feeds attribute structs with older/newer `size` values and asserts we don't over-read.

**Warning signs:**
Field values that are nonsensical only for apps built against a different CUDA/NVTX version; occasional garbage in category/color/payload fields; crashes tied to a specific app's NVTX version.

**Phase to address:** Injection base

---

### Pitfall 6: Symbol-collision / weak-strong init hazards and static-initialization-order

**What goes wrong:**
The Linux mechanism relies on our providing a **strong** `InitializeInjectionNvtx2` that overrides NVTX's **weak** default. Get this wrong and either (a) our symbol doesn't win (two weak defs, or ours also weak → app runs with no injection, silently), or (b) it collides with another injection provider linked into the same image (e.g. if the app also links CUPTI or another tool statically) → duplicate strong symbol, link error or undefined-behavior "whoever the linker picked." Compounding it: our own Rust/global capture state initialized in a static initializer may run *after* NVTX calls our init function, so the callback fires against half-constructed state.

**Why it happens:**
Weak/strong linkage is subtle and platform-specific; it "works on my machine" with one link order and breaks under another. Static init order across TUs/crates is unspecified.

**How to avoid:**
- Make the override symbol unambiguously strong via the C shim (PR #87's approach), and prefer the `NVTX_INJECTION64_PATH` runtime path where possible so linkage order is irrelevant.
- Keep injection-init side effects minimal and lazy: init function records the callback subscription table; heavy state is constructed lazily and guarded (e.g. `OnceCell`), so a callback that races init sees a consistent view or a safe empty one.
- Guard against double-init explicitly (idempotent `install_hook`), because NVTX made these init APIs thread-safe but *our* one-shot state must be too.
- Document that Quent's injection and any other in-process strong-symbol injector are mutually exclusive *except through our fan-out* (see Pitfall 7).

**Warning signs:**
"Works when I link this way" fragility; no events at all in some builds; duplicate-symbol link errors when combined with CUPTI/other tools; crashes only at process start.

**Phase to address:** Injection base (symbol/linkage); Fan-out (coexistence with other injectors)

---

### Pitfall 7: Fan-out — the single-subscriber invariant, sink registration races, and one bad sink taking down all

**What goes wrong:**
NVTX allows **exactly one injection per process**. The fan-out mediator claims that slot and re-broadcasts to N sinks (Quent + whatever `NVTX_INJECTION64_PATH` points to, e.g. nsys/AON). Failure modes:
- **Passthrough tool expects to be sole subscriber.** nsys assumes it *is* the injection: it may register its own domains, expect exclusive ownership of the function table, or re-enter NVTX. Handing it a shadow table it doesn't know is shared can break its correlation or double-initialize it.
- **Double-initialization of the external tool** — calling its `InitializeInjectionNvtx2` twice, or after we've already populated the global table, yields undefined behavior in that tool.
- **Sink registered after init.** A sink attaching after the first NVTX call misses all prior events (same class as Pitfall 1) and, if the mediator's sink list isn't concurrency-safe, racing registration against in-flight callbacks corrupts the list.
- **One slow/panicking sink stalls or crashes the shared callback.** Because all sinks run in the one app-thread callback, a sink that blocks backpressures the app (Pitfall 3 × N), and a sink that panics/segfaults takes down the application for *every* consumer, not just itself.

**Why it happens:**
The fan-out design (Johan's shadow-table sketch / Lawrence's walked-handler list) was never prototyped. Multiplexing a single-owner C API to multiple independently-authored consumers is inherently a "make it look like sole ownership to each" problem, which is easy to underestimate.

**How to avoid:**
- Treat the passthrough external tool as a first-class sink initialized **exactly once**, at mediator init, forwarding it the injection handshake it expects — give it its own shadow function table so it believes it owns the slot.
- Freeze the sink set at init where possible; if dynamic registration is needed, use a copy-on-write / RCU-style sink list so callbacks never see a torn list, and accept that late sinks miss earlier events (document it).
- **Isolate sinks from each other:** each sink's callback failure must not propagate. Catch panics at the Rust FFI boundary (`catch_unwind` — a Rust panic unwinding across the C ABI into the app is itself UB); for a foreign (C) passthrough sink you cannot catch its segfault, so document that a crashing external tool is outside our containment and test the common ones (nsys) explicitly.
- Apply Pitfall 3's non-blocking rule per sink: no sink may block the shared callback.
- Provide a mode to run Quent *alone* (no passthrough) so fan-out complexity is opt-in and the common single-consumer path is simple and well-tested first.

**Warning signs:**
nsys traces come out wrong or empty only when Quent's mediator is present; app crashes/hangs correlate with a particular sink; events missing for late-attached sinks; intermittent corruption under concurrent sink registration.

**Phase to address:** Fan-out mediator — this is the highest-risk, least-prototyped phase; flag it for deep, dedicated research/prototyping before committing the design.

---

### Pitfall 8: Trace reconstruction inherits the analyzer's panics on imperfect telemetry

**What goes wrong:**
Real NVTX streams are *routinely* imperfect: ranges left unclosed at process exit, RangeStart on one thread with RangeEnd on another (matched only by correlation id), out-of-order timestamps across threads, duplicate timestamps. Quent's analyzer today (per CONCERNS.md) **panics** on exactly these: `SpanUnixNanoSec::try_new(start,end).unwrap()` assumes time-ordered transitions (`crates/analyzer/src/fsm/mod.rs:125-135`), and *one* incomplete FSM fails the whole engine build (`crates/analyzer/src/fsm/runtime.rs:309-313`). Modeling NVTX ranges as single-state FSMs and pushing them through this analyzer means a single unclosed libcudf range or one out-of-order pair aborts the entire trace analysis.

**Why it happens:**
The analyzer was built for well-formed, complete, mostly-ordered simulator telemetry. NVTX from a real, abruptly-terminated GPU workload violates every one of those assumptions, and the panics are `unwrap`s the happy-path tests never exercise.

**How to avoid:**
- Before/while wiring NVTX into the analyzer, convert the span-construction `unwrap`s on the NVTX path to error propagation (`AnalyzerError`), and bucket incomplete FSMs separately instead of failing the build (the CONCERNS TODO already prescribes this).
- Define explicit reconstruction rules: unclosed range at EOF → span ending at "stream end"/sentinel (mind the `u64::MAX` boundary bug in `crates/time`, don't use `u64::MAX` as the sentinel); out-of-order pair → clamp/flag rather than panic; duplicate timestamps → deterministic tie-break.
- Match Push/Pop by **thread** (stack discipline per thread) but RangeStart/RangeEnd by **correlation id** across threads — these are two different mechanisms and must not be conflated. Nesting is per-thread for push/pop; process-ranges are explicitly cross-thread.
- Build malformed-telemetry fixtures (unclosed, reordered, cross-thread, duplicate-ts) as first-class analyzer tests.

**Warning signs:**
Analyzer panics/aborts on real captures but passes on synthetic ones; whole-trace analysis fails because of one bad entity; spans with impossible negative or zero-length durations.

**Phase to address:** Analyzer reconstruction — explicit success criterion: *tolerates* incomplete/out-of-order NVTX without panicking.

---

### Pitfall 9: Handle used before its registration event is seen (stream-ordering across per-entity files)

**What goes wrong:**
Domains, registered strings, and categories are referenced by numeric handle. The *registration* event (handle→value) and the *use* (a range citing that handle) may arrive out of order at the analyzer — especially if capture writes per-thread/per-entity files that the analyzer merges, or if the collector interleaves streams. A range that cites string-handle #7 may be processed before the "register string #7 = 'gather'" event, so naïve resolution yields "unknown." Domain-scoped handles compound it: the same numeric id means different things in different domains (and a global/default-domain namespace exists separately), so resolving a handle without its domain context gives the *wrong* string.

**Why it happens:**
Streaming reconstruction assumes registrations precede uses. With multiple ordered-per-source-but-not-globally-ordered streams merged, that assumption breaks. And handle namespaces being per-domain is easy to flatten into one global map.

**How to avoid:**
- Two-pass (or deferred-resolution) analysis: first collect *all* registration events, then resolve handles on ranges — don't resolve inline during a single ordered pass.
- Key every handle by **(domain, kind, id)**, never by id alone. Keep the default/global domain as its own namespace.
- If a handle is still unresolved after all registrations are ingested, render a stable placeholder (`domain#D/string#7`) rather than dropping the range — the range timing is still valid data.

**Warning signs:**
Strings resolve correctly in single-thread runs but wrong/"unknown" in multithreaded ones; the same range shows different labels across runs; labels bleed between domains.

**Phase to address:** Analyzer reconstruction; capture events must include domain context (Injection base / Payload capture)

---

### Pitfall 10: Payload-extension parsing — schema complexity, alignment, and forward compatibility

**What goes wrong:**
The NVTX payload extension lets apps attach binary blobs described by registered schemas (entry types, offsets, enums, nested/variable-length data). Getting this wrong means: reading fields at the wrong offset (schema entries carry explicit offsets/alignment that must be honored, not assumed packed); mishandling unaligned or variable-length entries; treating a schema id like a string handle; or hard-failing when an app uses a schema feature we don't model yet. Because the payload is opaque binary interpreted only via its schema, a parsing bug produces plausible-looking-but-wrong numbers — the worst kind of silent corruption in an analytics tool.

**Why it happens:**
Payload schemas are the most complex, least-documented corner of NVTX, marked "Phase 5 / not yet emitted" even in PR #87. It's tempting to under-model it (assume packed structs, fixed layouts) to ship.

**How to avoid:**
- Capture the *schema registration* verbatim (entries, types, offsets, alignment, flags) as its own event, exactly like string registration; resolve payloads against the captured schema in the analyzer, not in the callback.
- Honor declared offsets/alignment; never assume C-packed layout. Handle variable-length/array entries by length prefix per the schema.
- Forward-compatibility: unknown entry types → preserve raw bytes + mark "unparsed," never abort the whole payload or range.
- Start with a conservative subset (scalar entries) and expand; gate payload capture behind a flag so a payload bug can't break basic range capture.

**Warning signs:**
Payload numeric values that are plausible but wrong; crashes/over-reads only for apps that attach payloads; misalignment faults; new app schema versions breaking parsing.

**Phase to address:** Payload capture (dedicated phase); Analyzer (payload resolution). Flag for deeper research — this is the second least-charted area after fan-out.

---

### Pitfall 11: Process-global one-shot state makes `cargo test` tests interfere

**What goes wrong:**
The injection hook, NVTX's one-shot table resolution, and any global capture singleton are **process-global and initialize once**. `cargo test` runs all tests in *one* process by default (multiple threads). So test A installs the hook and captures into the global sink; test B can't install a fresh hook (one-shot already fired), sees A's events, or races A's global state. Tests pass alone and fail together, or worse pass together and mask real bugs. The existing collector server already has an analogous poisoning hazard (`RwLock.unwrap()` — a panic in one test poisons the lock for all).

**Why it happens:**
Rust's default in-process, multi-threaded test harness is fundamentally incompatible with once-per-process C global state. Developers write ergonomic in-process unit tests and get nondeterministic cross-test contamination.

**How to avoid:**
- Test the injection/fan-out layer via **subprocess-based integration tests**: each scenario spawns a child process (a small in-repo NVTX test binary) that installs the hook fresh, emits a scripted event sequence, and writes output the parent asserts on. This is also what the deterministic in-repo test app is for (per PROJECT.md's CI requirement).
- Keep pure logic (event conversion, handle resolution, span reconstruction, payload parsing) in *separate, side-effect-free* functions/crates that *can* be unit-tested in-process without touching global NVTX state.
- If any in-process test must touch the global hook, serialize it (`serial_test`) and treat the one-shot as a known limitation.
- Design the deterministic test app to script the nasty cases (unclosed ranges, cross-thread start/end, out-of-order, multi-domain handle reuse) so CI exercises Pitfalls 8/9 without GPU hardware.

**Warning signs:**
Tests pass in isolation, fail in the suite (or vice versa); flaky capture-count assertions; a panic in one test cascading into unrelated test failures; CI green only when tests run single-threaded.

**Phase to address:** Injection base *and* Test-app/validation — decide the subprocess test harness early; it shapes how every later phase is testable.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Serialize + send directly from the NVTX callback | Reuse existing `EventSender` with no drain thread | Distorts measured timings (Pitfall 2); backpressures/OOMs the app (Pitfall 3) | Never on the steady-state path; only a throwaway spike |
| Resolve handles inline during a single ordered pass | Simpler analyzer code | Wrong/"unknown" labels under multithread/merged streams (Pitfall 9) | Never — do deferred/two-pass resolution |
| Store raw C message pointer, copy on drain thread | Cheaper-looking callback | Use-after-free crashing the *app* (Pitfall 4) | Never |
| Flatten domain handles into one global id map | Simpler tables | Cross-domain label bleed (Pitfall 9) | Never |
| Ship with only push/pop, defer RangeStart/End & payloads | Faster first capture | Misses cross-thread ranges and structured data; refactor later | MVP capture only, with the model designed to extend |
| Unbounded capture channel to "avoid stalls" | Removes backpressure hang | OOMs instrumented app under bursts (Pitfall 3) | Never — use bounded ring + drop counter |
| In-process `cargo test` for the hook | Ergonomic | Cross-test contamination via global one-shot (Pitfall 11) | Only for pure logic crates, never the hook itself |
| Assume packed payload layout | Quick parser | Silent wrong numbers on aligned/variadic schemas (Pitfall 10) | Never |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| NVTX injection slot | Assuming we can co-exist with nsys by both being injectors | One slot per process; our fan-out claims it and forwards to nsys as a shadow-table sink (Pitfall 7) |
| `NVTX_INJECTION64_PATH` passthrough (nsys/AON) | Double-initializing the external tool or handing it a table it knows is shared | Init the external tool once, give it its own shadow function table so it believes it owns the slot |
| Quent `EventSender`/collector client | Feeding NVTX capture straight into the bounded-mpsc/42×1s-retry client on the app thread | Decouple via non-blocking drain; explicit drop policy; keep app threads off the collector path (Pitfall 3) |
| Quent analyzer FSM runtime | Reusing it as-is for NVTX ranges | Fix the span `unwrap` panics + incomplete-FSM bucket first (Pitfall 8) |
| NVTX registered strings | Copying the string on every range that uses a handle | Capture registration once; store handle on ranges; resolve in analyzer (Pitfall 4/9) |
| Bindgen over NVTX headers | Treating one header snapshot as the runtime ABI | Branch on `version`/`size`; pin+document header version; forward-tolerate (Pitfall 5) |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Allocation/serialization in callback | Measured ranges inflate; per-thread CPU up | Thread-local reusable buffers; serialize off-thread | Immediately at high emission rate (libcudf: thousands of ranges/operator) |
| Blocking drain (bounded, blocking channel) | App stalls when collector/disk slow | Non-blocking producer + drop-with-counter | As soon as consumer < producer rate |
| Unbounded buffer | Instrumented-process RSS climbs to OOM | Bounded ring + drop policy | Bursty workloads / long runs |
| Per-event string matching (not using handles) | Callback CPU scales with string length × rate | Prefer registered-string handles; compare ids | High-frequency ranges with long messages |
| Whole-model in-memory reconstruction (existing analyzer) | Memory grows with trace size; 32-engine cap is count-based | Aware reuse; long-term Arrow-fication (CONCERNS) | Long GPU runs producing large NVTX traces |
| Unpaginated list endpoints (existing) | Huge JSON, slow UI | Pagination before real traces (CONCERNS) | Traces with many ranges/domains |

## Security / Robustness Mistakes

(In-process instrumentation; "security" here is largely process-integrity, since Quent's network surface is already unauthenticated per CONCERNS.md.)

| Mistake | Risk | Prevention |
|---------|------|------------|
| Rust panic unwinding across the C ABI back into the app | Undefined behavior; app crash | `catch_unwind` at every FFI callback boundary; convert to logged error/dropped event |
| Trusting `const char*` lifetime past the call | Use-after-free crashing the instrumented app | Copy immediate strings inside the callback (Pitfall 4) |
| Over-reading attribute/payload structs past `size` | Reads foreign/uninit memory; corruption | Honor `version`/`size`; bounds-check offsets (Pitfalls 5, 10) |
| Injecting into arbitrary processes via `NVTX_INJECTION64_PATH` | Our .so runs in someone else's address space | Document the trust boundary; the injector is only loaded into processes the user opts in; no elevated behavior |
| Passthrough sink crash containment overclaimed | False confidence that fan-out isolates a segfaulting nsys | Document that foreign-sink segfaults are uncontainable; test the specific tools we claim to support |

## "Looks Done But Isn't" Checklist

- [ ] **Capture:** Works for push/pop on one thread — verify RangeStart/RangeEnd matched across threads by correlation id, not thread stack.
- [ ] **Capture:** Ranges come through — verify their domain/string/category *registration* events were also captured (Pitfall 1).
- [ ] **Callback:** Functionally correct — verify it doesn't distort timings (benchmark attached vs. detached) and never blocks/allocates on the hot path (Pitfalls 2, 3).
- [ ] **Strings:** Messages appear — verify no use-after-free under ASAN and that registered-string handles resolve to the right domain-scoped value (Pitfalls 4, 9).
- [ ] **Fan-out:** Quent captures — verify nsys *simultaneously* produces a correct trace unmodified, and that a slow/failing sink can't stall or crash the app (Pitfall 7).
- [ ] **Analyzer:** Reconstructs clean traces — verify it *tolerates* unclosed ranges at exit, out-of-order and duplicate timestamps without panicking (Pitfall 8).
- [ ] **Payloads:** Scalars parse — verify aligned/variable-length/unknown-schema entries don't over-read or silently mis-parse (Pitfall 10).
- [ ] **Tests:** Green in isolation — verify green in the full suite (no global-one-shot contamination) and that CI covers the malformed-stream cases without GPU (Pitfall 11).
- [ ] **ABI:** Works against the app I built — verify against an app built with a different NVTX/CUDA version (Pitfall 5).

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Events dropped before hook (1) | MEDIUM | Move injection discovery earlier (`NVTX_INJECTION64_PATH`); make analyzer tolerate unregistered handles so partial captures still render |
| Callback distorting timings (2) | MEDIUM | Move all serialization off the callback to a drain thread; re-benchmark |
| Backpressure/OOM at boundary (3) | MEDIUM | Replace blocking/unbounded channel with bounded ring + drop counter; keep app off collector path |
| C-string UAF (4) | LOW-MEDIUM | Copy immediate strings in-callback; ASAN test to confirm |
| ABI over-read (5) | LOW | Add version/size branching; re-bindgen against pinned header |
| Symbol/linkage (6) | MEDIUM | Force strong symbol via C shim; prefer runtime injection path; idempotent guarded init |
| Fan-out design wrong (7) | HIGH | Likely redesign; contain via per-sink isolation + shadow tables; may need to re-prototype — cheapest if caught in a spike, not after model/UI built on it |
| Analyzer panics (8) | MEDIUM | Convert span unwraps to errors; bucket incomplete FSMs; add malformed fixtures (CONCERNS prescribes this) |
| Handle-before-registration (9) | LOW-MEDIUM | Switch to deferred two-pass resolution keyed by (domain,kind,id) |
| Payload mis-parse (10) | MEDIUM-HIGH | Capture schema verbatim; rewrite parser to honor offsets/alignment; gate behind flag |
| Test contamination (11) | MEDIUM | Rework hook tests into subprocess integration tests; split pure logic into unit-testable crates |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1 Events before hook | Injection base; Analyzer (graceful unknowns) | Registration-event count matches referenced handles; captures render despite missing registrations |
| 2 Callback distortion | Hot-path capture (injection base) | Attached-vs-detached timing delta within budget; callback microbenchmark sub-µs |
| 3 Backpressure/OOM | Hot-path capture (injection base) | App doesn't stall under slow collector; RSS bounded under burst; drop counter emitted |
| 4 C-string lifetime | Injection base; Payload/handle capture | ASAN clean; registered handles resolve correctly |
| 5 ABI drift | Injection base | Runs against app built with a different NVTX version; struct-size fuzz test passes |
| 6 Symbol/init hazards | Injection base; Fan-out | Injection wins under multiple link orders; idempotent init; coexists with CUPTI-linked app |
| 7 Fan-out multiplex | Fan-out mediator (flag for deep research) | nsys + Quent simultaneously correct; failing sink can't crash/stall app |
| 8 Analyzer panics | Analyzer reconstruction | Malformed-stream fixtures (unclosed/out-of-order/dup-ts) reconstruct without panic |
| 9 Handle before registration | Analyzer; capture (domain context) | Multithreaded/merged-stream captures resolve labels correctly, per-domain |
| 10 Payload parsing | Payload capture (flag for deep research); Analyzer | Aligned/variadic/unknown-schema payloads parse or degrade safely |
| 11 Test contamination | Injection base + Test-app/validation | Suite green in-process and single-threaded; subprocess integration tests script the nasty cases |

## Sources

- NVIDIA NVTX C API Reference (registered strings are domain-scoped handles logged once at registration; process RangeStart/RangeEnd cross threads via correlation id; `nvtxEventAttributes` version/size for multi-version handling): https://nvidia.github.io/NVTX/doxygen/index.html — MEDIUM-HIGH
- NVIDIA Profiler User's Guide / Nsight Systems docs (once a tool is present, all NVTX calls jump into the tool's implementation on the app thread; strongly recommend registered strings to avoid per-call string-match overhead): https://docs.nvidia.com/cuda/profiler-users-guide/index.html , https://docs.nvidia.com/nsight-systems/UserGuide/index.html — MEDIUM-HIGH
- CUPTI docs (InitializeInjectionNvtx2 injection contract; init done once per process; init APIs made thread-safe): https://docs.nvidia.com/cupti/main/main.html — MEDIUM
- NVTX repository / headers (weak `InitializeInjectionNvtx2`, `NVTX_INJECTION64_PATH`, payload extension / schemas): https://github.com/NVIDIA/NVTX — MEDIUM
- Quent codebase concerns (analyzer panics on out-of-order/duplicate timestamps and incomplete FSMs; collector-client bounded-channel backpressure + 42×1s retry; `u64::MAX` timestamp boundary bug; RwLock poisoning): `.planning/codebase/CONCERNS.md` — HIGH
- Project context (one-slot-per-process invariant, unprototyped fan-out shadow-table design, payload "Phase 5 / not yet emitted", Windows out of scope for wchar_t/linker reasons, deterministic in-repo test app requirement): `.planning/PROJECT.md` — HIGH
- FFI/linkage hazards (Rust panic across C ABI is UB → `catch_unwind`; weak/strong symbol override; Linux 4-byte `wchar_t`; static init order): training-data engineering knowledge — MEDIUM

---
*Pitfalls research for: NVTX injection/consumer + fan-out mediator feeding the Quent telemetry pipeline*
*Researched: 2026-07-08*
