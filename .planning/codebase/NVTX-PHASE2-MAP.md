<!-- refreshed: 2026-07-22 -->
# NVTX Phase 2 Codebase Map

**Analysis Date:** 2026-07-22
**Scope:** Domain model pattern, domain analyzer pattern, analyzer framework FSM internals,
NVTX event vocabulary from Phase 1.

---

## 1. Domain Model Pattern — `domains/query_engine/model/`

The query engine model is the canonical reference for building a new domain model.
Every file in it must be read before writing an NVTX model.

### Top-level wiring — `domains/query_engine/model/src/lib.rs`

```rust
use quent_model::model;

model! {
    name: QueryEngine,
    root: engine::Engine,
    entities: {
        query::Query,
        worker::Worker,
        query_group::QueryGroup,
        plan::Plan,
        operator::Operator,
        port::Port,
    },
}
```

The `model!` macro names the model, declares its root entity (must be `Root = true`), and lists
all other entities. Each entity module is `pub mod` in `lib.rs`.

### Two entity macro flavors

**`entity!` macro** — for entities without an FSM (no states/transitions). Used for `Engine`,
`Worker`, `QueryGroup`, `Plan`, `Operator`, `Port`.

Pattern (from `domains/query_engine/model/src/engine.rs`):
```rust
// Event payloads: derive Attributes + Serialize + Deserialize
#[derive(Debug, Default, Attributes, Deserialize, Serialize)]
pub struct Init { ... }

#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct Exit;

entity! {
    Engine: ResourceGroup<Root = true> {
        declaration: init,           // which event is the "declaration" event
        events: {
            init: Init,
            exit: Exit,
        },
    }
}
```

`declaration: event_name` names the event that declares/identifies the entity instance.
`Root = true` on exactly one entity marks it as the model root.

**`fsm!` + `state!` macros** — for entities modeled as finite state machines. Used for `Query`
(the only FSM entity in the query engine model).

Pattern (from `domains/query_engine/model/src/query.rs`):
```rust
use quent_model::{Ref, fsm, state};

// Each state is a struct with optional typed attributes
state! {
    Init {
        attributes: {
            query_group_id: Ref<super::query_group::QueryGroup>,
        },
    }
}
state! { Planning {} }
state! { Executing {} }

// entry: is the first state name; exit_from: lists states from which exit is valid
fsm! {
    Query: ResourceGroup {
        states: { init: Init, planning: Planning, executing: Executing },
        entry: init,
        exit_from: { executing },
        transitions: {
            init => planning,
            planning => executing,
        },
    }
}
```

### Cross-entity references — `Ref<T>`

`quent_model::Ref<T>` is used in event payload structs to reference another entity by UUID:
```rust
pub struct Init {
    pub query_group_id: Ref<super::query_group::QueryGroup>,
}
```
`Ref<T>.uuid()` returns the inner `Uuid` at analysis time.

### NVTX model implication

NVTX does **not** use the `entity!`/`fsm!` proc-macro pattern for its model. The proc macros
generate typed per-entity event streams from the instrumentation layer. NVTX arrives as a single
flat stream of heterogeneous `NvtxEvent` variants (`NvtxEventEntity`). The NVTX model therefore
needs **custom entity types** in the analyzer layer rather than macro-generated ones.

The model `lib.rs` for NVTX will still use `model!` to register the model name and the
`NvtxEventEntity` stream (via `quent_model::model!`), but the per-entity logic is handwritten in
the analyzer. See Section 4 for the event vocabulary that the analyzer must consume.

---

## 2. Domain Analyzer Pattern — `domains/query_engine/analyzer/`

### Core types

