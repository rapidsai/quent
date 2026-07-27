# Stack Research

**Domain:** NVTX v3 injection/consumer library in Rust (FFI + fan-out mediator), feeding an existing Rust telemetry pipeline (Quent)
**Researched:** 2026-07-08
**Confidence:** HIGH (crate versions verified against docs.rs/crates.io; NVTX API verified against `NVIDIA/NVTX` `release-v3` source)

## Executive Take

The NVTX consumer side is a **thin FFI problem, not a framework problem**. There is no consumer-side NVTX crate to adopt — the upstream Rust crates (`nvtx` / `nvtx-sys`, `release-v3`) only *produce* events. The consumer surface is: (1) generate Rust bindings for the NVTX C injection headers with **bindgen**, (2) export a single `InitializeInjectionNvtx2` C-ABI entry point, populate the app's NVTX function table with Rust callbacks, and convert callbacks into Quent events, and (3) build a fan-out mediator that `dlopen`s any external injection library (`NVTX_INJECTION64_PATH`) via **libloading** and drives it from per-sink shadow function tables.

The PR #87 toolchain choice (**bindgen + cc + cargo_metadata**) is still correct for 2026 and matches what upstream `nvtx-sys/build.rs` itself uses. The main refinements: **cc/the C shim is optional** (the primary dlopen injection path is pure Rust; the shim is only needed for the statically-linked-injection test scenario), and versions should be bumped to current (bindgen 0.72.1, libloading 0.9.0).

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **bindgen** (build-dep) | `0.72.1` | Generate Rust FFI bindings for NVTX C headers (`nvtxTypes.h` callback tables, `NvtxCallbackModule`/`NVTX_CBID_*`, `nvtxImplCore.h`, `nvToolsExtPayload.h`) | De-facto standard for C→Rust bindings; exactly what upstream `nvtx-sys/build.rs` uses. NVTX headers are macro-heavy and versioned (`NVTX_VERSIONED_IDENTIFIER`) — hand-writing bindings for the payload extension + callback enums is error-prone and would rot against `release-v3`. 0.72.1 (May 2026) is current; PR #87's 0.72 is already on this line. |
| **libloading** | `0.9.0` | `dlopen`/`dlsym` external injection libraries in the fan-out mediator (`NVTX_INJECTION64_PATH` passthrough to nsys/AON) | Safe, cross-platform (`Library`/`Symbol`) wrapper over `dlopen`; lifetime-checked symbols prevent use-after-unload. Standard choice over raw `libc::dlopen`. NVTX itself uses `dlopen(x, RTLD_LAZY)` — libloading mirrors this. 0.9.0 (Jun 2026) is the current major; API (`Library::new`, `lib.get`) is unchanged from 0.8.x. |
| **cargo_metadata** | `0.23.1` | Locate the NVTX C `include/` dir at build time from the `nvtx-sys` git dependency's checked-out source | The headers ship inside the `nvtx-sys` crate source tree (and parent `c/include`). Resolving the dep's manifest path via `cargo metadata` avoids hardcoding a vendored copy and keeps header versions pinned to the git dep — the exact mechanism PR #87 uses. |
| **cc** (build-dep, *conditional*) | `1.2.66` | Compile the minimal C shim that defines the strong `InitializeInjectionNvtx2_fnptr` symbol for the **static-injection** path only | Only required if you support statically-linked injection (the in-repo test app linking Quent directly). The primary runtime path (`NVTX_INJECTION64_PATH` dlopen) needs **no C shim** — see "What NOT to over-build." Keep `cc` behind a feature/test-only build so the default build stays shim-free. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **nvtx-sys** (git dep, `release-v3`) | pin to a commit | Source of truth for NVTX C headers; provides the `-sys` layer whose source tree bindgen points its `-I` include paths at | Always — depend on it as a git dependency (not crates.io; the consumer headers live in the repo). Pin to a specific commit for reproducibility, not the moving branch. |
| **libc** | `0.2` | Fallback for any raw POSIX symbol not covered by libloading (e.g. `dlsym(NULL, ...)` process-wide table lookup NVTX documents for LD_PRELOAD cases) | Only if the mediator needs the null-handle `dlsym` path; libloading covers the common case. |
| **thiserror** | `2` (workspace) | Error types for bindgen/hook install failures and mediator load failures | Reuse Quent's existing `thiserror 2` — no new dep. |
| **once_cell / std::sync::OnceLock** | std | One-shot `install_hook()` guard (NVTX calls the init exactly once per process) | Use `std::sync::OnceLock` (std, edition 2024) — no external crate needed. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| **libclang / clang** | bindgen runtime dependency (parses the C headers) | **Must be added to `pixi.toml`** (`clang`/`libclang`). The existing stack provisions `cxx-compiler` but bindgen needs `libclang` specifically. Set `LIBCLANG_PATH` if pixi's clang isn't auto-discovered. This is the single most common bindgen CI failure. |
| **cargo test** (built-in) | Integration test that loads the injection lib against the in-repo NVTX test app | Matches existing Quent testing; no new framework. The deterministic in-repo NVTX producer app is the CI vehicle (no GPU). |
| **cargo metadata** (CLI, invoked in build.rs via the crate) | Header discovery at build time | Already covered by the `cargo_metadata` crate dependency. |

