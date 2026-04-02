# Open Questions

Unresolved design decisions for the NVTX injection library.

## ~~1. Injection library architecture~~ → RESOLVED

See [injection-architecture.md](./injection-architecture.md).

Rust static library, stateless forwarder. Strong symbol provided by a
compiled C file (`c/symbol.c`) linked with `-force_load`/`--whole-archive`.
No thread-local state, no lookup tables. Opaque handles are boxed integer
IDs. All interpretation happens in the analyzer.

## ~~2. Event transport~~ → RESOLVED

See [event-transport.md](./event-transport.md).

Reuse Quent's `Context<T>` and `EventSender<T>`. All events emitted under
a single session UUID. Exporter configuration mechanism still TBD.

## ~~3. Thread identification~~ → RESOLVED

The injection calls `gettid()` (Linux) and includes the OS thread ID as a
`u64` on every push/pop/mark event. `nvtxNameOsThreadA` is forwarded as a
`NameThread { os_thread_id, name }` event. The analyzer uses these to group
events by thread and assign human-readable names.

## ~~4. Timestamp capture~~ → RESOLVED

Timestamps are captured on the Rust side using `TimeUnixNanoSec::now()`
when constructing the `Event<T>` in each callback. Same clock source as
the rest of Quent. No C clock APIs needed.

## ~~5. NVTX header binding~~ → RESOLVED

Vendor NVTX v3 headers into the repo. Use `bindgen` to generate Rust FFI
types from the vendored headers at build time. Guarantees correctness
against the actual NVTX type definitions.

## ~~6. Crate structure~~ → RESOLVED

Under `integrations/nvtx/`. Separate from core crates and domains.

Sub-crates:
- `quent-nvtx-injection` at `integrations/nvtx/injection/` — stateless
  forwarder, `install_hook()` API
- `quent-nvtx-events` at `integrations/nvtx/events/` — `NvtxEvent`
  enum and per-variant types
- `quent-nvtx` at `integrations/nvtx/instrumentation/` — Quent wrapper,
  `install()` with `EventSender`
- `quent-nvtx-analyzer` at `integrations/nvtx/analyzer/` — `NvtxModelBuilder`
  (reconstructs traces, FSMs, resolves handles)
- `integrations/nvtx/include/` — `quent_nvtx.h` convenience C/C++ header
  (Phase 5)

## ~~7. Marks (instant events)~~ → RESOLVED

Forwarded as raw `Mark` events. The analyzer decides representation —
likely point-in-time markers on the timeline. No need to force into
span or FSM models.

## ~~8. NVTX resource naming~~ → RESOLVED

Forwarded as raw `ResourceCreate` / `ResourceDestroy` events. The
analyzer decides how to represent them (Quent entities, attributes, etc.).

## ~~9. Process lifecycle~~ → RESOLVED

See [installation.md](./installation.md).

Programmatic installation only. The application owns the `Context` and
controls its lifetime. No `atexit` handler needed. When the `Context`
drops, the `EventSender` disconnects and NVTX callbacks become no-ops.
