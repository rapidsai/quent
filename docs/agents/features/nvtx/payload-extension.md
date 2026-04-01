# Payload Extension: Quent-NVTX Correlation

## Goal

Allow applications to attach Quent FSM context (entity UUID + state) to
NVTX ranges, enabling the analyzer to correlate NVTX span trees with
FSM state transitions.

## Two layers

### Layer 1: Raw NVTX capture

The injection captures all standard NVTX calls regardless. Applications
and libraries using vanilla NVTX get their ranges captured. No payload
extension needed.

Library NVTX ranges nested inside an application's annotated range on
the same thread inherit the FSM correlation implicitly via the span tree
structure and timestamps.

### Layer 2: Quent-NVTX convenience header

A C/C++ header (`quent_nvtx.h`) that wraps NVTX with Quent-specific
payloads. Applications that want explicit FSM correlation use this header.

Under the hood, it uses the NVTX payload extension to attach structured
data to standard NVTX API calls.

## NVTX payload extension overview

The payload extension (`nvToolsExtPayload.h`) allows attaching structured
binary data to NVTX events. Key concepts:

- **Schema**: Defines the layout of a binary payload (field names, types,
  offsets). Registered via `nvtxPayloadSchemaRegister()`.
- **Payload data**: `nvtxPayloadData_t { schemaId, size, payload_ptr }`.
  Attached to events via `nvtxEventAttributes_t` or dedicated functions
  (`nvtxRangePushPayload`, etc.).
- **Static schemas**: Caller can assign a fixed schema ID via
  `NVTX_PAYLOAD_ENTRY_TYPE_SCHEMA_ID_STATIC_START + offset`.

## Quent payload schema

A static schema with a well-known ID:

```c
typedef struct {
    uint64_t entity_id_hi;  // UUID upper 64 bits
    uint64_t entity_id_lo;  // UUID lower 64 bits
} quent_nvtx_payload_t;
```

Schema entries:
- `entity_id_hi`: `NVTX_PAYLOAD_ENTRY_TYPE_UINT64`, offset 0
- `entity_id_lo`: `NVTX_PAYLOAD_ENTRY_TYPE_UINT64`, offset 8

The schema ID is statically assigned (well-known constant). This allows
the analyzer to recognize Quent payloads without needing the registration
event in the stream.

## Convenience header API

Located at `integrations/nvtx/include/quent_nvtx.h`.

```c
#include "quent_nvtx.h"

// Push a range annotated with a Quent entity UUID
quent_nvtx_push(quent_uuid_t entity_id, const char* name);

// Pop the range
quent_nvtx_pop();

// Start/End variants
quent_nvtx_range_id_t quent_nvtx_range_start(quent_uuid_t entity_id, const char* name);
quent_nvtx_range_end(quent_nvtx_range_id_t id);

// Mark (instant event)
quent_nvtx_mark(quent_uuid_t entity_id, const char* name);
```

Under the hood:
1. On first use, registers the Quent payload schema with a well-known
   static ID in a Quent-specific NVTX domain.
2. Populates `quent_nvtx_payload_t` with the UUID.
3. Attaches the payload via `nvtxPayloadData_t` in the event attributes.
4. Calls the standard NVTX domain API (`nvtxDomainRangePushEx`, etc.).

The range `name` parameter becomes the NVTX message (and thus the span
name in the Quent trace).

## Injection side

The injection implements `InitializeInjectionNvtxExtension` and registers
handlers for the payload extension slots:

| Slot | Callback | Purpose |
|------|----------|---------|
| 0 | `nvtxPayloadSchemaRegister` | Forward schema definition |
| 1 | `nvtxPayloadEnumRegister` | Forward enum definition |
| 2 | `nvtxMarkPayload` | Forward mark with payload |
| 3 | `nvtxRangePushPayload` | Forward push with payload |
| 4 | `nvtxRangePopPayload` | Forward pop with payload |
| 5 | `nvtxRangeStartPayload` | Forward start with payload |
| 6 | `nvtxRangeEndPayload` | Forward end with payload |

All forwarding is raw — the injection serializes the schema ID and
payload bytes into the event stream. No interpretation.

## Analyzer side

The analyzer:
1. Builds a schema registry from `SchemaRegister` events in the stream.
2. When processing events with payloads, looks up the schema by ID.
3. For the well-known Quent schema ID, extracts `entity_id` and
   correlates the NVTX span with the corresponding FSM entity.
4. For unknown schemas (e.g., from NCCL or other libraries), stores
   the raw payload bytes with the schema ID for potential future use.

## Correlation mechanism

Given:
- An NVTX push/pop range on thread T with Quent payload
  `{ entity_id: X }` from t1 to t2
- FSM entity X in state S from t1 to t2
- Library NVTX ranges nested under the application's range on the same
  thread

The analyzer links:
- The application's span → FSM entity X (via payload)
- Nested library spans → FSM entity X (via tree containment on thread T)
- All spans → FSM state S (via timestamp overlap with state duration)

## Stateless injection

The injection remains stateless. Payload extension handling adds:
- Schema registration events (forwarded, not stored)
- Raw payload bytes on events (forwarded, not interpreted)

No schema registry in the injection. No payload parsing.
