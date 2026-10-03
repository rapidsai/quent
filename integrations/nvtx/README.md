<!--
SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
SPDX-License-Identifier: Apache-2.0
-->

# NVTX integration

Captures NVTX events (ranges, marks, domains, registered strings, categories,
thread names, resources, and the core payload union) from an application and
turns them into Quent events.

The application owns its Quent `Context` and exporter, annotates its code with
the NVTX Rust API, and links a small shim so NVTX initializes capture
**in-process** — no cdylib, no `NVTX_INJECTION64_PATH`.

**Linux 64-bit only** — capture relies on the ELF weak-symbol mechanism
(`nvtx-injection` enforces this with a `compile_error!`).

## Contents

- [Crates](#crates)
- [How capture works](#how-capture-works)
- [Using it](#using-it)
- [Captured surface](#captured-surface)
- [NVTX injection bindings](#nvtx-injection-bindings)
- [Upstreaming](#upstreaming)

## Crates

| Crate | Path | Role |
|-------|------|------|
| `nvtx-events` | `events/` | The application-agnostic NVTX event **vocabulary** (`NvtxEvent` + attribute/payload types). Pure Rust, upstreamable to the NVTX Rust crates. |
| `nvtx-injection` | `injection/` | The **NVTX C ABI layer**. Fills NVTX's callback tables, copies each call into an owned `RawEvent`, and hands it to a sink-agnostic hook. Attach in-process via the `static-injection` feature, or at runtime as a cdylib via `NVTX_INJECTION64_PATH`. |
| `nvtx-bridge` | `bridge/` | The **bridge**: decodes owned records into `NvtxEventEntity` events on the capture worker and forwards them through Quent's ordinary observer API. |
| `nvtx-example` | `example/` | A runnable, self-verifying example. |

## How capture works

1. The application links `nvtx-injection` with its **`static-injection`**
   feature, publishing a *strong* `InitializeInjectionNvtx2_fnptr` that
   overrides the *weak* one in NVTX's header-only client. NVTX then initializes
   injection **in-process** at the first NVTX call.
2. Injection claims NVTX's callback tables — our `extern "C"` functions become
   NVTX's implementation of the subscribed calls.
3. Each callback copies caller-owned strings and initialized attribute fields
   into a `RawEvent` once a hook is installed. No caller pointers escape.
4. The hook queues owned records with their capture timestamps. An `NvtxCapture`
   worker drains the queue, decodes strings and numeric payloads, and builds
   `NvtxEvent` values to forward through an ordinary observer. NVTX handles and
   range nesting are maintained on the emitting thread.
5. Dropping `NvtxCapture` closes the queue, drains completed sends, joins the
   worker, and releases the observer to flush its exporter. The permanent hook
   owns only a queue sender. Late events are discarded, including calls during
   process cleanup after Rust TLS destruction.

## Using it

Create an observer and transfer it to the capture utility before emitting NVTX:

```rust
let ctx = ContextInner::try_new(session)?;
let observer = ctx.block_on(async {
    ctx.observer::<NvtxEventEntity>(&exporter).await
})?;
let capture = nvtx_bridge::NvtxCapture::new(session, observer)?;

nvtx::mark(c"startup");
let range = nvtx::Range::new(c"phase-1");
drop(range);

// Drain, join, and flush before releasing the context.
drop(capture);
drop(ctx);
```

The queue uses a separate SPSC producer for each emitting thread. Ordinary
sends do not upgrade a weak observer reference or wake the worker. The worker
polls every millisecond when idle. NVTX strings and pointer-backed attributes
must still be copied before the callback returns. Capture timestamps and NVTX
thread IDs are preserved through deferred forwarding.

Stop and join producers before dropping capture to ensure all their events are
flushed. Completed sends are drained, but sends racing with shutdown may be
lost. Late sends can temporarily succeed in the queue without being exported.
The observer API and instrumentation transport are unchanged. Installation is
one-shot even after capture is dropped.

The queue is provided by `quent-channel`. Quent's current entity handles
timestamp at emission, so this utility owns an observer
and entity ID to forward events with their original timestamps.

`static-injection` is requested in the manifest:

```toml
nvtx-injection = { path = "…/injection", features = ["static-injection"] }
nvtx = { version = "2", default-features = false, features = ["std"] }
```

The bundled example uses a **callback exporter** to debug-print each captured
event (a real app would swap in ndjson, the collector, …). Run it — no GPU:

```sh
pixi run cargo run -p nvtx-example
```

It prints one line per event — entity `id`, capture `timestamp` (ns), and the
`NvtxEvent`:

```text
[019f… @ 1784726504601037528] Mark { domain: 0, attributes: { message: Some(String("startup")), .. } }
```

### Test

`example/tests/capture.rs` reuses the same capture routine in-process with a
collecting callback exporter and asserts every core NVTX kind is captured — no
subprocess, no files:

```sh
pixi run cargo test -p nvtx-example
```

The tests also check failed duplicate registration, dropping the observer owner
inside the hook, and concurrent queue shutdown with completed-send flushing.
`example/tests/shutdown.rs` runs capture on a subprocess's main thread, then
emits CORE and CORE2 push/pop calls from an `atexit` handler after Rust TLS
destruction.
It checks stderr as well as the exit status, since contained TLS panics can
still exit successfully. These tests require no GPU.

## Captured surface

Both NVTX ASCII surfaces: **domain-scoped (CORE2)** — mark, range
start/end/push/pop, domain/register-string/name-category/resource — and the
**classic default domain (CORE)** on domain `0`, plus OS thread naming.

Default-domain wide-char (`*W`) variants are copied as owned code units and
decoded to UTF-8 by the bridge worker. Nesting and synthesized IDs are
preserved. Domain-scoped wide-name calls (`DomainCreateW`,
`DomainRegisterStringW`, and `DomainNameCategoryW`) are not yet subscribed.

## NVTX injection bindings

`nvtx-injection` consumes the upstream NVTX injection ABI through
[`nvtx-sys`](https://crates.io/crates/nvtx-sys) with its `tools` feature. That
feature exposes the callback tables, callback ids, injection result codes, and
function signatures needed by a tool such as Quent. `nvtx-sys` generates the
target-specific Rust declarations from the NVTX headers bundled in the crate,
so Quent no longer carries its own wrapper, bindgen allowlist, or generated
bindings file. No external NVTX installation or `CONDA_PREFIX` is required.

Because `nvtx-sys` runs bindgen and compiles its bundled C implementation, a
fresh downstream build does require a discoverable `libclang` and a native C
compiler. Quent's Pixi environment provides those on Linux through `libclang`
and `cxx-compiler`; the aarch64 environment also installs `clang` for its
compiler resource headers. Outside Pixi, install equivalent system packages
and set `LIBCLANG_PATH` only when `clang-sys` cannot discover the library.

## Upstreaming

`nvtx-injection` already sources its ABI definitions from upstream
`nvtx-sys/tools`. Its capture logic and the application-agnostic `nvtx-events`
vocabulary remain candidates for contribution to NVIDIA/NVTX. Parallel effort,
not a blocker.
