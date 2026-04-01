# Event Transport

## Decision: reuse the application's Context and EventSender

The injection does not create its own `Context`. The application passes
its `EventSender` via `quent_nvtx::install()`, and all NVTX events flow
into the application's existing event stream.

See [installation.md](./installation.md) for the full installation flow.

## Capture path

Each NVTX callback does:

```rust
sender.send(Event {
    id: session_uuid,
    timestamp: TimeUnixNanoSec::now(),
    data: NvtxEvent::Push { thread_id, ... },
})
```

`EventSender::send()` is non-blocking (unbounded mpsc channel).

## Event identity

All NVTX events share the application's entity UUID. They are part of the
same event stream as the application's own Quent events (FSM transitions,
resource events, traces, etc.).

## Lifecycle

The application owns the `Context` and controls its lifetime. When the
`Context` drops:
1. Forwarder drains remaining queued events.
2. Exporter is flushed.

The injection has no lifecycle management of its own. When the
`EventSender` disconnects (because the `Context` was dropped), subsequent
NVTX callbacks become no-ops (send fails silently, error logged once).

## Context internals (for reference)

`Context<T>` from `quent-instrumentation`:

- Creates an unbounded tokio mpsc channel.
- Spawns a forwarder task that receives events and pushes them to the
  exporter.
- Creates a tokio runtime if none exists, reuses the existing one otherwise.
- `EventSender<T>` is `Clone + Send` — safe to call from any thread.
- Noop mode (no exporter) is supported — sends are free.
