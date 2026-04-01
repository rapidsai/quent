# NVTX Domains and Categories

## Domains

### NVTX semantics

An NVTX domain is a logical namespace created via `nvtxDomainCreateA(name)`.
Domains scope:

- **Categories**: Category ID 1 in domain A is independent of category ID 1
  in domain B.
- **Registered strings**: String handles are per-domain.
- **Push/Pop stacks**: Each domain has its own per-thread push/pop stack.

There is always an implicit "default domain" (represented as a NULL handle)
used by the non-domain API (`nvtxRangePushA`, `nvtxMarkA`, etc.).

### Quent mapping: string attribute

Domains are represented as a string attribute `nvtx.domain` on each
analyzed event/span/FSM transition. Domain names are resolved by the
analyzer from `DomainCreate` events in the stream.

**Rationale**: NVTX domains are a flat namespace with no hierarchy or
capacity. Quent Resource Groups imply structural parent/child containment,
which would be a semantic mismatch.

### Capture (injection library)

The injection is stateless. On domain API calls, it emits raw events:

- `DomainCreate { domain_handle_id, name }` — records the name
- `DomainDestroy { domain_handle_id }` — records the destruction

Subsequent events reference the `domain_handle_id` as a raw integer.
The injection does not maintain a domain registry.

### Analysis (analyzer)

The analyzer builds a `domain_handle_id → name` lookup from
`DomainCreate` / `DomainDestroy` events. When processing other events,
it resolves `domain_handle_id` to the domain name.

## Categories

### NVTX semantics

A category is a `uint32_t` ID that can optionally be given a name via
`nvtxDomainNameCategoryA(domain, id, name)`. Categories are scoped to a
domain.

Categories are attached to events via `nvtxEventAttributes_t.category`.

### Quent mapping: string attribute

Categories are represented as a string attribute `nvtx.category`.

### Capture (injection library)

Category naming calls are forwarded as raw events:

- `NameCategory { domain_handle_id?, category_id, name }`

Events that carry a `category_id` in their attributes include it as a raw
integer.

### Analysis (analyzer)

The analyzer builds a `(domain_handle_id, category_id) → name` lookup from
`NameCategory` events. When processing events with a `category_id`:

- If the category has been named, the string name is used.
- If the category has not been named, the numeric ID is stored as a string
  (e.g., `"42"`).
- Category 0 means "no category" in NVTX and is omitted.

Note: if a category is used before being named, earlier events will have
the numeric ID. This is acceptable — the analyzer processes events in
order and applies names as they become available.

## Registered strings

### NVTX semantics

`nvtxDomainRegisterStringA(domain, string)` returns an opaque
`nvtxStringHandle_t`. This handle can be used in `nvtxEventAttributes_t`
with `messageType = NVTX_MESSAGE_TYPE_REGISTERED`.

Purpose: performance optimization — register once, use the lightweight
handle on the hot path.

### Capture (injection library)

Registration calls are forwarded as raw events:

- `RegisterString { domain_handle_id, string_handle_id, value }`

When an event references a registered string handle in its message field,
the raw `string_handle_id` is included (not the resolved string).

### Analysis (analyzer)

The analyzer builds a `(domain_handle_id, string_handle_id) → value` lookup
from `RegisterString` events and resolves handles when processing subsequent
events.
