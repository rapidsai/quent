# Quent YAML

## At a glance

Describe a worker with 16 threads and a job that requests four of them:

```yaml
quent: "0.1.0"
model: job_workload

entities:
  Worker:
    resource:
      threads:
        kind: occupancy
        known-bounds: true
    events:
      ready:
        attributes:
          name: string
          limits: { sets-resource-bounds: true }

fsms:
  Job:
    states:
      queued:
        initial: true
        attributes:
          name: string
          requested_threads: u64
        to: [running]
      running:
        attributes:
          worker: { uses: Worker }
        to: [completed]
      completed: {}
```

The generated Rust API turns states, attributes, and resource usage into typed
calls:

```rust
use instrumentation::{Context, Job, JobWorkload, Noop, Worker, WorkerBounds, WorkerUsage};

let context = Context::<JobWorkload>::try_new(Noop)?;

let mut worker = context.observer::<Worker>().handle();
worker.ready("worker-1".to_owned(), WorkerBounds { threads: 16 })?;

let _job = context
    .observer::<Job>()
    .handle()
    .queued("compile".to_owned(), 4)
    .running(worker.as_entity_ref_with(WorkerUsage { threads: 4 }))
    .completed();
```

Misspelled states, invalid transitions, and malformed resource payloads become
compile errors. The full [job workload example](examples/job-workload/) is
runnable.

## Validate a model

Run the checker from the repository root:

```console
cargo run -p quent-yaml --bin quent-yaml-check -- path/to/model.yaml
```

The command exits unsuccessfully and prints source diagnostics when the model
is invalid. Pass `--warnings` before the path to also report constraints that
have no registered validator.

## Interactive tutorial

The browser tutorial presents the examples one at a time and includes short
multiple-choice checks. It is published at
<https://rapidsai.github.io/quent/tutorial/>.

Start its local development server from the repository root:

```console
pixi run mdbook serve docs/tutorial
```

Then open <http://localhost:3000>.

## Examples

Each example contains a YAML model and a program using its generated Rust
instrumentation API. The programs use a no-op exporter, so a successful run
has no output.

### Minimal model

Every model declares the Quent version used when writing the source and a model
name. This model defines a `Task` entity with two events. Each event can occur
once for each `Task` instance, but the model does not constrain their order.