**`InMemoryQueryEngineModelBuilder`** (`domains/query_engine/analyzer/src/model.rs`):
```rust
pub struct InMemoryQueryEngineModelBuilder {
    engine: Engine,
    workers: HashMap<Uuid, Worker>,
    query_groups: HashMap<Uuid, QueryGroup>,
    queries: HashMap<Uuid, QueryBuilder>,   // QueryBuilder = FsmEventsBuilder<ModelQueryTransition>
    plans: HashMap<Uuid, Plan>,
    operators: HashMap<Uuid, Operator>,
    ports: HashMap<Uuid, Port>,
}
```
Ingestion method:
```rust
pub fn try_push(&mut self, event: Event<QueryEngineEvent>) -> AnalyzerResult<()>
```
Dispatches on the event enum variant, creates entity entries on first-seen IDs, and calls
`entity.push(...)` on the appropriate bucket.

Finalization:
```rust
pub fn try_build(self) -> AnalyzerResult<InMemoryQueryEngineModel>
```
This is where FSM builders are finalized — `Query::from_builder(v)` calls
`FsmEventsBuilder::try_build()` which sorts transitions and constructs `FsmEvents`. If any
`QueryBuilder` fails (e.g., nil id), the whole `try_build` returns that error.

**`InMemoryQueryEngineModel`** — the built model:
```rust
pub struct InMemoryQueryEngineModel {
    pub engine: Engine,
    pub workers: HashMap<Uuid, Worker>,
    pub queries: HashMap<Uuid, Query>,
    ...
}
```
Implements `QueryEngineModel`, `Model`, `ResourceCollection`.

### Per-entity analyzer types

**Entity entities** (no FSM) use `EntityEvents<ModelType>`:
- `engine.rs`: `Engine(EntityEvents<engine::Engine>)` — tracks earliest/latest timestamp,
  stores event data in `M::Data`. `to_ui()` reads `data().init.as_ref()`.
- `worker.rs`: `Worker(EntityEvents<worker::Worker>)` — same pattern.
- `operator.rs`: `Operator { inner: EntityEvents<operator::Operator>, active_span: Option<...> }` —
  extra computed field layered on top of the generic storage.

**FSM entities** use `FsmEvents<T>` via `FsmEventsBuilder<T>`:
- `query.rs`: `QueryBuilder = FsmEventsBuilder<ModelQueryTransition>`. On build:
  `Query::from_builder(builder)` calls `builder.try_build()`. The resulting `Query` wraps
  `FsmEvents<ModelQueryTransition>` and delegates all `Fsm` trait methods.
  `query.rs` extracts `query_group_id` from the first transition's data.

### `QueryEngineModel` trait — `domains/query_engine/analyzer/src/lib.rs`

```rust
pub trait QueryEngineModel: Model {
    fn engine(&self) -> AnalyzerResult<&Engine>;
    fn query(&self, query_id: Uuid) -> AnalyzerResult<&Query>;
    ...
    fn queries(&self) -> impl Iterator<Item = &Query>;
    ...
    fn plan_tree(&self, query_id: Uuid) -> AnalyzerResult<PlanTree>;
    // default methods: query_plans, query_workers, query_epoch,
    //                  plans_operators, operators_ports
}
```

The NVTX equivalent trait (`NvtxModel`) will expose iterators over domains, threads, push-ranges,
start-ranges, and marks.

### `UiAnalyzer` trait — `domains/query_engine/analyzer/src/ui.rs`

```rust
pub trait UiAnalyzer {
    type Event;
    type EntityRef;

    fn try_new(engine_id: Uuid, events: impl Iterator<Item = Event<Self::Event>>)
        -> AnalyzerResult<Self> where Self: Sized;

    fn extract_engine(...) -> AnalyzerResult<ui::Engine>;
    fn query_bundle(&self, query_id: Uuid) -> AnalyzerResult<ui::QueryBundle<Self::EntityRef>>;
    fn query_engine_model(&self) -> &impl QueryEngineModel;
    fn single_resource_timeline(...) -> AnalyzerResult<SingleTimelineResponse>;
    fn list_entities(...) -> AnalyzerResult<EntityListResponse>;
    fn bulk_resource_timeline(...) -> AnalyzerResult<BulkTimelinesResponse>;
    fn bulk_chunked_resource_timeline(...) -> AnalyzerResult<BulkChunkedTimelinesResponse>;
    fn data_flow_timeline(...) -> AnalyzerResult<...> { Err(AnalyzerError::Unsupported) }
}
```

