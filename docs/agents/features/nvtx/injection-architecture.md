# Injection Library Architecture

## Decision: stateless forwarder

The injection library (`quent-nvtx-injection`) is a **pure forwarder**.
Every NVTX API call is serialized into a raw event with the original
arguments and dispatched through a user-provided hook. The injection
performs no interpretation, no state tracking, and no mapping to Quent
modeling concepts.

The Quent-specific wrapper (`quent-nvtx` at
`integrations/nvtx/instrumentation/`) connects this hook to Quent's
`EventSender`.

All structuring — building span trees from push/pop sequences, resolving
domain/string/category handles, matching start/end range pairs — happens
in the **analyzer**, not the injection.

## Implementation

**Language**: Rust.

**Crate type**: Regular library crate (not a `cdylib`). The application
links against it statically. The crate provides the strong symbol for
`InitializeInjectionNvtx2_fnptr`, replacing NVTX's weak reference at
link time.

**Crate**: `quent-nvtx-injection` at `integrations/nvtx/injection/`.

**Installation**: Programmatic only. See [installation.md](./installation.md).

## State

The injection is **stateless** — it holds no lookup tables, no per-thread
stacks, no domain registries.

The only global state is the hook function stored by
`quent_nvtx_injection::install_hook()`. The Quent wrapper
`quent_nvtx::install()` calls this with a closure that forwards events
through an `EventSender`.

### Opaque handle allocation

NVTX requires the injection to return valid pointers for opaque handle
types (`nvtxDomainHandle_t`, `nvtxStringHandle_t`, `nvtxResourceHandle_t`).

Strategy: allocate a `Box<u64>` containing a monotonically incrementing
ID (from a global `AtomicU64`). Return the raw pointer as the handle.
The handle value (the integer ID) is included in the serialized event.

On `DomainDestroy` / `ResourceDestroy`, the box is reclaimed. No other
cleanup needed.

### What the injection does NOT track

- Push/pop stacks (per-thread or otherwise)
- Domain-to-name mappings
- Registered string values
- Category name tables
- Parent-child span relationships
- Range ID to entity mappings

All of these are reconstructed by the analyzer from the event stream.

## Raw event types

Every NVTX API call becomes one event. The injection captures:

- **Timestamp**: `TimeUnixNanoSec::now()` on the Rust side
- **Thread ID**: `gettid()` on the Rust side
- **The NVTX call's arguments** (verbatim)

### Core module events

| NVTX call | Event | Fields |
|---|---|---|
| `RangePushA(msg)` | `Push` | `thread_id`, `message: String` |
| `RangePushW(msg)` | `Push` | `thread_id`, `message: String` (converted from wchar) |
| `RangePushEx(attr)` | `Push` | `thread_id`, `attributes` (full) |
| `RangePop()` | `Pop` | `thread_id` |
| `RangeStartA(msg)` | `RangeStart` | `range_handle_id`, `message: String` |
| `RangeStartW(msg)` | `RangeStart` | `range_handle_id`, `message: String` |
| `RangeStartEx(attr)` | `RangeStart` | `range_handle_id`, `attributes` (full) |
| `RangeEnd(id)` | `RangeEnd` | `range_handle_id` |
| `MarkA(msg)` | `Mark` | `thread_id`, `message: String` |
| `MarkW(msg)` | `Mark` | `thread_id`, `message: String` |
| `MarkEx(attr)` | `Mark` | `thread_id`, `attributes` (full) |
| `NameCategoryA(id, name)` | `NameCategory` | `category_id`, `name: String` |
| `NameOsThreadA(tid, name)` | `NameThread` | `os_thread_id`, `name: String` |

### Core2 (domain-aware) module events

| NVTX call | Event | Fields |
|---|---|---|
| `DomainCreateA(name)` | `DomainCreate` | `domain_handle_id`, `name: String` |
| `DomainCreateW(name)` | `DomainCreate` | `domain_handle_id`, `name: String` |
| `DomainDestroy(handle)` | `DomainDestroy` | `domain_handle_id` |
| `DomainRangePushEx(d, attr)` | `Push` | `thread_id`, `domain_handle_id`, `attributes` |
| `DomainRangePop(d)` | `Pop` | `thread_id`, `domain_handle_id` |
| `DomainRangeStartEx(d, attr)` | `RangeStart` | `range_handle_id`, `domain_handle_id`, `attributes` |
| `DomainRangeEnd(d, id)` | `RangeEnd` | `range_handle_id`, `domain_handle_id` |
| `DomainMarkEx(d, attr)` | `Mark` | `thread_id`, `domain_handle_id`, `attributes` |
| `DomainRegisterStringA(d, s)` | `RegisterString` | `domain_handle_id`, `string_handle_id`, `value: String` |
| `DomainRegisterStringW(d, s)` | `RegisterString` | `domain_handle_id`, `string_handle_id`, `value: String` |
| `DomainNameCategoryA(d, id, n)` | `NameCategory` | `domain_handle_id`, `category_id`, `name: String` |
| `DomainResourceCreate(d, attr)` | `ResourceCreate` | `domain_handle_id`, `resource_handle_id`, `attributes` |
| `DomainResourceDestroy(handle)` | `ResourceDestroy` | `resource_handle_id` |

### Payload extension events

| NVTX call | Event | Fields |
|---|---|---|
| `nvtxPayloadSchemaRegister(d, attr)` | `SchemaRegister` | `domain_handle_id`, `schema_id`, `schema` (raw) |
| `nvtxPayloadEnumRegister(d, attr)` | `EnumRegister` | `domain_handle_id`, `schema_id`, `entries` (raw) |
| `nvtxRangePushPayload(d, data, n)` | `PushPayload` | `thread_id`, `domain_handle_id`, `payloads` (raw bytes + schema IDs) |
| `nvtxRangePopPayload(d, data, n)` | `PopPayload` | `thread_id`, `domain_handle_id`, `payloads` (raw bytes + schema IDs) |
| `nvtxRangeStartPayload(d, data, n)` | `RangeStartPayload` | `range_handle_id`, `domain_handle_id`, `payloads` |
| `nvtxRangeEndPayload(d, id, data, n)` | `RangeEndPayload` | `range_handle_id`, `domain_handle_id`, `payloads` |
| `nvtxMarkPayload(d, data, n)` | `MarkPayload` | `thread_id`, `domain_handle_id`, `payloads` |

Payload data is forwarded as raw bytes with schema IDs. The analyzer
interprets them using prior `SchemaRegister` events.

### Attributes serialization

When an `nvtxEventAttributes_t` is present, it is serialized as:

```
NvtxAttributes {
    category_id: u32,           // 0 = no category
    color: Option<u32>,         // ARGB, present if colorType != UNKNOWN
    payload: Option<NvtxPayload>,
    message: Option<NvtxMessage>,
}

NvtxPayload = U64(u64) | I64(i64) | F64(f64) | U32(u32) | I32(i32) | F32(f32)

NvtxMessage = Ascii(String) | RegisteredHandle(u64)
```

For registered string messages, the raw handle ID is emitted (not the
resolved string). The analyzer resolves it using prior `RegisterString`
events.