- [YAML model](examples/minimal-model/model.yaml)
- [Instrumentation API usage](examples/minimal-model/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin minimal-model
```

### Event data

An event's `attributes` declare the data captured when it occurs. Attributes
have explicit types, which become argument types in the generated
instrumentation API.

- [YAML model](examples/event-data/model.yaml)
- [Instrumentation API usage](examples/event-data/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin event-data
```

### Repeated events

An event declared with `multi: true` can occur repeatedly for one entity
instance. Events are `once` by default and return an error when emitted again
for the same instance.

- [YAML model](examples/repeated-events/model.yaml)
- [Instrumentation API usage](examples/repeated-events/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin repeated-events
```

### Records

A record groups related fields into a reusable type. It becomes a struct in the
generated Rust instrumentation API.

- [YAML model](examples/records/model.yaml)
- [Instrumentation API usage](examples/records/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin records
```

### Entity references

A `ref` field links one entity to another entity of a declared type. It does not
define a hierarchy between them.

- [YAML model](examples/entity-references/model.yaml)
- [Instrumentation API usage](examples/entity-references/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin entity-references
```

### Scoped references

A `scope-ref` field defines a parent relationship. Scoped references are
validated as a hierarchy in addition to being type-checked entity references.

- [YAML model](examples/scoped-references/model.yaml)
- [Instrumentation API usage](examples/scoped-references/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin scoped-references
```

### Finite-state machines

An FSM declares the allowed lifecycle topology. The parser validates its
initial state, reachability, and final paths, and derives event cardinality from
the transitions. The generated instrumentation API emits state-entry events.
Transition order is model metadata and is not enforced at runtime.

- [YAML model](examples/finite-state-machine/model.yaml)
- [Instrumentation API usage](examples/finite-state-machine/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin finite-state-machine
```

### FSM self-loops

A self-loop allows an FSM to enter the same state repeatedly. The generated
event for that state has `multi` cardinality, while states without a cycle have
`once` cardinality.

- [YAML model](examples/fsm-self-loop/model.yaml)
- [Instrumentation API usage](examples/fsm-self-loop/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin fsm-self-loop
```

### Unit resources

`resource: true` declares an indivisible resource. Each `Thread` is scoped under
a `ThreadPool`. The task's scoped reference carries `ThreadUsage`, so the task
is scoped under and claims a specific thread while it is running.

- [YAML model](examples/unit-resource/model.yaml)
- [Instrumentation API usage](examples/unit-resource/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin unit-resource
```

### Resource capacities

A resource can expose a measured capacity instead of being indivisible. An
`occupancy` usage records a quantity held for the duration of an FSM state.

- [YAML model](examples/resource-capacity/model.yaml)
- [Instrumentation API usage](examples/resource-capacity/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin resource-capacity
```

### Bounded resources

`known-bounds: true` gives a capacity an explicit bound. An attribute marked
with `sets-resource-bounds: true` carries the generated bounds record whenever
the bound changes.

- [YAML model](examples/bounded-resource/model.yaml)
- [Instrumentation API usage](examples/bounded-resource/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin bounded-resource
```

### Job workload

This capstone combines an FSM, event attributes, and measured resource usage.
A worker publishes its thread limit. A job records how many threads it requests
and how many it occupies while running.

- [YAML model](examples/job-workload/model.yaml)
- [Instrumentation API usage](examples/job-workload/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin job-workload
```

## Log sinks

A `logs:` entry declares an entity-scoped logging sink. Its ordered levels
become repeatable events with an implicit `message: string` attribute:

```yaml
logs:
  AppLog:
    levels:
      - name: trace
      - name: debug
      - name: info
      - name: warning
      - name: error
```

Common and level-specific attributes use the ordinary field syntax. A sink can
use common attributes to preserve information supplied by a logging facade:

```yaml
logs:
  AppLog:
    doc: Application logging sink.
    attributes:
      target: string
      file: string
      line: u32
      module: string
      thread_name: string
    levels:
      - name: info
        doc: Informational messages.
      - name: error
        attributes:
          error_code: u32
```

Only `message` has built-in meaning. Names such as `target`, `file`, `line`, and
`module` are ordinary attributes. This example requires every call to provide
them; another model may leave them out, rename them, or change their types.

A log declaration may contain ordinary events when another constraint needs
them. These events are not log levels and do not receive an implicit `message`
attribute. For example, a one-time event can attach each log instance to a
reference tree without repeating the parent reference on every message:

```yaml
entities:
  Application:
    events:
      started: {}

logs:
  AppLog:
    events:
      initialized:
        attributes:
          application: { scope-ref: Application }
    levels:
      - name: info
      - name: error
```

- [YAML model](examples/log-sink/model.yaml)
- [Instrumentation API usage](examples/log-sink/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin log-sink
```

## NVTX sinks

Enable NVTX under the top-level `integrations` key. This adds exactly one entity
type, `quent::nvtx::Sink`, and its event records to the schema. At least one
entity in the schema must represent an OS process:

```yaml
quent: alpha
model: nvtx_sink
integrations:
  nvtx: true
entities:
  Process:
    events:
      started:
        attributes:
          identity: { os: process }
```

The NVTX entity has a once-only `initialized` event with a single `process` attribute,
an ordinary type-erased `ref` without data. It associates an NVTX instance with
a process instance without declaring a scope relationship. Its remaining events
are repeatable:
`range_push`, `range_pop`, `range_start`, `range_end`, `mark`, `domain_create`,
`domain_destroy`, `register_string`, `name_category`, `name_thread`,
`resource_create`, `resource_destroy`, `schema_register`, `enum_register`, and
`binary_payload`. Each carries one `data` attribute referencing a canonical
record, such as `quent::nvtx::RangePush`.

The 15 call records and four shared records (`NvtxEventAttributes`, `NvtxColor`,
`NvtxMessage`, and `NvtxPayload`) are generated once under `quent::nvtx`, shared
by all instances. Handles remain raw integers, and domain zero denotes the default
domain. Field names and widths match the existing NVTX event vocabulary.

`NvtxMessage` has optional `string` and `registered_handle` fields. Producers
must populate exactly one. `NvtxPayload` contains `payload_type: i32` and
`value: u64`. Store the tagged union member's bits, using two's complement for
signed integers and `to_bits()` for floating-point values. For 32-bit members,
zero the upper 32 bits. Optional message, color, and payload attributes remain
optional.

Rust generation imports `SinkEvent` and the canonical records from
`quent-nvtx-sink`, using `quent.codegen.rust.type` metadata added by the integration.
Generated Rust crates must depend on `quent-nvtx-sink`. The complete definitions
remain in the schema for validation and other language generators.

The generated NVTX entity and its events cannot be customized. The
`quent.nvtx.v0.1.0` constraint is attached automatically and cannot be written
directly in YAML.

All process entity types share the single `quent::nvtx::Sink` entity type. Each process instance can have its own NVTX
instance, identified by a UUID and associated through `initialized.process`.
The NVTX entity is a separate root in schemas using the reference-tree
constraint; its process reference carries no target-type restriction.

The [NVTX sink example](../../integrations/nvtx/example/) captures calls made through the NVTX
Rust API and prints the events received by the generated context. It requires
Linux 64-bit:

```console
cargo run -p quent-nvtx-example
```

The example installs the bridge with a sink observer and a process reference.

## Type reference

Scalar attributes support `bool`, `string`, `uuid`, `dynamic`, `u8`, `u16`,
`u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `f32`, and `f64`. A declared record
name can also be used as a type.

Composite types use mapping forms:

| YAML                         | Meaning                          |
| ---------------------------- | -------------------------------- |
| `{ list: string }`           | List of strings                  |
| `{ option: u64 }`            | Optional `u64`                   |
| `ref`                        | Reference to any entity          |
| `{ ref: Worker }`            | Reference to a `Worker`          |
| `{ ref: Worker, data: u64 }` | `Worker` reference carrying data |

Composite forms can nest. For example, `{ option: { list: string } }` is an
optional list of strings.

Semantic modules add these type forms:

### Reference tree

| YAML                      | Meaning                       |
| ------------------------- | ----------------------------- |
| `{ scope-ref: Pipeline }` | Tree-forming entity reference |

### Resource

| YAML               | Meaning                  |
| ------------------ | ------------------------ |
| `{ uses: Memory }` | Resource usage reference |

### Operating system

| YAML              | Meaning                           |
| ----------------- | --------------------------------- |
| `{ os: process }` | Operating-system process identity |
| `{ os: thread }`  | Operating-system thread identity  |

## Complete model

The [instrumentation-build model](../instrumentation-build/example/model.yaml)
combines records, references, FSMs, and event attributes in one larger example.
Its [Rust program](../instrumentation-build/example/src/main.rs) configures an
exporter and uses the generated instrumentation API.

## Directed acyclic graphs

A DAG uses entity types for the graph, its vertices, and its directed edges.
Each vertex and edge records its containing DAG through a typed reference. Each
edge also records typed source and target vertex references.

- [YAML model](examples/dag/model.yaml)
- [Instrumentation API usage](examples/dag/src/main.rs)

Run the example from the repository root:

```console
cargo run --manifest-path crates/yaml/examples/Cargo.toml --bin dag
```

An entity's `dag` declaration assigns its graph role:

| YAML | Meaning |
| --- | --- |
| `dag: true` | DAG entity |
| `dag: false` | No DAG role; equivalent to omitting `dag` |
| `dag: vertex` | Vertex entity |
| `dag: edge` | Directed-edge entity |

These declarations are accepted on ordinary entities and FSM entities. On an
FSM, topology fields are state attributes.

Vertex and edge events declare topology through fields whose `dag` mappings
also create targeted entity references:

| YAML | Meaning |
| --- | --- |
| `dag: { in: Plan }` | Reference to the containing `Plan` DAG |
| `dag: { source: Operator }` | Reference to the edge's source `Operator` vertex |
| `dag: { target: Operator }` | Reference to the edge's target `Operator` vertex |

Every vertex and edge must declare exactly one field with `dag: { in: ... }` in
a once-event. A directed edge must declare exactly one `source` and one `target`
relation in that same event. Both endpoint types must be vertices belonging to
the same DAG type as the edge. These are schema-level checks; instance
membership and acyclicity must be validated after emitted events are
reconstructed.

## Compatibility and versioning of application event models as YAML sources

Write the version of Quent used when writing the YAML source in the `quent`
field:

```yaml
quent: "0.1.0"
model: minimal

# ...
```

After upgrading Quent, the parser checks whether it still accepts sources based
on that version. For example, Quent 0.2.0 may still accept the source above
despite breaking changes elsewhere. If it no longer supports sources based on
0.1.0, it reports an unsupported version. The source then needs migration.

The current parser accepts versions in the range `>=0.1, <0.3`. See the
[versioning policy](../../CONTRIBUTING.md#versioning).