The Phase 2 NVTX analyzer implements `UiAnalyzer` with `type Event = NvtxEventEntity`.

`QuentViewer` trait (same file) must be implemented on a type named `Viewer` at the crate root.
It connects the event importer to the `UiAnalyzer` for `quent-open`.

### `list_entities` generic helper — `domains/query_engine/analyzer/src/entities.rs`

```rust
pub fn list_entities<M, P>(model: &M, keep: P, query: ListQuery<'_>) -> AnalyzerResult<EntityListResponse>
where
    M: FsmCollection,
    M::Fsm: for<'a> FsmUsages<'a>,
    P: Fn(&M::Fsm) -> bool,
```

If the NVTX model exposes `FsmCollection`, this function can be reused directly to list ranges.

---

## 3. Analyzer Framework — `crates/analyzer/src/`

### Top-level traits — `crates/analyzer/src/lib.rs`

```rust
pub trait Entity {
    fn id(&self) -> Uuid;
    fn type_name(&self) -> &str;
    fn instance_name(&self) -> &str;
}

pub trait Span {
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec>;
}

pub trait Model: ResourceCollection {
    type EntityIdType: EntityId;
    fn try_entity_ref(&self, entity_id: Uuid) -> AnalyzerResult<Self::EntityIdType>;
    fn root(&self) -> AnalyzerResult<&impl ResourceGroup>;
    fn resource_tree(&self) -> AnalyzerResult<ResourceTreeNode> { ... }
}
```

### Error variants — `crates/analyzer/src/error.rs`

```rust
pub enum AnalyzerError {
    Importer(quent_model::io::ImporterError),
    Validation(String),         // general validation (wrong last state, duplicate id, …)
    InvalidId(Uuid),            // unknown entity/resource id
    InvalidTypeName(String),
    Time(quent_time::TimeError),
    ValueType(String),
    BrokenImpl(&'static str),
    IncompleteEntity(String),   // entity missing required events (no exit, no timestamps)
    IncompleteFsm(String),      // FSM-specific incompleteness (declared but unused)
    FsmExitTransitionConversion,
    InvalidArgument(String),
    Unsupported,                // returned as HTTP 501 by the server
}
```

`IncompleteEntity` and `IncompleteFsm` are the variants to use for unclosed NVTX ranges.
`Unsupported` is the correct return for optional `UiAnalyzer` methods not yet implemented.

### `Fsm` + `FsmUsages` traits — `crates/analyzer/src/fsm/mod.rs`

```rust
pub trait Fsm: Entity {
    type TransitionType: Transition;
    fn len(&self) -> usize;                              // number of states (= transitions - 1)
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType>;
    fn state<'a>(&'a self, index: usize) -> Option<FsmStateRef<'a, Self, Self::TransitionType>>;
    fn states<'a>(&'a self) -> impl ExactSizeIterator<...>;
    fn first<'a>(&'a self) -> Option<...>;
    fn last<'a>(&'a self) -> Option<...>;
}
```

`Span` is blanket-implemented for all `Fsm` types:
```rust
impl<U: Fsm> Span for U {
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        if let Some(start) = self.first().map(|s| s.span().start())
            && let Some(end) = self.last().map(|s| s.span().end())
        { Ok(SpanUnixNanoSec::try_new(start, end)?) }
        else { Err(AnalyzerError::IncompleteEntity(...)) }
    }
}
```

An empty FSM (zero transitions, so `len() == 0`) returns `AnalyzerError::IncompleteEntity` from
`span()` — it does not panic.

`FsmStateRef::span()` calls `self.fsm.transition(self.index + 1).unwrap()` — safe only when
`index < self.len()`, which `state(index)` enforces.

### `RtFsmBuilder` and what `try_build` requires — `crates/analyzer/src/fsm/runtime.rs`

`RtFsmBuilder<RtFsmTransition>` is the builder for **runtime-defined** FSMs (not macro-generated).
The NVTX analyzer will likely use this or a similar custom path.

