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
compile time. The event types, analyzer, server, and UI build without the
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
| `quent-nvtx-events`   | `events/`        | The NVTX event types used by the Quent observer and analyzer.                                                                                                                                                                                 |
| `quent-nvtx-bridge`   | `bridge/`        | Queues owned `Record` values, converts them to `NvtxEvent` on a worker, and forwards them to an observer. Depends on the Linux-only injection crate.                                                                                         |
| `quent-nvtx-example`  | `example/`       | A runnable example that forwards NVTX events to an observer.                                                                                                                                                                                 |
| `quent-nvtx-analyzer` | `analyzer/`      | Reconstructs ranges, names, and resources from recorded NVTX events.                                                                                                                                                                         |
| `quent-nvtx-ui`       | `ui/`            | Defines the NVTX viewport data and presentation contracts.                                                                                                                                                                                   |
| `nvtx-server`         | `server/`        | Loads NVTX streams and serves the analyzer and UI data through HTTP routes.                                                                                                                                                                  |

The server reads recorded `NvtxEvent` values directly. Recording applications
use the bridge to convert injection `Record`s.

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
   records accepted before it closed, then flushes the observer. The hook and
   callback pointers stay installed for the process lifetime. Later calls are
   ignored.

## Using it

Create an observer, then pass it to `Capture::install` before the NVTX events to
capture. See the [runnable example](example/src/main.rs) for the complete setup.

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
`NvtxEvent`:

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
