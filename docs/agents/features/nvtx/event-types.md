# NVTX Event Types

## NvtxEvent enum

The `NvtxEvent` enum represents all raw NVTX API calls. It lives in
the `quent-nvtx-events` crate at `integrations/nvtx/injection/events/`.

The application's top-level event enum must include a variant wrapping
`NvtxEvent`:

```rust
enum SimulatorEvent {
    QueryEngineEvent(QueryEngineEvent),
    Task(TaskEvent),
    Resource(ResourceEvent),
    Trace(TraceEvent),
    Nvtx(NvtxEvent),  // new variant
}
```

## install() trait bound

```rust
pub fn install<T>(sender: EventSender<T>, session_id: Uuid)
where
    T: From<NvtxEvent> + Serialize + Send + Debug + 'static
```

Internally, the Quent wrapper (`quent-nvtx`) calls
`quent_nvtx_injection::install_hook` with a closure that wraps the sender
to convert `NvtxEvent` into `T`
via `From` before sending. This follows the existing pattern — all domain
event types in Quent are wrapped as variants in a top-level enum.

## Enum variants

Mirrors the raw event table from
[injection-architecture.md](./injection-architecture.md):

```rust
pub enum NvtxEvent {
    // Push/Pop ranges
    Push(Push),
    Pop(Pop),

    // Start/End ranges
    RangeStart(RangeStart),
    RangeEnd(RangeEnd),

    // Marks
    Mark(Mark),

    // Domains
    DomainCreate(DomainCreate),
    DomainDestroy(DomainDestroy),

    // Registered strings
    RegisterString(RegisterString),

    // Categories
    NameCategory(NameCategory),

    // Thread naming
    NameThread(NameThread),

    // Resource naming
    ResourceCreate(ResourceCreate),
    ResourceDestroy(ResourceDestroy),

    // Payload extension
    SchemaRegister(SchemaRegister),
    EnumRegister(EnumRegister),
    PushPayload(PushPayload),
    PopPayload(PopPayload),
    RangeStartPayload(RangeStartPayload),
    RangeEndPayload(RangeEndPayload),
    MarkPayload(MarkPayload),
}
```

## Shared fields

Fields captured by the injection on every event (not inside the enum):

- `timestamp`: `TimeUnixNanoSec` — captured via `TimeUnixNanoSec::now()`
- `id`: `Uuid` — the session/application entity UUID

These are part of `Event<T>`, not `NvtxEvent` itself.

## Per-variant fields

```rust
pub struct Push {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
    pub attributes: Option<NvtxAttributes>,
}

pub struct Pop {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
}

pub struct RangeStart {
    pub range_handle_id: u64,
    pub domain_handle_id: Option<u64>,
    pub attributes: Option<NvtxAttributes>,
}

pub struct RangeEnd {
    pub range_handle_id: u64,
    pub domain_handle_id: Option<u64>,
}

pub struct Mark {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
    pub attributes: Option<NvtxAttributes>,
}

pub struct DomainCreate {
    pub domain_handle_id: u64,
    pub name: String,
}

pub struct DomainDestroy {
    pub domain_handle_id: u64,
}

pub struct RegisterString {
    pub domain_handle_id: u64,
    pub string_handle_id: u64,
    pub value: String,
}

pub struct NameCategory {
    pub domain_handle_id: Option<u64>,
    pub category_id: u32,
    pub name: String,
}

pub struct NameThread {
    pub os_thread_id: u32,
    pub name: String,
}

pub struct ResourceCreate {
    pub domain_handle_id: u64,
    pub resource_handle_id: u64,
    pub attributes: NvtxResourceAttributes,
}

pub struct ResourceDestroy {
    pub resource_handle_id: u64,
}

pub struct NvtxAttributes {
    pub category_id: u32,
    pub color: Option<u32>,
    pub payload: Option<NvtxPayload>,
    pub message: Option<NvtxMessage>,
}

pub enum NvtxPayload {
    U64(u64),
    I64(i64),
    F64(f64),
    U32(u32),
    I32(i32),
    F32(f32),
}

pub enum NvtxMessage {
    Ascii(String),
    RegisteredHandle(u64),
}
```

## Payload extension variants

```rust
pub struct SchemaRegister {
    pub domain_handle_id: Option<u64>,
    pub schema_id: u64,
    pub schema: RawPayloadSchema,
}

pub struct EnumRegister {
    pub domain_handle_id: Option<u64>,
    pub schema_id: u64,
    pub entries: Vec<RawPayloadEnum>,
}

pub struct PushPayload {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
    pub payloads: Vec<RawPayloadData>,
}

pub struct PopPayload {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
    pub payloads: Vec<RawPayloadData>,
}

pub struct RangeStartPayload {
    pub range_handle_id: u64,
    pub domain_handle_id: Option<u64>,
    pub payloads: Vec<RawPayloadData>,
}

pub struct RangeEndPayload {
    pub range_handle_id: u64,
    pub domain_handle_id: Option<u64>,
    pub payloads: Vec<RawPayloadData>,
}

pub struct MarkPayload {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
    pub payloads: Vec<RawPayloadData>,
}

pub struct RawPayloadData {
    pub schema_id: u64,
    pub data: Vec<u8>,  // raw payload bytes, interpreted by analyzer
}
```