**`RtFsmBuilder::try_build()` requirements** (lines 118–143):
1. `transitions` must be non-empty — `transitions.last().unwrap()` at line 122 **panics** if empty.
2. The last transition's `name` must be `"exit"` — returns `AnalyzerError::Validation` otherwise.
3. `type_name` must be set — returns `AnalyzerError::IncompleteEntity` if `None`.
4. `instance_name` must be set — returns `AnalyzerError::IncompleteEntity` if `None`.

```rust
pub fn try_build(self) -> AnalyzerResult<RtFsm> {
    let transitions = self.transitions.into_inner();
    let last_name = &transitions.last().unwrap(/*len checked above*/).name; // PANICS if empty!
    if last_name != "exit" {
        Err(AnalyzerError::Validation(...))
    } else {
        Ok(RtFsm {
            id: self.id,
            type_name: self.type_name.ok_or_else(|| AnalyzerError::IncompleteEntity(...))?,
            instance_name: self.instance_name.ok_or_else(|| AnalyzerError::IncompleteEntity(...))?,
            transitions,
        })
    }
}
```

**NVTX failure modes that hit these conditions:**
- `RangePush` with no matching `RangePop` → no "exit" transition → `Validation` error (not panic,
  because at least one transition exists from the push).
- Zero transitions (an FSM builder created but never fed any event) → **panic** at `unwrap()`.
- An FSM whose type/instance name was never set → `IncompleteEntity`.

**`RtFsmsBuilder::try_build()` — the TODO at lines 310–312:**
```rust
pub fn try_build(self) -> AnalyzerResult<InMemoryFsms<RtFsm>> {
    for (k, fsm) in self.fsms.into_iter() {
        // TODO(johanpel): for now bubble up this error but if there are
        // e.g. abrupt failures we may want to move incomplete FSMs into
        // their own bucket.
        let fsm = fsm.try_build()?;   // propagates first error, stops all FSMs
        ...
    }
    ...
}
```

The `?` means the first incomplete FSM (unclosed range) aborts reconstruction of *all* FSMs.
Phase 2 must change this: either skip incomplete FSMs with a `tracing::warn!`, store them in a
separate bucket, or — simpler — the NVTX analyzer synthesizes its own "exit" transitions for
unclosed ranges before calling `try_build`.

**Additional panic point in `FsmUsages for RtFsm`:**
```rust
fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)> {
    self.transitions.windows(2).flat_map(|window| {
        let span = SpanUnixNanoSec::try_new(start, end).unwrap(); // panics if end < start
        ...
    })
}
```
`TimeOrderedCollector` prevents this in practice by always returning timestamps in
non-decreasing order, but equal timestamps (two events at exactly the same nanosecond) produce a
zero-length span. `SpanUnixNanoSec::try_new` may panic or error on zero-duration — verify before
producing duplicate-timestamp transitions.

### `TimeOrderedCollector` — `crates/time/src/lib.rs` lines 121–165

```rust
pub struct TimeOrderedCollector<T>(Vec<T>);

impl<T: Timestamp> TimeOrderedCollector<T> {
    pub fn push(&mut self, state: T) {
        if let Some(last) = self.0.last()
            && last.timestamp() <= state.timestamp()
        {
            self.0.push(state);                    // O(1) — common case, in-order arrival
        } else {
            let pos = self.0.partition_point(|s| s.timestamp() < state.timestamp());
            self.0.insert(pos, state);             // O(log n + n) — out-of-order insertion
        }
    }
    pub fn into_inner(self) -> Vec<T> { self.0 }  // returns sorted vec
}
```

**Key properties for Phase 2:**
- Out-of-order events are handled transparently — no panic, no data loss.
- Duplicate timestamps are stable: equal-timestamp items are appended in insertion order (because
  `partition_point(|s| s.timestamp() < state.timestamp())` points just before any equal-timestamp
  items, so a new duplicate goes before them; in-order equal-timestamp pushes go after via the fast
  path). This is fine for NVTX: two events at the same nanosecond preserve arrival order.