## Installation

```toml
# integrations/nvtx/quent-nvtx-injection/Cargo.toml

[dependencies]
libloading = "0.9"          # fan-out mediator: dlopen external injection libs
nvtx-sys    = { git = "https://github.com/NVIDIA/NVTX", branch = "release-v3" }  # pin to a commit in practice
thiserror   = { workspace = true }
libc        = "0.2"         # optional: null-handle dlsym path

[build-dependencies]
bindgen        = "0.72"     # 0.72.1 current
cargo_metadata = "0.23"     # locate nvtx-sys headers
cc             = "1.2"      # only compiled for the static-injection feature/test
```

```toml
# pixi.toml additions
[dependencies]
clang = "*"       # provides libclang for bindgen
```

## NVTX API Surface (verified against release-v3 source)

These are the concrete C symbols the bindgen layer must expose (drives crate scoping, not a dependency choice, but load-bearing for the roadmap):

- **Entry point:** `int InitializeInjectionNvtx2(NvtxGetExportTableFunc_t exportTable)` — the one C-ABI symbol Quent's injection lib must export (`#[no_mangle] extern "C"`).
- **Export tables:** `NvtxGetExportTableFunc_t` → `NVTX_ETID_CALLBACKS` yields `NvtxExportTableCallbacks { GetModuleFunctionTable(module, out_table, out_size) }`. This is how the injection populates the app's per-module function-pointer table.
- **Callback modules:** `NVTX_CB_MODULE_CORE` (1), `CORE2` (5), plus `CUDA`/`OPENCL`/`CUDART`/`SYNC`. CORE + CORE2 are the mandatory surface (marks, push/pop, range start/end, domains, registered strings, categories, resources, thread naming). CBIDs enumerated in `nvtxTypes.h` (`NVTX_CBID_CORE_*`, `NVTX_CBID_CORE2_*`).
- **Payload extension:** `nvToolsExtPayload.h` — schema registration (`nvtxPayloadSchemaRegister`/`nvtxPayloadSchemaAttr_t`), enum registration (`nvtxPayloadEnumRegister`), binary payloads (`nvtxPayloadData_t{ schemaId, size, payload* }`). Delivered through a **separate extension module** (`nvtxExtInit`/`NvtxExtModuleInfo`), not the CORE callback table — this is materially more complex and has **no existing Rust tooling**. `nvtxExtPayloadTypeInfo.h` gives field type/offset layout for parsing blobs against a registered schema.

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| bindgen 0.72 | Hand-written FFI bindings | Never for the full surface. A tiny hand-written binding for *only* `NvtxGetExportTableFunc_t` + `NvtxExportTableCallbacks` could avoid bindgen entirely for a minimal MVP — but the payload extension and CBID enums make hand-maintenance untenable. |
| bindgen 0.72 | `bindgen-cli` + checked-in `bindings.rs` | If you want to drop `libclang` from CI: run bindgen once, commit the generated file, gate regeneration behind a feature. Reduces build-time deps at the cost of manual regen when NVTX headers change. Reasonable for a "separable upstream crate." |
| libloading 0.9 | raw `libc::dlopen`/`dlsym` | Only if you need behavior libloading doesn't expose (e.g. specific `RTLD_*` flag combos beyond `os::unix::Library::open`). libloading's `os::unix` submodule already exposes flags, so rarely needed. |
| cargo_metadata header discovery | Vendored/`git submodule` NVTX headers | If you decide to vendor a frozen copy of `c/include` into the repo for hermetic builds. Trades reproducibility-via-pin for a manual update burden; acceptable if the git dep proves flaky in CI. |
| cc C shim (static path) | Pure-Rust strong symbol | **Prefer pure Rust when possible.** Rust symbols are strong by default, so `#[no_mangle] pub static InitializeInjectionNvtx2_fnptr: NvtxInitializeInjectionNvtxFunc_t = Some(initialize_injection);` can override NVTX's `__attribute__((weak))` symbol at link time — potentially eliminating the C shim even for static injection. Validate that the linker resolves the Rust static over the weak C symbol (needs `#[used]`, `-fPIC` parity); fall back to the cc shim only if it doesn't. |

## What NOT to Use / Over-build

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| The upstream `nvtx` / `nvtx-sys` crates as a *consumer* | They only **produce** events (instrument-your-app APIs). No consumer/injection API is exposed. | Use `nvtx-sys` **only** as the header source for your own bindgen build; write the consumer layer yourself. |
| A C shim on the default build path | The primary injection path is dlopen of your lib via `NVTX_INJECTION64_PATH`, which only needs an exported `InitializeInjectionNvtx2` — pure `#[no_mangle] extern "C"` Rust, zero C. Adding `cc`/libclang to the default build for the shim taxes every build. | Gate `cc` + the shim behind a `static-injection` feature used only by the in-repo static test. |
| `autocxx` / `cxx` for these bindings | Those are for C++ interop. NVTX's injection ABI is plain C function-pointer tables; cxx adds a bridge layer that fights the raw-pointer/function-table shape. | Plain `bindgen` + `extern "C"`. (Quent uses `cxx` elsewhere for its own C++ bridge — different problem.) |
| Windows support tooling (MSVC weak-symbol shims, `wchar_t` handling) | Out of scope per PROJECT.md; NVTX weak-symbol injection doesn't work on Windows. | Guard the crate with `#[cfg(unix)]`; compile-error on Windows as PR #87 already does. |
| Pinning the NVTX git dep to a moving `release-v3` branch | Non-reproducible builds; header ABI can shift under you (payload extension is actively evolving). | Pin to a specific commit SHA; bump deliberately. |

