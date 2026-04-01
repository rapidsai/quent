# Installation: Programmatic Injection

## Approach

The application links against the `quent-nvtx` crate and programmatically
installs the injection by passing its own `EventSender`.

```rust
let context = Context::try_new(exporter, engine_id)?;
quent_nvtx::install(context.events_sender());
// all subsequent NVTX calls are captured into the same event stream
```

No environment variables. No standalone `Context` creation inside the
injection. The injection is always part of a Quent-instrumented application.

## Mechanism

### Static linking

The `quent-nvtx` crate provides the weak symbol
`InitializeInjectionNvtx2_fnptr` as a non-weak (strong) symbol. When the
application links against the crate, the linker resolves NVTX's weak
reference to it. No `dlopen` / `NVTX_INJECTION64_PATH` involved.

### Initialization sequence

1. Application creates its `Context<T>` with the desired exporter.
2. Application calls `quent_nvtx::install(sender)`.
   - Stores the `EventSender` in a global `static`.
3. First NVTX API call by the application (or any linked library) triggers
   `nvtxInitOnce()` → calls `InitializeInjectionNvtx2`.
4. `InitializeInjectionNvtx2` registers callbacks for CORE, CORE2, and
   payload extension modules.
5. All callbacks read the `EventSender` from the global `static` and use
   it to emit events.

### Ordering

`install()` must be called before the first NVTX API call. If NVTX
initializes before `install()` is called, the `EventSender` is not yet
available and events are lost (or the callbacks are no-ops).

In practice this is straightforward — the application sets up its
`Context` early in `main()` and calls `install()` before starting work.

## Process lifecycle

Since the `Context` is owned by the application (not the injection),
the application controls its lifetime. When the `Context` drops, the
`EventSender` becomes disconnected — subsequent NVTX calls are effectively
no-ops (send fails silently, error logged once).

No `atexit` handler needed. The application is responsible for keeping
the `Context` alive for as long as NVTX capture is desired.

## Event identity

All NVTX events are emitted with the same `id: Uuid` as the application's
`Context`. They share the same event stream as the application's own
Quent events (FSM transitions, resource events, etc.).

## What the crate provides

- `quent_nvtx::install(sender: EventSender<T>)` — stores the sender and
  ensures the injection symbol is linked.
- The `InitializeInjectionNvtx2` symbol (strong, replaces NVTX's weak
  reference).
- Optionally, `InitializeInjectionNvtxExtension` for payload extension
  support.
- The `quent_nvtx.h` convenience header for C/C++ applications that want
  to attach Quent payloads to NVTX ranges.