- The collector is used inside `RtFsmBuilder` and `FsmEventsBuilder` — so both `RtFsm`-based and
  `FsmEvents`-based FSM paths sort transitions before `try_build`.

### `FsmEventsBuilder` / `FsmEvents` — `crates/analyzer/src/fsm/events.rs`

Used by macro-generated FSM entities (like `Query`). Different from `RtFsmBuilder`:
```rust
pub fn try_build(self) -> AnalyzerResult<FsmEvents<T>> {
    let transitions: SmallVec<...> = self.transitions.into_inner().into();
    Ok(FsmEvents { id: self.id, instance_name: self.instance_name, transitions })
}
```
`FsmEventsBuilder::try_build()` does **not** check for an exit state or minimum transitions —
it succeeds even with zero transitions. `FsmEvents::len()` returns `transitions.len().saturating_sub(1)`,
so an empty `FsmEvents` has `len() == 0` and `span()` returns `IncompleteEntity` gracefully.

This approach (always succeed at build time, propagate incompleteness lazily via `span()`) is
an alternative the NVTX analyzer could adopt for its range entities.

### `EntityEvents<M>` — `crates/analyzer/src/entity/mod.rs`

Generic accumulator for entity (non-FSM) types:
```rust
pub struct EntityEvents<M: EntityData> {
    id: Uuid,
    earliest_timestamp: Option<TimeUnixNanoSec>,
    latest_timestamp: Option<TimeUnixNanoSec>,
    data: M::Data,   // generated struct with one Option<EventType> per event kind
}

pub fn push(&mut self, event: Event<M::Event>) {
    // updates earliest/latest timestamps
    M::push(&mut self.data, event.data);   // sets the appropriate Option field
}
```

For NVTX domain/thread entities, the equivalent can be a simple hand-written struct (no macro
needed) since the events are not typed by the proc-macro system.

---

## 4. NVTX Event Vocabulary — `integrations/nvtx/`

### `NvtxEvent` enum — `integrations/nvtx/events/src/lib.rs`

All capture is verbatim: handles are raw integers, nothing is resolved at capture time.

```
Variant              | Key fields                                    | Notes
---------------------+-----------------------------------------------+---------------------------
RangePush            | domain: u64, attributes: NvtxEventAttributes  | opens thread-local range
RangePop             | domain: u64                                   | closes innermost push
RangeStart           | domain: u64, range_id: u64, attributes: ...   | opens process-wide range
RangeEnd             | domain: u64, range_id: u64                    | closes matching RangeStart
Mark                 | domain: u64, attributes: ...                  | instantaneous, no pairing
DomainCreate         | domain: u64, name: String                     | registers domain handle
DomainDestroy        | domain: u64                                   | releases domain handle
RegisterString       | domain: u64, handle: u64, string: String      | registers string handle
NameCategory         | domain: u64, category: u32, name: String      | names a (domain, category)
NameThread           | thread_id: u32, name: String                  | names an OS thread
ResourceCreate       | domain: u64, handle: u64, identifier_type: i32,| associates resource handle
                     | identifier: u64, message: Option<NvtxMessage>|
ResourceDestroy      | handle: u64                                   | releases resource handle
```

Default domain `0` is used for CORE (non-domain-scoped) NVTX calls.

### `NvtxEventAttributes` — `integrations/nvtx/events/src/attributes.rs`

```rust
pub struct NvtxEventAttributes {
    pub category: u32,               // 0 = none; namespaced per domain
    pub color: Option<NvtxColor>,    // raw ARGB value
    pub message: Option<NvtxMessage>,
    pub payload: Option<NvtxPayload>,
}

pub enum NvtxMessage {
    String(String),           // immediate string, copied verbatim
    RegisteredHandle(u64),    // handle → resolved via RegisterString events
}
```

### `NvtxPayload` — `integrations/nvtx/events/src/payload.rs`

