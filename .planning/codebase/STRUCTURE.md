# Codebase Structure

**Analysis Date:** 2026-07-08

## Directory Layout

```
nvtx_consumer/  (quent repo worktree)
├── Cargo.toml               # Cargo workspace root (members + default-members)
├── pixi.toml                # pixi env: Rust, Node, pnpm, protoc toolchains
├── docker-compose.yml       # simulator server + application for UI dev
├── Dockerfile               # server container build
├── deny.toml                # cargo-deny config
├── crates/                  # Application-agnostic Rust crates
│   ├── analyzer/            # Domain-agnostic analysis (fsm/, resource/, timeline/, trace/, entity/)
│   ├── attributes/          # Attribute/Value types
│   ├── build-info/          # Model provenance (BuildInfo, ModelSource)
│   ├── codegen/             # C++ (cxx) and Python (PyO3) bridge generation
│   ├── collector/           # gRPC collector: proto/, client/, server/
│   ├── events/              # Event<T> envelope, EntityEvent trait
│   ├── exporter/            # Umbrella crate + types/, ndjson/, msgpack/, postcard/, collector/, callback/
│   ├── instrumentation/     # Context, Observer, EventSender, sidecar
│   ├── model/               # Core model types (fsm_event, capacity, resource, usage, ref)
│   ├── model-macros/        # Proc macros (entity!, fsm!, model!, state!, resource!, instrumentation!)
│   ├── open/                # quent-open binary (artifact viewer)
│   ├── stdlib/              # Reusable resources: channel, memory, processor
│   ├── time/                # Timestamps, spans, test clock override
│   ├── ui/                  # Rust→TS view types (ts-rs), timeline request/response
│   ├── constraints/ schema/ ref-target/ ref-tree/ fsm/ resource/
│   │                        # UNUSED preliminary crates (issue #191) — avoid
│   └── instrumentation-build/  # UNUSED preliminary (with example/)
├── domains/
│   └── query_engine/        # Domain-specific crates
│       ├── model/           # Engine/Worker/Query/QueryGroup/Plan/Operator/Port model
│       ├── analyzer/        # Model reconstruction + UiAnalyzer trait (plan/ subtree)
│       ├── server/          # axum+tonic composition, analyzer_cache, timeline_cache, ui routes
│       ├── ui/              # TS-shared request/response types
│       └── tests/           # cpp/, python/ bridge tests; fixed/ deterministic emitter
├── examples/
│   ├── readme/              # Full modeling-concepts example (src/lib.rs, src/main.rs)
│   ├── simulator/           # End-to-end pipeline: instrumentation/, application/, analyzer/, server/, ui/
│   ├── cpp-integration/     # C++ app + bridge/ crate
│   └── python-integration/  # Python app + bridge/ crate
├── proto/
│   └── quent/collector/v1/collector.proto   # gRPC contract (compiled by crates/collector/proto/build.rs)
├── docs/                    # mdBook spec (book.toml, SUMMARY.md)
│   ├── event_model.md       # Event design rationale
│   ├── modeling/            # entity.md, fsm.md, resource.md, resource_group.md, attributes.md, time.md
│   └── domains/query_engine/  # Domain spec + examples
└── ui/                      # pnpm workspace: React 19 + Vite frontend
    ├── src/                 # App code (routes/, pages/, components/, atoms/, hooks/, contexts/, lib/, test/)
    ├── packages/@quent/     # Workspace packages: client/, components/, hooks/, utils/
    ├── e2e/                 # Playwright tests
    ├── examples/            # Opt-in examples (NOT in pnpm workspace)
    └── public/              # Static assets
```

## Directory Purposes

**`crates/` (application-agnostic):**
- Purpose: reusable building blocks; every crate is `quent-*` named
- Key files: `crates/model/src/lib.rs` (trait surface), `crates/instrumentation/src/lib.rs` (runtime), `crates/exporter/src/lib.rs` (transport umbrella), `crates/analyzer/src/lib.rs` (analysis traits)

**`domains/query_engine/` (domain-specific):**
- Purpose: the first supported domain; model + analyzer + server + UI types
- Key files: `domains/query_engine/model/src/lib.rs` (the `model!` invocation), `domains/query_engine/server/src/ui.rs` (HTTP routes), `domains/query_engine/analyzer/src/ui.rs` (`UiAnalyzer`)

**`examples/simulator/` (reference wiring):**
- Purpose: canonical end-to-end pipeline; copy this layout when building a new tool
- Key files: `examples/simulator/instrumentation/src/lib.rs` (model composition + `instrumentation!`), `examples/simulator/server/src/main.rs` (service wiring), `examples/simulator/analyzer/src/lib.rs` (`SimulatorUiAnalyzer`)

**`ui/` (frontend):**
- Purpose: React SPA; talks to `/api/engines` endpoints
- Key files: `ui/src/main.tsx`, `ui/src/routes/__root.tsx`, `ui/packages/@quent/client/src/api.ts`

**`docs/` (specification):**
- Purpose: mdBook defining modeling concepts; normative for model semantics
- Key files: `docs/modeling/README.md`, `docs/event_model.md`, `docs/faq.md`

**`proto/`:**
- Purpose: protobuf contract for the collector service
- Key files: `proto/quent/collector/v1/collector.proto`

