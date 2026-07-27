# Current Crates Map — Latest-Design quent Crates

> Deep map of the **11 current/latest-design crates** under `crates/`. Everything
> else in the repo is **legacy** (do not build on it / copy its patterns).
> Produced 2026-07-22 by parallel deep-read of full source (not excerpts).
> Scope confirmed by the user: the 9 "issue #191" crates + `events` + `dynamic-attributes`.

## The 11 crates & the active/preliminary boundary

Registration in the root `Cargo.toml` splits the set in two:

| Crate | Registered as | Role |
|-------|---------------|------|
| `crates/events` | **default-member** (active) | Runtime event envelope |
| `crates/dynamic-attributes` | **default-member** (active) | Runtime dynamic attribute values |
| `crates/instrumentation` | **default-member** (active) | Runtime event pipeline (sync→async) |
| `crates/schema` | members-only (#191 preliminary) | Event-model data model + builders + visitor |
| `crates/constraints` | members-only (#191 preliminary) | Validation engine (`Constraint`/`validate`) |
| `crates/ref-target` | members-only (#191 preliminary) | `EntityRef` target-type constraint |
| `crates/ref-tree` | members-only (#191 preliminary) | Tree-shape reference constraint |
| `crates/fsm` | members-only (#191 preliminary) | FSM constraint + entity builder |
| `crates/resource` | members-only (#191 preliminary) | Resource/capacity constraint + builder |
| `crates/instrumentation-build` | members-only (#191 preliminary) | Codegen: `Schema` → instrumentation Rust source |
| `crates/yaml` | members-only (#191 preliminary) | YAML event-model source → `Schema` |

"members-only" = excluded from `default-members`, so a bare `cargo build`/`cargo test`
skips them (preserves the `quent-time` zero-cost guarantee). Build/test them with `-p <crate>`.

## Intra-set dependency graph

Two nearly-decoupled clusters, bridged only at codegen time by `instrumentation-build`.

```text
CLUSTER A — event-model / validation (#191)          CLUSTER B — runtime
────────────────────────────────────────            ─────────────────────
                 schema  (root, no in-set deps)       events   dynamic-attributes
                   │                                     │  (both leaves; events
        ┌──────────┼───────────┐                        │   also uses quent-time)
        ▼          ▼           ▼                         └────────┬──────────┘
   constraints  (used by all below)                              ▼
        │                                                  instrumentation
   ┌────┼──────────┬─────────┐                        (+ legacy quent-io,
   ▼    ▼          ▼         ▼                            quent-build-info)
ref-target  fsm         (schema+constraints)
   │         │
   ▼         ▼
ref-tree   resource
(→ref-target) (→fsm)

              yaml ── depends on ──▶ schema, constraints, fsm, ref-target, ref-tree

   instrumentation-build ── depends on ──▶ schema, constraints, ref-target
        └─ emits source that names `::quent_instrumentation::*`, `::quent_events::*`,
           `::quent_dynamic_attributes::*` as TEXT — NO cargo edge to Cluster B.
           Only the *generated consumer crate* links the runtime.
```

Explicit edges (cargo `[dependencies]` + verified `use`):

- `schema` → (none in-set)
- `constraints` → `schema`
- `ref-target` → `schema`, `constraints`
- `ref-tree` → `schema`, `constraints`, `ref-target`
- `fsm` → `schema`, `constraints`
- `resource` → `schema`, `constraints`, `fsm` (only to read `FsmConstraint::NAME`)
- `yaml` → `schema`, `constraints`, `fsm`, `ref-target`, `ref-tree`
- `events` → (none in-set; uses `quent-time`)
- `dynamic-attributes` → (none in-set)
- `instrumentation` → `events`, `dynamic-attributes`
- `instrumentation-build` → `schema`, `constraints`, `ref-target` (**not** `instrumentation`)

**Key structural insight:** the schema/validation world (Cluster A) and the runtime
world (Cluster B) never link directly. The only connection is `instrumentation-build`
consuming a `Schema` and emitting *text* that references the runtime crates; the runtime
becomes a dependency of the generated consumer, not of any crate here.

## Constraint registry

Annotation-driven constraints (each is a `Constraint` = `Visitor + Default` with a
versioned `NAME`; validated in one `schema.walk` by `quent_constraints::validate`):

| `NAME` | Crate | Purpose | Payload (opaque JSON in `Constraint.data`) |
|--------|-------|---------|--------------------------------------------|
| `quent.ref-target.v0.1.0` | ref-target | Restrict which entity an `EntityRef` points at | target `Identifier` as string |
| `quent.ref-tree.v0.1.0` | ref-tree | Mark refs as tree-forming; enforce single-rooted tree | marker (no data) |
| `quent.fsm.v0.1.0` | fsm | Entity events form a validated FSM topology | serde of `Fsm { initial_state, transitions, exit_from_states }` |
| `quent.resource.v0.1.0` | resource | Capacity definitions + usage/bounds records | serde of `Resource` enum (`definition`/`usage`/`bounds`) |

Plus **always-on base constraints** (in `constraints`, not annotation-driven, "should
always pass"): unresolved record refs, recursive records, entities-without-events, and
`unregistered_constraints` (reported as warnings — unknown annotation names).

Validation is capped at **tuple arity ≤ 12** (macro-bounded in both `schema`'s tuple
`Visitor` impl and `constraints`' `Constraints` impl).

---

## Cluster A — event-model / validation

### crates/schema  *(root of the set — everything in Cluster A builds on it)*

- **Purpose** — Minimal, annotation-extensible in-memory data model of an application
  event model (enough to read/write every event without further interpretation), plus
  builders, an optional `visitor` traversal API, and optional serde/TS support.
- **Public surface** (re-exported from `src/lib.rs`): `Schema`, `Entity`, `Event`,
  `Cardinality {Once, Multi}`, `Record`, `Field`, `DataType`, `Identifier`, `Annotations`,
  `Constraint`, `Metadata`. Modules: `builder`, `schema`, `visitor` (feat), `test_utils` (feat).
  - `DataType`: `Bool, Uuid, String, U8..U64, I8..I64, F32, F64, Option(Box), List(Box),
    Record(Identifier), DynamicRecord, EntityRef { data: Option<Box<DataType>>, annotations }`.
  - `Identifier(String)` grammar `[A-Za-z][A-Za-z0-9_]*`, **ASCII-only** (cross-lang interop);
    `IdentifierError {Empty, InvalidStart, InvalidChar}`.
  - `Annotations { docs, constraints: Map<String,Constraint>, metadata: Map<String,Metadata> }`
    — `has_constraint(name)`, `constraint(name)`, etc. `Constraint`/`Metadata` are opaque
    `{ name, data: Option<String> }`.
  - Builders (`SchemaBuilder`/`EntityBuilder`/`EventBuilder`/`RecordBuilder`/`AnnotationsBuilder`):
    consistent triad **`set_*` / `try_insert_*` / `try_with_*`**; `BuilderError {DuplicateName,
    EmptyName, NoEvents}`. Only `EntityBuilder::build()` is fallible (`NoEvents`).
  - `visitor` feature: `trait Visitor { type Output; fn visit(&mut self,&Cursor); fn finish(self)->Output }`,
    `Schema::walk<T:Visitor>`, `enum Element<'s> {Schema,Annotations,Entity,Event,Field,Record,DataType}`,
    `Cursor` (dot-path `Display`, `current()/previous()/root()/elements()`). Impl'd for `()` and tuples 1..=12.
- **Patterns** — Private fields + `pub(crate) from_parts` + read-only accessors (mutation only
  via builders). Ordered `IndexMap<_,_,FxBuildHasher>` everywhere. Walk order: element →
  its `Annotations` → children; `Record(name)` is a **leaf** at a reference (fields walked
  once via `Schema::records`), so recursion terminates.
- **Features** — `default=["visitor"]`; `serde`; `visitor` (pulls smallvec); `test-utils`;
  `ts` (pulls ts-rs, implies serde).
- **Gotchas** — `eventless_entity` test-util bypasses the builder via `from_parts` (the base
  constraint exists precisely to flag such schemas). `Record` refs unvalidated here (that's
  `constraints`' job). **No stable binary format yet** — serde is an explicit stop-gap.

### crates/constraints

- **Purpose** — The validation engine: turns opaque `Constraint` annotations into real
  checks, running always-on base constraints + caller-supplied constraints in **one walk**.
- **Public surface** (`src/lib.rs`): `trait Constraint: Visitor + Default { const NAME }`;
  `trait Constraints: Visitor + Default { const NAMES }` (impl'd for `()` and tuples 1..=12);
  `fn validate<C: Constraints>(&Schema) -> Report<C::Output>`;
  `Report { base_constraints: Result<(),BaseConstraintsError>, unregistered_constraints: Vec<String>, results: R }`;
  `BaseConstraintsError { invalid_references, recursive_records, entities_without_events }`;
  `mod utils::bullet_list`.
- **Patterns** — `validate` builds a fixed 5-tuple visitor `(UnresolvedReferences,
  UnregisteredConstraints, RecursiveRecords, EntitiesWithoutEvents, C::default())` and walks
  once. Recursion detection uses `petgraph` `tarjan_scc` + a manual self-loop pass. Only
  `constraints()` are checked for registration — `metadata` is never validated. Error
  aggregation idiom (0→Ok / 1→err / >1→`Multiple`) recurs across the whole set.
- **Deps** — `schema` (feat `visitor`), `petgraph`, `rustc-hash`.
- **Gotchas** — Base constraints signal *internally inconsistent* schemas, distinct from
  user-constraint results. Generic over arity ≤12 tuples only.

### crates/ref-target

- **Purpose** — `quent.ref-target.v0.1.0`: restrict which entity type an `EntityRef` targets.
- **Surface** — `RefTarget(Identifier)` newtype with `from_annotations(&Annotations)->Option<Self>`
  (lenient read path — swallows errors); `RefTargetConstraint` (Visitor validator);
  `RefTargetError {InvalidData, UnknownTarget, Multiple}`.
- **Deps** — `schema` (feat visitor), `constraints`.
- **Gotchas** — `from_annotations` returns `None` on missing/bad data (used by ref-tree);
  diagnostics only via the `Visitor` path. Only inspects `EntityRef` nodes.

### crates/ref-tree

- **Purpose** — `quent.ref-tree.v0.1.0`: mark refs tree-forming; validate the entity/reference
  graph is a single tree rooted at exactly one entity connecting all entities.
- **Surface** — `RefTreeConstraint` (Visitor); `RefTreeError {NotTargetConstrained,
  UnknownTarget, MultiplePerEvent, MultipleRefsInRecord, NoRoot, MultipleRoots,
  ConflictingParents, Unreachable, Multiple}`.
- **Patterns** — Builds a `petgraph::Graph` of `Node {Entity, Event(entity,event), Record}`;
  `finish()` runs deferred checks (per-event ref count via DFS through nested records; per-entity
  unique parents; root count; BFS reachability from root). Requires a ref-target (depends on
  ref-target to force a concrete target — type-erased refs may not carry this constraint).
- **Deps** — `schema`, `constraints`, `ref-target`; `petgraph`, `rustc-hash`.
- **Gotchas** — A parent ref may appear on *any number* of events (intentional; disambiguation
  deferred to producers/consumers — see the design note, do not tighten). Cycles surface as
  `NoRoot` or per-entity `Unreachable`, not a dedicated cycle error. The module doc-comment
  requirements list is the source of truth over looser inline test comments.

### crates/fsm

- **Purpose** — `quent.fsm.v0.1.0`: model an entity's event-ordering as an FSM (states = event
  names) and validate the topology; also a builder that produces a valid FSM entity.
- **Surface** — `Fsm { initial_state, transitions: Vec<Transition>, exit_from_states }`
  (serde = the JSON payload; `cardinality(state)->Option<Cardinality>`: Multi if on a cycle);
  `FsmConstraint` (Visitor); `FsmError {InvalidData, ReservedStateName, UnreachableFromInit,
  CannotReachExit, UnknownState, UncoveredEvent, CardinalityMismatch, Multiple}`;
  builder re-exports `FsmEntityBuilder`, `StateDecl {name, attributes, to, initial, exit}`,
  `FsmEntityBuilderError`.
- **Patterns** — 7 documented requirements enforced in `check_entity`: reserved `exit` name
  (case-insensitive), state↔event bijection, reachability from init, every state reaches exit,
  cardinality matches cycle membership. Graph via `petgraph` `DiGraphMap` with synthetic
  `Init`/`Exit` nodes; `tarjan_scc` + **manual self-loop pass** (tarjan misses single-node
  self-loops). `ExitStates` structurally guarantees ≥1 exit. Builder re-runs `check_entity`
  so a built entity is always valid.
- **Deps** — `schema` (feat serde+visitor), `constraints`; `petgraph`, `serde_json`.
- **Gotchas** — Self-loop handling must survive any graph refactor. TODO: "allow FSMs to have
  freestanding events" (currently every entity event must be an FSM state) — relevant if
  phase-2 event work wants to relax it. Payload format is a shared external contract.

### crates/resource

- **Purpose** — `quent.resource.v0.1.0`: entities declare `Capacity` values; FSM-only entities
  claim them via usage records on entity references; bounded capacities need a bounds record.
- **Surface** — `Capacity {kind: CapacityKind {Occupancy, Rate}, bounded}`; `Resource` enum
  `{Definition(Capacities), Bounds{resource}, Usage{resource}}` (serde = payload);
  `ResourceConstraint` (Visitor); `ResourceError {InvalidData, MisplacedRole, UnknownResource,
  UndeclaredCapacity, UsageNotOnReference, NonFsmUser, ForeignBounds, UnboundedCapacity,
  UncoveredCapacity, UnexpectedBounds, MissingBounds, Multiple}`; builder re-exports
  `ResourceBuilder`, `ResourceParts {definition, usage, bounds}`, `BuildError`.
- **Patterns** — Two-phase visitor: `visit()` collects state (resources/usage/bounds/record-refs),
  cross-element requirements resolved in `finish()`. `Capacities = IndexMap` (name uniqueness +
  order). Unit resource = no capacities. Roles decoded by location (Definition=entity,
  Usage/Bounds=records).
- **Deps** — `schema`, `constraints`, **`fsm`** (only reads `FsmConstraint::NAME` to check the
  user entity is an FSM — the sole fsm→resource coupling); `indexmap`, `rustc-hash`, `serde_json`.
- **Gotchas** — FSM-only-users is the hard cross-crate rule; it checks only for the *presence*
  of the FSM constraint annotation, not topology validity. Doc note: end-of-usage enforcement is
  currently only possible for FSM entities and the exit state cannot hold attributes — a known
  limitation to be aware of when extending resource semantics.

### crates/yaml

- **Purpose** — `quent-yaml`: parse a YAML event-model source (format version **`alpha`**) and
  lower it into a validated `Schema`, returning `Parsed { schema, warnings }` or rich `Diagnostics`.
- **Surface** — `parse_from_str(src, source: Option<&str>) -> Result<Parsed, Error>`,
  `parse_from_file(path)`; `Error {Io, Invalid(Diagnostics)}`; re-exports `Diagnostic`,
  `Diagnostics`, `Origin {Location{line,column}, Path(String), Whole}`. Binary `quent-yaml-check`
  (`src/bin/check.rs`).
- **YAML accepted** — top-level `quent: alpha`, `model:`, `doc:`, `constraints:`, `metadata:`,
  `records:`, `entities:` (events with `multi: bool`, attributes), `fsms:` (states with
  `initial`, `attributes`, `to: [...]`, reserved `exit` target). Field types: builtins
  `bool/u8..u64/i8..i64/f32/f64/string/uuid/dynamic/ref`, bare name = record ref, `{list:T}`,
  `{option:T}`, `{ref:E, data?:T}`, `{scope-ref:E, data?:T}` (tree-forming). `dynamic` lowers to
  `DataType::DynamicRecord`.
- **Patterns** — Two-phase (serde-saphyr deserialize into AST → `lower()` builds via schema
  builders). **Error-accumulating** into one `&mut Diagnostics` sink (no `?`) so a run surfaces
  *all* problems. Post-lower `validate::<(RefTargetConstraint, RefTreeConstraint, FsmConstraint)>`;
  unregistered constraints → non-fatal warnings. `#[serde(deny_unknown_fields)]` throughout.
- **Deps** — `schema`, `constraints`, `fsm`, `ref-target`, `ref-tree`; `serde-saphyr`, `indexmap`,
  `tracing` (the `check` bin).
- **Gotchas** — FSM constraint is **builder-only** (hand-writing `quent.fsm.*` in `constraints:`
  is an error; FSMs must go through `fsms:`). Ref-target existence + scope-tree validity checked
  only in the post-lower validate pass — a new reference kind needs a registered validator or it
  only warns. No support yet for: includes/imports, enum/union types, defaults, comments-as-docs.

---

## Cluster B — runtime

### crates/events  *(leaf)*

- **Purpose** — Generic event envelope + entity marker trait. Single 53-line file.
- **Surface** — `trait EntityEvent { const NAME: &'static str }`;
  `struct Event<T> { id: Uuid, timestamp: TimeUnixNanoSec (u64), data: T }` with
  `new_now(id, data)` (stamps current clock) / `new(id, timestamp, data)`; `impl<T> Timestamp for Event<T>`.
- **Deps** — none in-set; `quent-time` (support crate, not one of the 11), `uuid`, optional `serde`.
- **Gotchas** — `Event<T>` does **not** bound `T: EntityEvent`. `serde` is optional (`default=[]`);
  `uuid` always compiled with serde. `quent-time` TODO: reserve `u64::MAX` as a missing/open-ended
  timestamp sentinel — relevant if phase-2 introduces open-ended spans. No tests.

### crates/dynamic-attributes  *(leaf)*

- **Purpose** — Self-describing, runtime-keyed attribute model (keys known only at runtime).
  Renamed + re-prefixed `Dynamic*` in **#418** (`refactor(attributes)!`) to signal the perf cost
  vs static schema attributes.
- **Surface** — `DynamicAttribute {key: String, value: Option<DynamicValue>}`;
  `DynamicAttributes(Vec<DynamicAttribute>)` (ordered `Vec`, `serde(transparent)`);
  `DynamicValue {U8..U64, I8..I64, F32, F64, String, Struct(DynamicStruct), List(DynamicList)}`;
  `DynamicList` (homogeneous typed lists; no nested `List`); `DynamicStruct`; typed constructors
  (`null/u8../string/structure/list`); collection helpers (`add_*`, incl. `add_bool` → `U8(0|1)`);
  numeric extraction `TryFrom<DynamicValue> for f64` (lossy widening); `DynamicValueError {NotNumeric}`.
- **Deps** — none in-set; `thiserror`, optional `serde`/`ts-rs`. Features `default=[]`, `serde`,
  `ts` (implies serde). No tests.
- **Gotchas** — `DynamicAttributes` is a **`Vec`, not a map** (dup keys allowed, O(n) lookup,
  order-preserving). **No `Bool` variant** (bool overloads `U8`). Only `f64` numeric bridge, lossy;
  no integer-preserving extraction. No `Eq`/`Hash` (float variants).

### crates/instrumentation  *(active runtime)*

- **Purpose** — The event pipeline backing generated instrumentation libraries:
  `Context` → `Observer`/`EventSender` → per-instance `Handle`, bridging a **sync caller API**
  to an **async exporter-fed forwarder** task.
- **Surface** (`src/lib.rs`, all `#[doc(hidden)]`): `Context` (`try_new(Uuid)`, `noop(Uuid)`,
  `block_on`, async `observer<T>(provider: impl ExporterProvider<T>) -> Result<Observer<T>>`);
  `Observer<T>` (`noop`, `send`, `emit(id, impl Into<T>)`; Drop cancels+drains+flushes);
  `EventSender<T>` (`noop`, `send`, `emit`; cloneable); `Handle<E>` (`new`, `with_id`, `emit`,
  `emit_once::<INDEX>`, `is_emitted::<INDEX>`) + `HandleError`; `EntityRef<E,T=()>` + `AnyEntity`;
  `write_sidecar(...)`. Bare re-exports for generated code: `build_info`, `DynamicAttributes`,
  `EntityEvent`, `Event`, `ExporterOptions`, `Uuid`, (feat) `EventCallback`.
- **Pipeline** — `Context::observer` builds the exporter *synchronously* (errors surface eagerly),
  then `spawn_forwarder` creates an unbounded mpsc + `CancellationToken` and spawns a task
  (`recv_many` batch loop; on cancel: drain + `exporter.shutdown()`). Hot path = build `Event<T>`
  + push to mpsc.
- **Runtime model** — `BackendRuntime::Borrowed` (adopt ambient runtime) or `Owned` (spawn one);
  shared via `Arc`; torn down with `shutdown_background` (never a blocking drop). Observer keeps
  the Owned runtime alive so drop-flush is valid after `Context` is gone.
- **Deps** — in-set: **`events`**, **`dynamic-attributes`**. Legacy/support: `quent-io`
  (`Exporter`/`ExporterProvider`/`ExporterOptions` + backends), `quent-build-info`, `tokio`,
  `tokio-util`. Features: exporter backends `io-{callback,ndjson,msgpack,postcard,collector}`
  forward to `quent-io/*`; `serde` fans out to events/dynamic-attributes/uuid serde.
- **Gotchas / phase-2** —
  - **Panics on a current-thread tokio runtime** at any sync→async crossing (`observer()` build,
    drop-flush). Documented + tested; generated contexts inherit it.
  - `emit_once` caps at **64 once-events per handle** (u64 bit word).
  - **`Observer::sender(&self) -> EventSender<T>`** was added in PR #402 (merged 2026-07-22), cloning the
    inner `EventSender` for use in `'static` hook closures. Also exposes `send`/`emit`; generated code
    wraps `Arc<Observer<E>>` in a `Handle`.

### crates/instrumentation-build  *(#191 preliminary — codegen)*

- **Purpose** — Consume a `Schema` and emit Rust source for a full instrumentation library
  targeting `::quent_instrumentation::*`. For use from a consumer's `build.rs` into `OUT_DIR`.
- **Surface** — `generate(&Schema, &Options) -> Result<GenerateInfo, GenerateError>` (writes file),
  `generate_str(...) -> Result<String, ...>`; `Options {event_derives, record_derives: &'static
  [&'static str], out_dir, file_name, any_event: bool}`; `GenerateError {InvalidSchema,
  InvalidDerive, InvalidGeneratedCode, TooManyOnceEvents, Io}`.
- **Emits** — per-entity `{Entity}Event` enum + `EntityEvent` impl; per-record struct;
  optional `AnyEvent<'a>` (type-erased downcast for callback exporters); and a runtime surface:
  `{Schema}Context` (`try_new(Option<ExporterOptions>)`), `{Entity}Observer` (`.handle()`),
  `{Entity}Handle` (once-events `&mut self` claiming successive bits + `{event}_emitted()`,
  multi-events `&self`). Uses `quote`/`syn`/`prettyplease`.
- **Deps** — in-set: `schema`, `constraints`, `ref-target` (reads `RefTargetConstraint` for the
  `EntityRef` marker type). **Does NOT depend on `instrumentation`** — the runtime is named as
  text tokens only; the cargo edge is on the *generated consumer*. External: `convert_case`,
  `prettyplease`, `proc-macro2`, `quote`, `syn`.
- **Gotchas** — Generated code hard-requires the consumer to depend on `quent-instrumentation`
  with a `Serialize`-providing derive in both derive lists (not checked here). Once-events capped
  at 64/entity; type nesting capped at 64 (`map_data_type` panics past it). `{Schema}Context::try_new`
  inherits the runtime current-thread panic. The `example/` sub-crate is its **own workspace**
  (callback-only, `Serialize`-free) and demos `quent-yaml` → `generate` end-to-end. TODO: typestate
  FSM handles planned (issue #416).

---

## Cross-cutting patterns (apply these when extending)

1. **Visitor + Constraint + one-walk validation.** Every constraint is a `Visitor` with a
   versioned `NAME`; `validate::<(A,B,...)>()` runs base + user constraints in a single
   `schema.walk`. Tuple arity ≤ 12.
2. **Builder triad** `set_* / try_insert_* / try_with_*` across schema, fsm, resource builders;
   fallible builders re-validate on `build()`.
3. **Error aggregation idiom** `0 → Ok / 1 → that error / >1 → Multiple(Vec<E>)` — ref-target,
   ref-tree, fsm, resource, constraints, and yaml's diagnostics sink all use it.
4. **Ordered maps** `IndexMap<_, _, FxBuildHasher>` everywhere (declaration order is semantically
   significant and preserved end-to-end: YAML → schema → codegen).
5. **Opaque annotations carrying JSON payloads.** `Constraint`/`Metadata` store opaque UTF-8;
   fsm/resource serialize their config structs to JSON into `Constraint.data`. This is a shared
   external contract between the constraint validator, the builder, and YAML/codegen producers.
6. **Optional serde, optional ts.** `serde` is feature-gated and treated as a stop-gap (no stable
   binary format). `ts` implies serde and generates TypeScript bindings (schema, dynamic-attributes).
7. **Sync→async isolation** in the runtime: all IO/serialization happens off the caller thread in
   the forwarder task; the caller only builds an `Event<T>` and pushes it.

## Phase-2-relevant notes & gaps

- **`Observer::sender(&self) -> EventSender<T>` was added in PR #402** (merged 2026-07-22) and is now
  in `crates/instrumentation/src/observer.rs`. The gap noted at map-write time is resolved. The cloned
  `EventSender<T>` lets a `'static` hook closure forward NVTX events into an app-owned observer that
  still flushes on drop. See `integrations/nvtx/example/src/lib.rs` for the usage pattern.
- **New NVTX integration crates** (landed 2026-07-22, under `integrations/nvtx/`):
  - `nvtx-bridge` — `NvtxEventEntity`, a `#[serde(transparent)]` newtype over `NvtxEvent` implementing
    `EntityEvent` (NAME = `"NvtxEvent"`). Orphan-rule adapter so `nvtx-events` stays Quent-agnostic.
  - `nvtx-example` — Runnable wiring: builds `Context` + `Observer<NvtxEventEntity>`, installs injection
    hook via `observer.sender()`, emits via `nvtx` crate macros, drops observer to flush. Integration
    test asserts all 6 core NVTX kinds round-trip through ndjson. Canonical example of the in-process
    capture pattern. (`members`-only, not `default-members`.)
- **Two distinct "Event" concepts** — `schema::Event` (an event *type* in the model, with
  `Cardinality`) vs `events::Event<T>` (a runtime event *envelope*). Don't conflate them.
- **Cluster A ↔ Cluster B only meet at codegen.** Any phase-2 work that needs the validated
  event-model (fsm/resource/refs) to influence runtime behavior must go through
  `instrumentation-build` (schema → generated source), not a direct crate dependency.
- **Runtime current-thread-panic** is a hard constraint for any host embedding the pipeline
  (including NVTX in-process capture): construction and drop-flush must not happen on a
  current-thread tokio runtime.
- **Capacity/timestamp caps**: 64 once-events/entity, 64 type-nesting depth, and the pending
  `u64::MAX` open-ended-timestamp sentinel in `quent-time` are all worth respecting in new work.

## Source

Full-source deep read of each crate's `Cargo.toml`, `lib.rs`, every module, `build.rs`, and tests,
2026-07-22. Legacy crates (`analyzer`, `build-info`, `codegen`, `collector`, `io`, `model`,
`model-macros`, `open`, `stdlib`, `time`, `ui`) intentionally out of scope except where a current
crate depends on them (`instrumentation` → `quent-io`, `quent-build-info`; `events` → `quent-time`).