```rust
pub struct NvtxPayload {
    pub payload_type: i32,       // raw NVTX_PAYLOAD_TYPE_* tag
    pub value: NvtxPayloadValue, // UnsignedInt64 | Int64 | Double | UnsignedInt32 | Int32 | Float | Pointer
}
```

Payload decoding is deferred; Phase 2 should store it verbatim and expose it as a raw field.

`PayloadExtensionEvent` (schema/enum registration, binary blobs) is **not wired** into `NvtxEvent`
and no capture path emits it — ignore in Phase 2.

### Handle resolution map for the NVTX analyzer

The analyzer must build these lookup tables while scanning the event stream:

```
Handle type                 | Source event(s)        | Key                | Value
----------------------------+------------------------+--------------------+-----------------
Domain name                 | DomainCreate           | domain: u64        | String
Registered string           | RegisterString         | (domain, handle)   | String
Category name               | NameCategory           | (domain, category) | String
Thread name                 | NameThread             | thread_id: u32     | String
Resource info               | ResourceCreate         | handle: u64        | ResourceInfo
                            | ResourceDestroy        | handle: u64        | (remove)
```

`DomainDestroy` marks a domain as ended; the domain entity's span ends there.

For push/pop ranges: the analyzer must maintain a per-(thread, domain) stack of open pushes so
each `RangePop` closes the most recent `RangePush` on that thread+domain. This mirrors the
injection layer's `RANGE_DEPTH` thread-local, but in the analyzer it is a replay over timestamped
events, not a live thread-local.

For start/end ranges: a `HashMap<u64, OpenStartRange>` keyed by `range_id`. A `RangeEnd` pops
from it; if no matching start is found, log and skip (tolerant).

### `NvtxEventEntity` — `integrations/nvtx/bridge/src/lib.rs`

```rust
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NvtxEventEntity(pub NvtxEvent);

impl EntityEvent for NvtxEventEntity {
    const NAME: &'static str = "NvtxEvent";
}
```

The entire NVTX stream is a single entity stream named `"NvtxEvent"`. The NVTX analyzer's
`UiAnalyzer::Event` = `NvtxEventEntity`.

### Capture wiring — `integrations/nvtx/example/src/lib.rs`

```rust
pub fn run_capture(session: Uuid, exporter: EventCallback) -> Result<(), Box<dyn Error>> {
    let ctx = Context::try_new(session)?;
    let observer = ctx.block_on(async { ctx.observer::<NvtxEventEntity>(exporter).await })?;
    let sender = observer.sender();
    nvtx_injection::install_hook(move |event| sender.emit(session, event))?;
    annotated_work();
    drop(observer);   // drains + flushes
    Ok(())
}
```

The hook receives `NvtxEvent` (from the injection layer) and emits it as `NvtxEventEntity` via
`sender.emit()`. The `sender.emit()` accepts `NvtxEvent` because `NvtxEventEntity: From<NvtxEvent>`.
This pattern is the Phase 2 starting point; the NVTX analyzer consumes the resulting
`Event<NvtxEventEntity>` stream.

---

## 5. Key Questions — Phase 2 Answers

### What does `RtFsmBuilder::try_build` require to succeed?

1. At least one transition pushed (else panics at `transitions.last().unwrap()` — line 122 of
   `crates/analyzer/src/fsm/runtime.rs`).
2. Last transition named `"exit"` (else `AnalyzerError::Validation`).
3. `type_name` set via `set_type_name(...)` (else `AnalyzerError::IncompleteEntity`).
4. `instance_name` set via `set_instance_name(...)` (else `AnalyzerError::IncompleteEntity`).

For NVTX push ranges: an unclosed range never gets its "exit" transition, so `try_build` fails
with `Validation`, not a panic (assuming at least the push was recorded). The zero-transition
panic is the real risk: if an FSM entry is created in `RtFsmsBuilder` but no transitions are
ever pushed (e.g., a builder pre-populated by ID but the events were dropped), `try_build` panics.

### What exactly fails or panics with malformed NVTX input?

