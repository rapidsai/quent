<!--
SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
SPDX-License-Identifier: Apache-2.0
-->

# NVTX integration

Captures NVTX events (ranges, marks, domains, registered strings, categories,
thread names, resources, and the core payload union) through the same generated
instrumentation, exporters, collector, store, and analyzer lifecycle as an
application's other Quent events.

An application selects an OS-process entity with `nvtx: true`. Its generated
process identity method activates a private NVTX stream; Rust, C++, and Python
callers do not construct an NVTX observer or sender. The application keeps its
ordinary Quent `Context` and annotates work with an NVTX client API.

Live capture is **Linux 64-bit only** because it relies on the ELF weak-symbol
mechanism. Schema generation, stored-event loading, and analysis remain
portable when live capture is not requested.

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
| `nvtx-schema` | `schema/` | Canonical schema composition and validation for `nvtx: true`, including the typed process binding and lossless NVTX records. |
| `nvtx-injection` | `injection/` | The **NVTX C ABI layer**. Fills NVTX's callback tables, converts each call into a verbatim `NvtxEvent`, and hands it to a sink-agnostic `Fn(NvtxEvent)` hook. Attach in-process via the `static-injection` feature, or at runtime as a cdylib via `NVTX_INJECTION64_PATH`. |
| `nvtx-analyzer` | `analyzer/` | Borrowed field-access traits and one reconstruction implementation for both native and independently generated event types. It reconstructs each `(context, process, stream)` source independently. |
| `nvtx-ui` / `nvtx-server` | `ui/`, `server/` | Reusable presentation and analyzer-backed HTTP routes. The server borrows the application's shared analyzer cache. |
| `nvtx-example` | `example/` | A generated model exercising capture, shared export/import, and reconstruction. |

## How capture works

1. YAML or typed schema composition expands `nvtx: true` into a canonical,
   private event entity bound to the selected OS-process entity.
2. Instrumentation generation emits conversion and analyzer traits for that
   event type. With `Options::nvtx_capture`, it also attaches activation to the
   process identity event.
3. The application links `nvtx-injection` with its **`static-injection`**
   feature. Its strong `InitializeInjectionNvtx2_fnptr` overrides the weak one
   in NVTX's header-only client, so NVTX initializes capture in-process on the
   first NVTX call.
4. The generated activation adapter validates the local PID, queues process
   identity, installs a gated hook, queues the private process binding, and
   forwards captured events through the model's existing exporter pipeline.
5. `quent-store` loads the complete context once. The application analyzer
   dispatches generated NVTX events to `nvtx-analyzer`, retaining context,
   process, and stream identities for application-specific correlation.

## Using it

Select the process entity in the model:

```yaml
entities:
  RuntimeProcess:
    nvtx: true
    events:
      started:
        attributes:
          process: { os: process }
```

Enable live capture when generating instrumentation and add the target-specific
injection dependency:

```rust
quent_instrumentation_build::generate(
    &schema,
    &quent_instrumentation_build::Options {
        nvtx_capture: true,
        ..Default::default()
    },
)?;
```

```toml
[target.'cfg(all(target_os = "linux", target_pointer_width = "64"))'.dependencies]
nvtx-injection = { path = "…/injection", features = ["static-injection"] }
nvtx = { version = "2", default-features = false, features = ["std"] }
```

The ordinary process event activates capture. The generated NVTX stream stays
private:

```rust
let context = generated::Context::try_new(exporter)?;
let mut process = context.observer::<generated::RuntimeProcess>().handle();
process.started(generated::quent::os::Process {
    native_id: std::process::id(),
})?;

let range = nvtx::LocalRange::new(c"phase-1");
drop(range);
drop(process);
drop(context); // drains and flushes every generated stream
```

Active producer contexts enable configured sources by default. Tests,
synthetic producers, and collector/replay contexts can disable activation while
continuing to export ordinary events:

```rust
let options = generated::ContextOptions::default()
    .with_source_capture(generated::SourceCapture::Disabled);
let context = generated::Context::try_new_with_options(exporter, options)?;
```

Generated C++ and Python bindings expose the same control only for models that
contain the validated NVTX source:

```cpp
auto context = quent::Context::ndjson(
    output_dir, quent::SourceCapture::Disabled);
```

```python
context = quent.Context(
    quent.ExporterOptions.ndjson(output_dir),
    source_capture=quent.SourceCapture.Disabled,
)
```

No-op contexts and generated collector forwarding never activate the source.
Until the multi-context router tracked by issue #696 lands, the production
backend permits one hook installation per process. A later activation returns
`HandleError::SourceActivation`; it does not export a phantom private binding.

The bundled example uses a generated process model and a callback exporter to
debug-print captured schema events. Run it without a GPU:

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

The example's roundtrip sends a real capture through Quent's NDJSON
filesystem exporter, loads it through `quent-store`, and then runs the shared
NVTX reconstruction:

```sh
pixi run cargo test -p nvtx-example --test roundtrip
```

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