## Fan-out Mediator Design (stack-relevant mechanics)

Confirmed against `nvtxInit.h`/`nvtxTypes.h`. The mediator is the sole registered injection; sinks are shadow tables:

1. Quent's lib exports `InitializeInjectionNvtx2`; NVTX calls it with the app's `NvtxGetExportTableFunc_t`.
2. Mediator obtains `NvtxExportTableCallbacks.GetModuleFunctionTable` and, for each module (CORE, CORE2, …), installs **mediator-owned** function pointers into the app's real table.
3. Mediator reads `NVTX_INJECTION64_PATH` itself, **`libloading::Library::new`** on it, `get::<InitializeInjectionNvtx2>()`, and calls it with a **synthetic `getExportTable`** that hands that sink a **per-sink shadow `GetModuleFunctionTable`** writing into a shadow function table the mediator owns.
4. Quent is registered as another sink the same way (its own shadow table).
5. Each mediator callback iterates the sink shadow tables and invokes each sink's populated pointer (skipping unset/no-op slots).

Only new runtime dependency this introduces: **libloading**. Everything else is std + the generated bindings.

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `bindgen 0.72.1` | Rust edition 2024 / toolchain ≥1.93 (Quent's MSRV) | Requires `libclang` ≥ 9 at build time; provision via pixi `clang`. |
| `libloading 0.9.0` | Rust ≥ 1.63 | API-compatible with 0.8.x for `Library`/`Symbol`; safe to standardize on 0.9. |
| `cargo_metadata 0.23.1` | Cargo/workspace resolver "3" (Quent uses it) | Reads the same `cargo metadata` JSON the workspace already produces. |
| `cc 1.2.66` | Any | Only pulled in under the `static-injection` feature; keep it off default builds. |
| `nvtx-sys` (`release-v3`) | NVTX v3 ABI | Pin to a commit. Payload-extension headers (`nvToolsExtPayload*.h`) are present but the ext-module wiring is the hard part, not the binding generation. |

## Stack Patterns by Variant

**If MVP = CORE/CORE2 ranges only (defer payload extension):**
- bindgen + libloading + nvtx-sys headers; no `cc`, no ext-module handling.
- Because the callback-table path covers all of push/pop, range start/end, marks, domains, registered strings, categories, resources — the analytically essential surface — with the least FFI risk.

**If payload extension is in v1 scope (per PROJECT.md "Active"):**
- Same core stack, plus hand-written ext-module glue over `nvtxExtInit`/`NvtxExtModuleInfo` and `nvtxExtPayloadTypeInfo.h`-driven blob parsing. No crate exists for this — budget it as custom, higher-risk work and flag for deeper phase research.

**If the crate must stay upstreamable (separability constraint):**
- Prefer checked-in generated bindings (`bindgen-cli`, committed `bindings.rs`) and the pure-Rust strong-symbol override to minimize the build-time toolchain (`libclang`, `cc`) a downstream NVIDIA consumer would inherit.

## Sources

- `NVIDIA/NVTX` `release-v3` — `c/include/nvtx3/nvtxDetail/{nvtxTypes.h, nvtxInit.h}` (injection ABI, `InitializeInjectionNvtx2`, `NvtxExportTableCallbacks.GetModuleFunctionTable`, `NVTX_CB_MODULE_*`/`NVTX_CBID_*`, weak-symbol/`NVTX_INJECTION64_PATH` load sequence) — **HIGH** (primary source)
- `NVIDIA/NVTX` `release-v3` — `rust/crates/nvtx-sys/build.rs` (confirms upstream uses bindgen+cc, include-path layout) — **HIGH**
- `NVIDIA/NVTX` `release-v3` — `c/include/nvtx3/nvToolsExtPayload.h`, `nvtxDetail/nvtxExtPayloadTypeInfo.h` (payload schema/enum registration, `nvtxPayloadData_t`) — **HIGH**
- docs.rs — bindgen 0.72.1 (2026-05-19), libloading 0.9.0 (2026-06-06), cc 1.2.66 (2026-07-05), cargo_metadata 0.23.1 (2026-06-07) — **HIGH** (current versions verified)
- GitHub `nagisa/rust_libloading` tags — 0.9.0 latest — **HIGH**
- `.planning/PROJECT.md`, `.planning/codebase/STACK.md` — project constraints, PR #87 prior art, existing Quent stack — **HIGH**

---
*Stack research for: NVTX v3 injection/consumer library in Rust*
*Researched: 2026-07-08*