| Condition | Outcome | Location |
|-----------|---------|----------|
| `RangePop` with no matching `RangePush` | Silent drop (no builder exists) | Analyzer logic |
| `RangePush` with no `RangePop` (unclosed) | `AnalyzerError::Validation` via `try_build` | `crates/analyzer/src/fsm/runtime.rs:123` |
| FSM builder created, zero transitions | **panic** (`unwrap()`) | `crates/analyzer/src/fsm/runtime.rs:122` |
| Two events at identical timestamps | Zero-duration span; `SpanUnixNanoSec::try_new(t, t)` may error | `crates/analyzer/src/fsm/runtime.rs:170` |
| `RtFsmsBuilder::try_build` — first incomplete FSM | Propagates error, aborts all FSM builds | `crates/analyzer/src/fsm/runtime.rs:313` |

### How does the query engine analyzer consume `RtFsmBuilder` and handle `try_build` errors?

The query engine model does **not** use `RtFsmBuilder` or `RtFsmsBuilder` at all. Its FSM
entities use `FsmEventsBuilder<T>` (proc-macro-generated), which always succeeds at build time
and defers incompleteness to `span()`. `RtFsmBuilder` is only used in tests and potentially
future resource-runtime consumers.

The query engine's `InMemoryQueryEngineModelBuilder::try_build()` calls
`Query::from_builder(v).map(|v| (k, v))` inside a `collect::<AnalyzerResult<_>>()`. The first
`Query` that fails propagates the error. The query engine never encounters incomplete FSMs in
practice because the simulator always emits balanced transitions.

### What is `TimeOrderedCollector` and how does it sort/handle out-of-order events?

```
crates/time/src/lib.rs:126
```

- Backed by a `Vec<T>` always maintained in non-decreasing timestamp order.
- In-order push: O(1) append.
- Out-of-order push: O(log n) binary search + O(n) shift insert.
- Duplicate timestamps: stable insertion (new item placed at the first position where no
  strictly-earlier item exists, i.e., before any same-timestamp item in the fast path, or
  appended after same-timestamp items via the fast path when arriving in order).
- `into_inner()` returns the sorted `Vec` — consumers get a fully sorted sequence regardless
  of arrival order.

For NVTX: system clock resolution can produce bursts of events at the same nanosecond (common
with high-frequency NVTX calls). `TimeOrderedCollector` handles this without panic.

### What would need to change in the analyzer framework to tolerate incomplete FSM lifecycles?

Two changes, one mandatory and one structural:

**Change 1 — Fix the panic in `RtFsmBuilder::try_build`** (`crates/analyzer/src/fsm/runtime.rs:122`):
```rust
// Current (panics):
let last_name = &transitions.last().unwrap().name;

// Required fix — return IncompleteFsm for zero-transition FSMs:
let Some(last) = transitions.last() else {
    return Err(AnalyzerError::IncompleteFsm(
        format!("fsm {} has no transitions", self.id)
    ));
};
let last_name = &last.name;
```

**Change 2 — Partition incomplete FSMs in `RtFsmsBuilder::try_build`** (`crates/analyzer/src/fsm/runtime.rs:310-313`):
```rust
// Current (the TODO):
let fsm = fsm.try_build()?;   // first error aborts all FSMs

// Required — separate good and incomplete:
match fsm.try_build() {
    Ok(fsm) => { fsms.insert(k, fsm); }
    Err(e) => {
        tracing::warn!("dropping incomplete NVTX range {k}: {e}");
        // or push to an incomplete_fsms: Vec<(Uuid, AnalyzerError)> bucket
    }
}
```

Alternatively, the NVTX analyzer can **synthesize a closing "exit" transition** at analysis time
for any unclosed push range before calling `try_build`, using the last observed event's timestamp
as the synthetic end. This avoids touching the framework and keeps the error path clean.

### How does `NvtxEvent` carry handle references that need resolution?

Handles appear in these positions within `NvtxEvent` variants:

| Field | Type | Resolved from |
|-------|------|---------------|
| `domain` (all range/mark/registration events) | `u64` | `DomainCreate { domain, name }` |
| `attributes.message = Some(RegisteredHandle(h))` | `u64` | `RegisterString { domain, handle, string }` |
| `attributes.category` | `u32` (domain-namespaced) | `NameCategory { domain, category, name }` |
| `RangeStart::range_id` / `RangeEnd::range_id` | `u64` | correlates start↔end pair |
| `RegisterString::handle` | `u64` | itself (is the key) |
| `ResourceCreate::handle` | `u64` | itself (is the key); message may contain `RegisteredHandle` |
| `NameThread::thread_id` | `u32` | itself (thread id from the OS) |

**Resolution ordering constraint:** Registration events (`DomainCreate`, `RegisterString`,
`NameCategory`, `NameThread`) typically arrive before the events that reference their handles.
The NVTX spec does not strictly guarantee this, so the analyzer must tolerate forward references:
either do a two-pass scan (first pass builds all tables, second pass reconstructs entities), or
use an `Option<String>` name field and fill it post-hoc.

Two-pass is simpler and safer for Phase 2.

---

## 6. Directory Placement for Phase 2 Artifacts

Following the established repo layering:

```
integrations/nvtx/
├── events/           # Phase 1 — NvtxEvent vocabulary (no changes needed)
├── injection/        # Phase 1 — cdylib + static-injection
├── bridge/           # Phase 1 — NvtxEventEntity newtype
├── example/          # Phase 1 — capture demo + integration test
└── (no new crates in integrations/ for Phase 2)

domains/nvtx/         # NEW for Phase 2
├── model/            # Cargo.toml + src/lib.rs  (model! registration only)
│   └── src/
│       └── lib.rs    # model! macro call
├── analyzer/         # Cargo.toml + src/
│   └── src/
│       ├── lib.rs          # NvtxModel trait
│       ├── model.rs        # InMemoryNvtxModel + Builder
│       ├── domain.rs       # Domain entity
│       ├── thread.rs       # Thread entity
│       ├── push_range.rs   # PushRange FSM entity (or custom type)
│       ├── start_range.rs  # StartRange entity
│       ├── mark.rs         # Mark entity (instantaneous)
│       ├── ui.rs           # UiAnalyzer impl + QuentViewer + Viewer type
│       └── view.rs         # InMemoryNvtxModelView (optional)
```

Cargo.toml for `domains/nvtx/analyzer/` should depend on:
- `nvtx-events` (the event vocabulary)
- `nvtx-bridge` (the `NvtxEventEntity` type)
- `quent-analyzer` (framework traits)
- `quent-events`, `quent-instrumentation`, `quent-time`, `quent-model`
- `uuid`, `rustc-hash`, `tracing`, `thiserror`, `serde`

Add both new crates to root `Cargo.toml` `[workspace] members`.

---

## 7. Tolerant Analyzer Design Principles

The Phase 2 brief specifies tolerance for three conditions without panicking:

**Unclosed ranges:**
- Strategy: in `InMemoryNvtxModelBuilder::try_build()`, after consuming all events, scan
  `open_push_ranges` and `open_start_ranges` for any non-closed entries. Synthesize a terminal
  transition at `last_seen_timestamp + 1` (or use `last_seen_timestamp`) named `"exit"` before
  calling the FSM builder. Log at `warn!` level.
- Do NOT call `RtFsmBuilder::try_build()` on FSMs with zero transitions. If a builder exists
  with no transitions, drop it with a warning (or fix the framework panic first).

**Out-of-order events:**
- `TimeOrderedCollector` handles this transparently — no special handling needed in the NVTX
  analyzer. All transition pushes go through `RtFsmBuilder::push()` which calls
  `TimeOrderedCollector::push()`.

**Duplicate timestamps:**
- `TimeOrderedCollector` handles these without panic. A zero-duration span produced by two
  consecutive equal timestamps may cause `SpanUnixNanoSec::try_new(t, t)` to fail; verify
  whether the span type allows zero duration. If not, offset duplicates by +1ns on insertion.

---

*NVTX Phase 2 map: 2026-07-22*
