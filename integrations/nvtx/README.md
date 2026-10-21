<!--
SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
SPDX-License-Identifier: Apache-2.0
-->

# NVTX integration

Captures NVTX events (ranges, marks, domains, registered strings, categories,
thread names, resources, and the core payload union) from an application and
turns them into Quent events.

The application owns its Quent context and exporter, emits NVTX annotations,
and links a small shim so NVTX initializes capture **in-process**. Runtime
loading through `NVTX_INJECTION64_PATH` is not supported.

**In-process capture requires 64-bit Linux.** `nvtx-injection` enforces this at
compile time. The event types, analyzer, and UI build without the
injection crate. The bridge is used only when capturing NVTX events.

## Contents

- [Crates](#crates)
- [How capture works](#how-capture-works)
- [Using it](#using-it)
- [Captured surface](#captured-surface)

## Crates

| Crate                 | Path             | Role                                                                                                                                                                                                                                         |
| --------------------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `nvtx-injection`      | `injection/`     | The **NVTX C ABI layer**, intended for upstreaming. Fills NVTX's callback tables, copies caller-owned values into `Record`, and calls a `Fn(Record)` hook. Supports in-process `static-injection` only.                              |
| `quent-nvtx-sink` | `sink/` | Schema-generated NVTX event types used by the bridge. |
| `quent-nvtx-bridge`   | `bridge/`        | Queues owned `Record` values, converts them to `SinkEvent` on a worker, and forwards them to an observer. Depends on the Linux-only injection crate.                                                                                         |
| `quent-nvtx-example`  | `example/`       | A runnable example that forwards NVTX events to an observer.                                                                                                                                                                                 |
| `quent-nvtx-analyzer` | `analyzer/`      | Reconstructs ranges, names, and resources from recorded NVTX events.                                                                                                                                                                         |
| `quent-nvtx-ui`       | `ui/`            | Defines the NVTX viewport data and presentation contracts.                                                                                                                                                                                   |

The bridge and analyzer share the schema-generated event types in
`quent-nvtx-sink`. The filesystem stream is named `quent::nvtx::Sink`.

NVTX events use the application's generated schema loader. Application analysis
reconstructs an `NvtxModel` for each sink and exposes it through
the simulator analyzer's `nvtx_models` method. Application-owned routes serve
these models using the same analyzer cache as the regular server, without a
separate NVTX importer or cache. The query-engine domain does not depend on NVTX.

## How capture works

1. The application links `nvtx-injection` with its **`static-injection`**
   feature, publishing a _strong_ `InitializeInjectionNvtx2_fnptr` that
   overrides the _weak_ one in NVTX's header-only client. NVTX then initializes
   injection **in-process** at the first NVTX call.
2. Injection claims NVTX's callback tables — our `extern "C"` functions become
   NVTX's implementation of the subscribed calls.
3. Each callback copies caller-owned NVTX data into an owned `Record` and
   calls the installed hook on the emitting thread. Injection returns handles,
   IDs, and nesting levels even before a hook is installed.
4. The bridge hook records the timestamp and, for push/pop calls, the calling
   thread's OS ID. It queues the owned record for a worker to convert and send
   to the observer.
5. The worker owns the observer. Dropping the capture closes its queue, drains
   records accepted before it closed, and releases its observer. Exporter
   flushing occurs when the final observer owner is dropped. The hook and
   callback pointers stay installed for the process lifetime. Later calls are ignored.

## Using it

`Capture::install` accepts a regular `Observer<E>` whose payload is the shared
`quent_nvtx_sink::quent::nvtx::SinkEvent`, plus a type-erased reference to the parent process:

```rust
let capture = Capture::install(
    context.observer::<instrumentation::quent::nvtx::Sink>(),
    process.as_any_entity_ref(),
)?;
```

Installation creates an NVTX instance and emits `Initialized` with the process
reference before captured events. The process reference is not validated at runtime.
Captured events share the new instance UUID and retain their original timestamps.
The capture worker drains accepted records before releasing its observer.
Other observer owners, including handles and generated contexts, must also be
released before exporter flushing completes.

The [runnable example](example/src/main.rs) creates a process handle and obtains
the sink observer from a generated application context. Its schema enables
`integrations.nvtx: true`; Rust generation imports the payload and records from
`quent-nvtx-sink`. The application combined-event enum wraps the shared payload.

```console
cargo run -p quent-nvtx-example
```

Installation is one-shot. The hook remains installed after the capture is
dropped, but events are discarded. An exporter that emits NVTX for every
received NVTX event can generate an unbounded stream of events.

Stop and join NVTX-producing threads before dropping the capture if all events
must be flushed. An in-flight callback may finish after shutdown begins, and
its event may be discarded.

`static-injection` is requested in the manifest:

```toml
nvtx = { version = "2", default-features = false, features = ["std"] }
nvtx-injection = { path = "…/injection", features = ["static-injection"] }
quent-nvtx-bridge = { path = "…/bridge" }
```

Building capture requires `libclang` and a C compiler. Pixi provides both.

The bundled example uses a **callback exporter** to debug-print each captured
event. Run it without a GPU:

```sh
pixi run cargo run -p quent-nvtx-example
```

It prints one line per event — entity `id`, capture `timestamp` (ns), and the
`SinkEvent`:

```text
[019f… @ 1784726504601037528] Mark { domain: 0, attributes: { message: Some(String("startup")), .. } }
```

### Test

The bridge tests check record conversion, capture, thread IDs, and shutdown
while NVTX calls are active:

```sh
pixi run cargo test -p quent-nvtx-bridge
```

Injection tests cover late hook installation, rejected second installation,
and NVTX calls after Rust TLS destruction:

```sh
pixi run cargo test -p nvtx-injection --features static-injection
```

These tests require no GPU.

## Captured surface

Capture includes marks, ranges, domains, registered strings, categories,
resources, and thread names. It handles calls on both the default domain and
custom domains. Wide-character text is supported for default-domain calls, but
not for domain-scoped names.
