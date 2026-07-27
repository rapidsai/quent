# Codebase Concerns

**Analysis Date:** 2026-07-08

## Tech Debt

**Unused preliminary crates (issue #191):**
- Issue: Seven crates are explicitly marked "Unused preliminary crates" in the workspace manifest and excluded from active use
- Files: `crates/constraints/`, `crates/schema/`, `crates/ref-target/`, `crates/ref-tree/`, `crates/fsm/`, `crates/resource/`, `crates/instrumentation-build/` (listed in `Cargo.toml:54-62`)
- Impact: Dead weight in the workspace; duplicated concepts (e.g. `crates/fsm` vs FSM logic in `crates/model-macros` / `crates/analyzer`) can confuse navigation and refactoring
- Fix approach: Either fold their concepts into the active crates per https://github.com/rapidsai/quent/issues/191 or remove them from the workspace

**Missing pagination on list endpoints:**
- Issue: Engine/query-group/query listing endpoints return unbounded result sets; each has a `TODO(johanpel): pagination`
- Files: `domains/query_engine/server/src/ui.rs:63`, `domains/query_engine/server/src/ui.rs:113`, `domains/query_engine/server/src/ui.rs:144`
- Impact: Large engines (many query groups/queries) produce huge JSON responses; UI and server memory/latency degrade with trace size
- Fix approach: Add limit/offset or cursor pagination to `list_engines`, `list_query_groups`, `list_queries` and the corresponding UI fetch hooks

**In-memory Rust-native analyzer data model (planned Arrow migration):**
- Issue: All events are deserialized into Rust-native in-memory types; the crate-level doc flags this as post-PoC debt with a planned move to Arrow/DataFusion
- Files: `domains/query_engine/analyzer/src/lib.rs:6-17`, `crates/analyzer/src/`
- Impact: Query performance and memory scalability limited; interactive/ad-hoc analysis requires new hand-written Rust per question
- Fix approach: Arrow-fication of the event store and/or a DataFusion-backed query layer (prior art referenced in the doc comment)

**UI type bindings sourced from simulator ts-bindings:**
- Issue: The front-end resolves `~quent/types` to TS bindings generated into the simulator server crate; comment notes this must change when bindings come from the webserver
- Files: `ui/vite.config.ts:69-71` (alias to `examples/simulator/server/ts-bindings`)
- Impact: The production UI is type-coupled to an example crate's generated output; regenerating or removing the simulator breaks the UI build
- Fix approach: Serve/publish bindings from the query-engine server (or a dedicated bindings package) and repoint the alias

**Duplicated route logic in UI:**
- Issue: The node-detail route duplicates the query route's behavior; open question about DAG node selection behavior
- Files: `ui/src/routes/profile.engine.$engineId.query.$queryId.node.$nodeId.tsx:11`, `ui/src/components/QueryPlan.tsx:64` (root-plan fetch timing question)
- Impact: Divergence risk when one route changes; unclear intended selection semantics
- Fix approach: Extract shared query-view logic; decide and document node-selection behavior

**Scattered TODO(johanpel) design debt in analyzer core:**
- Issue: ~25 TODOs mark deferred validation and design decisions: transition-logic validation (`crates/analyzer/src/resource/runtime.rs:110`), rate-capacity variant (`crates/analyzer/src/resource/mod.rs:84`), blocking/non-blocking channel semantics (`crates/analyzer/src/resource/collection.rs:161`), view construction (`examples/simulator/analyzer/src/view.rs:31`), per-query view caching (`examples/simulator/analyzer/src/lib.rs:151`)
- Files: see paths above
- Impact: Analyzer accepts some invalid transition sequences silently; view construction is ad hoc
- Fix approach: Triage TODOs into issues; prioritize resource-transition validation since it guards data correctness

## Known Bugs

**`u64::MAX` is representable but not a valid timestamp:**
- Symptoms: `u64::MAX` cannot fall into any half-open span interval; binning of a span ending at `u64::MAX` is inconsistent at the boundary
- Files: `crates/time/src/lib.rs:30-34` (TODO on `TimeUnixNanoSec`), `crates/time/src/bin.rs:267-272` (test documenting the issue)
- Trigger: Any event or span carrying `u64::MAX` (e.g. as an "unknown end" placeholder)
- Workaround: None enforced; TODO suggests a newtype excluding `u64::MAX` or making it an explicit "up to infinity" sentinel

**One incomplete FSM fails the whole analyzer build:**
- Symptoms: `RtFsmsBuilder::try_build` bubbles up the first FSM build error, so a single entity that crashed mid-lifecycle (missing exit transition) makes the entire engine's analysis fail
- Files: `crates/analyzer/src/fsm/runtime.rs:309-313` (TODO: move incomplete FSMs into their own bucket)
- Trigger: Ingesting telemetry from an application that terminated abruptly
- Workaround: None; requires complete telemetry per FSM

## Security Considerations

**No authentication on collector gRPC or analyzer HTTP API:**
- Risk: Anyone who can reach the ports can inject telemetry events or read all analysis data; default bind is all interfaces (`[::]:7836` collector, `[::]:8080` analyzer)
- Files: `examples/simulator/server/src/main.rs:18-20`, `domains/query_engine/server/src/ui.rs` (routes), `crates/collector/server/src/server.rs`
- Current mitigation: None in code; project is pre-alpha and intended for trusted environments (see `README.md` Status section)
- Recommendations: Document the trust model; add optional TLS/token auth before any non-local deployment; default binds to loopback

**Collector trusts client-supplied `source-context-id`:**
- Risk: A client can claim any UUID as its source context; a colliding or spoofed id routes events into another source's mirrored context and shares its sink
- Files: `crates/collector/server/src/server.rs:107-117` (metadata parsing), `crates/collector/server/src/server.rs:130-165` (context registry keyed by that id)
- Current mitigation: None; ids are UUIDv7 so accidental collision is unlikely
- Recommendations: Treat as part of the unauthenticated-trust model above; consider server-assigned session ids if auth is added

**Ignored RUSTSEC advisories:**
- Risk: `atomic-polyfill` is unmaintained (RUSTSEC-2023-0089, via `postcard -> heapless`); `quick-xml` DoS advisories (RUSTSEC-2026-0194/0195, via dev-only `pprof -> inferno`)
- Files: `deny.toml:9-21` (documented ignore list)
- Current mitigation: Documented rationale; quick-xml issues are dev/bench-only, not in shipped artifacts
- Recommendations: Revisit when `heapless`/`pprof`/`inferno` publish fixed releases (deny.toml notes quick-xml >= 0.41 as the exit condition)

**Archive extraction (mitigated — keep it that way):**
- Risk: Zip/tar extraction of user-supplied trace archives (path traversal, zip bombs)
- Files: `crates/open/src/archive.rs` (per-archive and run-wide byte caps, entry-count cap, rejection of escaping paths/symlinks/hardlinks/devices, lines 18-42 and 147-216)
- Current mitigation: Comprehensive — caps plus entry filtering, with tests
- Recommendations: Preserve these invariants when modifying `extract_archive`; add new entry types to the reject list, not the allow list

## Performance Bottlenecks

**Per-bin hashmaps in timeline binning:**
- Problem: Binned resource timeline aggregation allocates hashmaps per aggregation step
- Files: `crates/analyzer/src/timeline/binned/resource.rs:66` (TODO: "perf is fine for now but ... prevent all the hashmaps")
- Cause: Convenience-first data layout during PoC
- Improvement path: Flatten to indexed arrays keyed by pre-resolved bin/resource indices; part of the broader Arrow-fication plan

**Whole-model in-memory reconstruction per engine:**
- Problem: Opening an engine deserializes its entire event history into memory before any query can run
- Files: `domains/query_engine/server/src/analyzer_cache.rs` (moka cache, `max_capacity(32)` analyzers at `analyzer_cache.rs:192`), `crates/analyzer/src/`
- Cause: In-memory analyzer design (see Tech Debt: Arrow migration)
- Improvement path: Lazy/columnar storage; capacity is count-based, so 32 large engines can still exhaust memory — consider byte-weighted eviction

**Timeline chunk cache sized by entry count, not bytes:**
- Problem: `TimelineCache` holds up to 4096 `SingleTimelineResponse` chunks with 1h TTL; per-chunk size varies with bin count, so memory use is unbounded in byte terms
- Files: `domains/query_engine/server/src/timeline_cache.rs:146-153`
- Cause: moka `max_capacity` counts entries by default
- Improvement path: Provide a `weigher` based on serialized bin payload size

**Collector client backpressure into the instrumented application:**
- Problem: Bounded mpsc channels (1024) mean `Client::send` awaits when the collector is slow, stalling the instrumented app's event path; TODO notes "consider unbounded"
- Files: `crates/collector/client/src/lib.rs:95-101`, `crates/collector/client/src/lib.rs:235-241`
- Cause: Fixed channel capacity with no drop policy
- Improvement path: Decide policy explicitly: drop-oldest, unbounded with memory watermarks, or documented backpressure

**Fixed 42x1s connect retry loop:**
- Problem: Client startup blocks up to 42 seconds retrying the collector connection with no backoff configurability; TODO suggests using health checks
- Files: `crates/collector/client/src/lib.rs:75-92`
- Cause: Hardcoded retry constants
- Improvement path: Configurable retry/backoff; use gRPC health-check service

## Fragile Areas

**Proc-macro crates (`model-macros`):**
- Files: `crates/model-macros/src/entity_macro.rs` (1094 lines), `crates/model-macros/src/resource_derive.rs` (909), `crates/model-macros/src/model_macro.rs` (561), `crates/model-macros/src/fsm_macro.rs` (543), `crates/model-macros/src/state_macro.rs` (527)
- Why fragile: Token-level code generation; small changes ripple into every generated instrumentation API
- Safe modification: Extend the compile-fail suite in `crates/model/tests/compile_fail/` alongside any change; `crates/model/tests/fsm_validation.rs:12` notes missing trybuild cases for invalid FSM definitions
- Test coverage: Good macro-expansion coverage via `crates/model/tests/` (incl. `macro_corner_cases.rs`, `cross_crate_composition.rs`); gaps in invalid-FSM compile-fail cases

**String-based C++/Python bridge codegen:**
- Files: `crates/codegen/src/cxx_bridge.rs` (1373 lines), `crates/codegen/src/pyo3_bridge.rs` (1112 lines)
- Why fragile: Generates cxx `unsafe extern "C++"` blocks and PyO3 modules via string templates; `panic!` on unexpected model shapes (`cxx_bridge.rs:706,716,1050,1202,1232`, `pyo3_bridge.rs:954`) turns model edge cases into build-time crashes
- Safe modification: Run `crates/codegen/tests/cxx_bridge_generation.rs` and `crates/codegen/tests/pyo3_bridge_generation.rs` plus the end-to-end bridge tests in `domains/query_engine/tests/cpp/` and `domains/query_engine/tests/python/`
- Test coverage: Generation snapshot tests plus compiled bridge tests exist; panics are untested paths

**Analyzer ordering assumptions (panics on malformed telemetry):**
- Files: `crates/analyzer/src/fsm/mod.rs:125-135` (`SpanUnixNanoSec::try_new(start, end).unwrap()`), `crates/analyzer/src/fsm/runtime.rs:170,284`, `crates/analyzer/src/trace/mod.rs:170-201`, `crates/analyzer/src/resource/runtime.rs:97-103`
- Why fragile: `.unwrap()` on span construction assumes transitions are time-ordered; out-of-order or duplicate-timestamp events from a buggy exporter panic the analyzer instead of returning `AnalyzerError`
- Safe modification: Convert unwraps to error propagation when touching these paths; add malformed-event fixtures
- Test coverage: Happy-path tested; no malformed/out-of-order event tests

**Collector server lock handling:**
- Files: `crates/collector/server/src/server.rs:50,132,157` (`RwLock` `.unwrap()`), `server.rs:48-67` (`StreamGuard::drop` spawning an OS thread to flush)
- Why fragile: A panic while holding the contexts lock poisons it and panics every subsequent stream; the drop-on-plain-thread flush dance encodes a subtle runtime-blocking invariant documented only in comments
- Safe modification: Preserve the "drop context off-runtime" invariant; consider `parking_lot` or explicit poison recovery
- Test coverage: Round-trip covered by `crates/instrumentation/tests/collector_roundtrip.rs`; no cancellation/poisoning tests

**Timeline cache chunk geometry:**
- Files: `domains/query_engine/server/src/timeline_cache.rs` (1452 lines — largest file in the repo)
- Why fragile: Cross-endpoint cache-key sharing (single vs bulk) depends on per-entry `params_hash` equivalence (`timeline_cache.rs:134-143`); a divergence silently serves wrong bins
- Safe modification: Keep single/bulk key derivation in one function; run the inline test suite (has mock analyzers with `unimplemented!` stubs at lines 929-946)
- Test coverage: Good inline unit tests for cache behavior

## Scaling Limits

**In-memory engine models:**
- Current capacity: Bounded by host RAM; `AnalyzerCache` keeps up to 32 fully-materialized engine models (`domains/query_engine/server/src/analyzer_cache.rs:192`)
- Limit: Large traces (long-running engines, many workers) exhaust memory before the entry-count cap matters
- Scaling path: Arrow/columnar storage per the analyzer TODO (`domains/query_engine/analyzer/src/lib.rs:6-17`); byte-weighted cache eviction

**Unpaginated API responses:**
- Current capacity: Fine for demo-scale engines
- Limit: Response size grows linearly with query/query-group count (`domains/query_engine/server/src/ui.rs:63,113,144`)
- Scaling path: Pagination (see Tech Debt)

**Single-collector ingestion:**
- Current capacity: One gRPC collector process; per-source contexts in one process (`crates/collector/server/src/server.rs`)
- Limit: All sources' ingest and observer flushing share one process; 4 MiB gRPC message budget per batch (`crates/collector/client/src/lib.rs:116`)
- Scaling path: Not designed yet (pre-alpha); horizontal sharding by source-context-id would be the natural cut

## Dependencies at Risk

**`atomic-polyfill` (transitive via `postcard -> heapless`):**
- Risk: Unmaintained (RUSTSEC-2023-0089); ignored in `deny.toml:12-14`
- Impact: No safe upgrade currently; affects the postcard exporter path (`crates/exporter/postcard/`)
- Migration plan: Bump when `heapless` drops the dependency; ignore entry documents this

**`quick-xml` 0.26 (dev-only, via `pprof -> inferno`):**
- Risk: Two DoS advisories (RUSTSEC-2026-0194/0195); ignored in `deny.toml:15-21`
- Impact: Dev/bench flamegraph generation only; not in shipped artifacts
- Migration plan: Drop ignores once pprof/inferno bump to quick-xml >= 0.41 (documented in deny.toml)

**Pre-alpha instability (self-declared):**
- Risk: `README.md:25-31` declares all APIs continuously subject to breaking changes, no releases
- Impact: Downstream consumers (C++/Python bridges, UI bindings) must track HEAD
- Migration plan: Not applicable until the project stabilizes

## Missing Critical Features

**Authentication/authorization:**
- Problem: No auth on any network surface (collector gRPC, analyzer HTTP)
- Blocks: Any deployment beyond a trusted local network

**Pagination:**
- Problem: List endpoints return everything (`domains/query_engine/server/src/ui.rs`)
- Blocks: UI usability on large traces

**Tolerance for incomplete telemetry:**
- Problem: Analyzer requires complete FSM lifecycles (`crates/analyzer/src/fsm/runtime.rs:309-313`)
- Blocks: Analyzing traces from crashed or abruptly-terminated applications

## Test Coverage Gaps

**UI unit tests:**
- What's not tested: 19+ route/component files in `ui/src/routes/` and `ui/src/components/` have only 3 unit test files (`ui/src/components/QueryResourceTree.test.tsx`, `ui/src/routes/profile.index.test.tsx`, `ui/src/test/example.test.tsx`) plus a single Playwright smoke test (`ui/e2e/smoke.spec.ts`)
- Files: `ui/src/components/QueryPlan.tsx`, `ui/src/routes/profile.engine.*.tsx`, timeline components
- Risk: Timeline rendering, DAG selection, and data-fetch regressions ship unnoticed
- Priority: Medium (pre-alpha UI, but it is the primary demo surface)

**Malformed/out-of-order event handling in analyzer:**
- What's not tested: Analyzer behavior on out-of-order transitions, duplicate timestamps, missing exit events
- Files: `crates/analyzer/src/fsm/`, `crates/analyzer/src/trace/mod.rs`, `crates/analyzer/src/resource/runtime.rs`
- Risk: Panics (unwraps on span construction) instead of graceful errors when real applications emit imperfect telemetry
- Priority: High

**Collector stream lifecycle edge cases:**
- What's not tested: Stream cancellation mid-batch, concurrent first-touch context construction races, lock poisoning
- Files: `crates/collector/server/src/server.rs`, `crates/collector/client/src/lib.rs`
- Risk: Context leaks or double-drops under client disconnects
- Priority: Medium

**Invalid FSM compile-fail cases:**
- What's not tested: `crates/model/tests/fsm_validation.rs:12` lists missing trybuild compile-fail tests for invalid FSM definitions
- Files: `crates/model/tests/compile_fail/`
- Risk: Macro regressions accept invalid models silently
- Priority: Medium

---

*Concerns audit: 2026-07-08*
