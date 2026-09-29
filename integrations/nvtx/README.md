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
| `nvtx-injection` | `injection/` | The **NVTX C ABI layer**. Fills NVTX's callback tables, converts each call into a verbatim `NvtxEvent`, and hands it to a sink-agnostic `Fn(NvtxEvent)` hook. Attach in-process via the `static-injection` feature, or at runtime as a cdylib via `NVTX_INJECTION64_PATH`. |
| `nvtx-bridge` | `bridge/` | The **bridge**: `NvtxEventEntity`, a newtype over `NvtxEvent` implementing Quent's `EventPayload`. The orphan rule forces the impl here; the only crate depending on Quent internals. |
| `nvtx-example` | `example/` | A runnable, self-verifying example. |

## How capture works

1. The application links `nvtx-injection` with its **`static-injection`**
   feature, publishing a *strong* `InitializeInjectionNvtx2_fnptr` that
   overrides the *weak* one in NVTX's header-only client. NVTX then initializes
   injection **in-process** at the first NVTX call.
2. Injection claims NVTX's callback tables — our `extern "C"` functions become
   NVTX's implementation of the subscribed calls.
3. Each callback converts the raw NVTX ABI struct into a verbatim `NvtxEvent`
   and dispatches it to the installed hook while capture is active.
4. The application's hook forwards each event (wrapped in `NvtxEventEntity`)
   into its `Observer`. Handles, ids, and nesting levels are synthesized so the
   app still behaves correctly — including before the hook is installed and
   after capture is disabled, so handles an app caches early stay valid.
5. `install_hook` returns a `CaptureGuard`. Dropping it disables capture without
   waiting for hook calls already in progress. Callback pointers stay installed,
   and callbacks tolerate destroyed TLS during late process cleanup.

## Using it

The app owns the pipeline; wiring capture is two steps — build an observer, then
install the hook:

```rust
// 1. The app owns its Quent context and picks the exporter.
let ctx = Context::try_new(session)?;
let options = FileSystemExporterOptions::new(FileSystemFormat::Ndjson, out_dir);
let observer = ctx.block_on(async { ctx.observer::<NvtxEventEntity>(options).await })?;

// 2. Forward captured NVTX events into it, before the first NVTX call.
let sender = observer.sender();
let capture = nvtx_injection::install_hook(move |event| sender.emit(session, event))?;

// 3. Ordinary app code, annotated with NVIDIA's NVTX Rust API.
nvtx::mark(c"startup");
let range = nvtx::Range::new(c"phase-1");
drop(range); // end the range before flushing

// 4. End capture, then flush the observer.
drop(capture);
drop(observer);
```

Capture lasts exactly as long as the guard. Bind it to a named variable:
`let _ = install_hook(..)` drops it immediately. Declare it after the observer,
as above, so that on an early return or panic the guard still drops first
(locals drop in reverse order; struct fields drop in declaration order, so there
the guard field must come first). Installation is one-shot: a failed caller gets
no guard and cannot end the owner's capture, and capture cannot be restarted.

Stop and join NVTX-producing threads before ending capture if all events must
be flushed. Callbacks that already passed the dispatch check can still invoke
the hook after the guard is dropped. Events racing with observer shutdown may
be discarded or log a send error.

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

The tests also check failed duplicate registration and dropping the guard from
inside the hook.
`example/tests/shutdown.rs` runs capture on a subprocess's main thread, then
emits CORE and CORE2 push/pop calls from an `atexit` handler after Rust TLS
destruction.
It checks stderr as well as the exit status, since contained TLS panics can
still exit successfully. These tests require no GPU.

## Captured surface

Both NVTX ASCII surfaces: **domain-scoped (CORE2)** — mark, range
start/end/push/pop, domain/register-string/name-category/resource — and the
**classic default domain (CORE)** on domain `0`, plus OS thread naming.

Default-domain wide-char (`*W`) variants are converted to owned UTF-8 strings
and captured while preserving nesting and synthesized IDs. Domain-scoped
wide-name calls (`DomainCreateW`, `DomainRegisterStringW`, and
`DomainNameCategoryW`) are not yet subscribed.

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