## Key File Locations

**Entry Points:**
- `examples/simulator/server/src/main.rs`: collector + analyzer server binary
- `examples/simulator/application/src/main.rs`: telemetry-emitting simulator
- `crates/open/src/main.rs`: `quent-open` artifact viewer
- `ui/src/main.tsx`: frontend app bootstrap

**Configuration:**
- `Cargo.toml` (root): workspace members, `default-members`, shared `[workspace.dependencies]`
- `pixi.toml`: toolchain env; `docker-compose.yml`: dev backend
- `ui/vite.config.ts`, `ui/tsconfig.json`, `ui/pnpm-workspace.yaml` (includes audit-ignore policy with mandatory justification comments)
- `deny.toml`: dependency license/advisory policy

**Core Logic:**
- `crates/model-macros/src/{entity_macro.rs,fsm_macro.rs,model_macro.rs,state_macro.rs,resource_macro.rs}`: code generation
- `crates/instrumentation/src/observer.rs`: event pipeline
- `crates/analyzer/src/{fsm,resource,timeline}/`: analysis runtimes
- `domains/query_engine/server/src/{analyzer_cache.rs,timeline_cache.rs}`: serving caches

**Testing:**
- Rust integration tests: per-crate `tests/` dirs (e.g. `crates/model/tests/`, `crates/fsm/tests/`); unit tests inline in `#[cfg(test)] mod tests`
- Bridge tests: `domains/query_engine/tests/{cpp,python}/`
- Deterministic fixture emitter: `domains/query_engine/tests/fixed/src/main.rs` (opt-in via `-p`)
- Frontend: co-located `*.test.tsx` (e.g. `ui/src/components/QueryResourceTree.test.tsx`), setup in `ui/src/test/`, Playwright in `ui/e2e/`

## Naming Conventions

**Crates:**
- Directory: short snake/kebab name (`crates/model-macros`); package name prefixed `quent-` (`quent-model-macros`, `quent-simulator-server`, `quent-query-engine-analyzer`)

**Rust files:**
- snake_case modules; multi-file features become directories with `mod.rs` (`crates/analyzer/src/fsm/mod.rs`)
- Every source file carries SPDX headers (`SPDX-FileCopyrightText` / `SPDX-License-Identifier: Apache-2.0`)

**Frontend files:**
- Components: PascalCase `.tsx` (`ui/src/components/QueryPlan.tsx`)
- Routes: TanStack Router dotted file-based names with `$param` segments (`ui/src/routes/profile.engine.$engineId.query.$queryId.timeline.tsx`)
- Atoms/hooks/lib: camelCase `.ts` (`ui/src/atoms/resourceTree.ts`, `ui/src/hooks/useExpandedIds.ts`)
- Workspace packages: `@quent/*` scope under `ui/packages/@quent/`

## Where to Add New Code

**New application-agnostic capability:**
- New crate under `crates/<name>/` with package name `quent-<name>`; register in BOTH `members` and `default-members` of root `Cargo.toml`; pin shared deps via `[workspace.dependencies]`

**New domain:**
- `domains/<domain>/{model,analyzer,server,ui}/` mirroring `domains/query_engine/`

**New query-engine model entity:**
- Entity module in `domains/query_engine/model/src/`; add to the `model!` block in `domains/query_engine/model/src/lib.rs`; analyzer counterpart in `domains/query_engine/analyzer/src/`

**New exporter format/transport:**
- Sub-crate `crates/exporter/<format>/`; feature-gate and wire into `ExporterOptions` in `crates/exporter/src/lib.rs`

**New HTTP endpoint:**
- Handler + route in `domains/query_engine/server/src/ui.rs`; response types (ts-rs derived) in `domains/query_engine/ui/src/lib.rs` or `crates/ui/src/`; typed fetcher in `ui/packages/@quent/client/src/`

**New UI page/route:**
- File in `ui/src/routes/` (run `pnpm routes:generate`); shared visuals in `ui/packages/@quent/components/`; client state atoms in `ui/src/atoms/`

**Tests:**
- Rust: inline `#[cfg(test)]` for units, `tests/` dir for integration
- Frontend: co-located `*.test.tsx` (vitest + testing-library + msw), e2e in `ui/e2e/`

## Special Directories

**`crates/{constraints,schema,ref-target,ref-tree,fsm,resource,instrumentation-build}`:**
- Purpose: unused preliminary crates for rapidsai/quent#191
- Generated: No — Committed: Yes — do not extend without consulting the issue

**`.pixi/`:**
- Purpose: pixi-managed toolchains (includes a full Rust toolchain — exclude from searches)
- Generated: Yes — Committed: No

**`examples/simulator/server/ts-bindings/`, `crates/schema/ts/`:**
- Purpose: ts-rs generated TypeScript bindings
- Generated: Yes (by Rust tests/build) — Committed: Yes

**`ui/examples/`:**
- Purpose: opt-in frontend examples, intentionally NOT in the pnpm workspace (see `ui/pnpm-workspace.yaml` comments); run pnpm inside the example folder

**`domains/query_engine/tests/fixed/`:**
- Purpose: deterministic event emitter; activates `quent-time/__test-clock-override`, excluded from `default-members` to preserve the zero-cost guarantee

---

*Structure analysis: 2026-07-08*
