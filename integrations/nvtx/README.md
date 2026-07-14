<!--
SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
SPDX-License-Identifier: Apache-2.0
-->

# Quent NVTX integration

Captures NVTX events (ranges, marks, domains, registered strings, categories,
thread names, resources, and the CORE payload union) from an instrumented
application and turns them into Quent events. Capture is enabled with **no
application code changes** — NVTX loads the capture library at runtime via the
`NVTX_INJECTION64_PATH` environment variable.

**Linux 64-bit only** (D-04): injection relies on the ELF weak-symbol /
`NVTX_INJECTION64_PATH` mechanism, which excludes Windows and 32-bit targets.
`quent-nvtx-injection` enforces this with a `compile_error!`.

## Crate layout

| Crate | Path | Role |
|-------|------|------|
| `quent-nvtx-events` | `events/` | Verbatim, Quent-agnostic NVTX event vocabulary (`NvtxEvent` + attributes/payload). Depends on nothing Quent-internal so it can be offered upstream to NVIDIA/NVTX later (D-03). |
| `quent-nvtx-injection` | `injection/` | The Quent-agnostic injection cdylib: exports `InitializeInjectionNvtx2`, fills the CORE/CORE2 callback tables, converts calls to verbatim `NvtxEvent`s, and hands them to a sink-agnostic `Fn(NvtxEvent)` hook. |
| `quent-nvtx` | `instrumentation/` | The bridge into Quent's event pipeline plus the self-configuring capture cdylib that NVTX loads. Bounded ring + drop-and-count in front of Quent's `EventSender` (CAP-05). |

## Attaching the capture library

The **primary** attach path is the runtime injection env var (D-15). Point it at
the built `quent-nvtx` capture cdylib and set an output directory before the
process makes its first NVTX call:

```sh
NVTX_INJECTION64_PATH=/path/to/libquent_nvtx.so \
QUENT_NVTX_OUTPUT_DIR=/path/to/output \
  ./your-instrumented-app
```

A **secondary** link-time path exists behind the `static-injection` feature of
`quent-nvtx-injection` (a strong-symbol C shim); the runtime cdylib path is the
one used everywhere else.

## Bindings: committed bindgen output (D-14)

`injection/src/bindings.rs` is a **committed** `bindgen` artifact for the NVTX
injection ABI. Because it is checked in:

- **Default and CI builds need no libclang** and fetch **no** NVTX git source —
  `build.rs` simply `include!`s the committed file.
- The pinned NVIDIA/NVTX git dependency in `injection/Cargo.toml` is `optional`
  and only pulled in by the `regenerate-bindings` feature.

A small amount of additional NVTX ABI surface that the committed bindings'
allowlist intentionally omits (the range-id and resource-attribute types) is
declared by hand in `injection/src/convert.rs` (module `abi`). Keep it in sync
with the pinned NVTX headers when bumping the rev (see below).

### Regenerating the bindings

Run this **only** when the pinned NVIDIA/NVTX `rev` in `injection/Cargo.toml` is
bumped (D-13). It pulls in `bindgen` + libclang and the NVTX git checkout,
regenerates `injection/src/bindings.rs`, and must not run on normal or CI
builds:

```sh
pixi run cargo build -p quent-nvtx-injection --features regenerate-bindings
```

The headers are located from the pinned `nvidia-nvtx` git checkout
(`<checkout>/c/include`) via `cargo metadata`, or from an `NVTX_INCLUDE_DIR`
override for offline/hermetic regeneration:

```sh
NVTX_INCLUDE_DIR=/path/to/NVTX/c/include \
  pixi run cargo build -p quent-nvtx-injection --features regenerate-bindings
```

After regenerating, **commit the updated `bindings.rs`** (and any hand-written
`abi` additions it supersedes). On an unchanged `rev` the regeneration is a
byte-identical no-op.

## Running the end-to-end capture test (no GPU)

The capture cdylib (`libquent_nvtx.so`) must exist next to the deterministic
test-app binary before the subprocess harness runs, so build it first:

```sh
pixi run cargo build -p quent-nvtx --features e2e
pixi run cargo test  -p quent-nvtx --features e2e --test capture_e2e
```

The test spawns a Quent-free NVTX emitter as a subprocess under
`NVTX_INJECTION64_PATH`, then reads back the captured ndjson and asserts every
core NVTX kind, the CORE payload union, and cross-thread range pairing were
captured — all without any GPU.
