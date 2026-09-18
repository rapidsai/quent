<!--
SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
SPDX-License-Identifier: Apache-2.0
-->

# Issue 618 research

Research date: 2026-09-17. This document records observations, not implemented
behavior. The [comparison](618-plan.md) links two alternative implementation
plans; the current preference is declarative reconstruction in approach 2.

## Scope and revision anchors

[Issue #618](https://github.com/rapidsai/quent/issues/618) asks for model/schema
descriptions of NVTX and future auxiliary streams, shared transport/analysis,
Rust/CXX/Python parity, and removal of per-event-type compatibility work from
`quent-open`. Source attachment remains separate from ordinary transport.

| Source | Inspected state |
| --- | --- |
| Local branch | `unify_nvtx_exp_imp_analy`, initially clean |
| Local commit | `5e6818e89ccde0a41dc08a12819938a093969665` |
| [PR #638](https://github.com/rapidsai/quent/pull/638) | Merged 2026-09-15 at 14:23:07 UTC |
| [PR #699](https://github.com/rapidsai/quent/pull/699) | Open; inspected head `db49fbb6115c0b22f688bd9417a192fb5526993b` |

The [repository freeze](../../AGENTS.md) is still active because #699 is open.
Planning is possible now; implementation overlapping that migration must wait
for both PRs to merge or for explicit user direction permitting the overlap.
Both plans assume implementation on the merged #699 architecture.

## Existing transport is already generic

[`quent-events`](../../crates/events/src/lib.rs) defines `Event<T>` with entity
UUID, timestamp, and typed payload. `EntityEvent::NAME` identifies its stream.
[`quent-io-types`](../../crates/io/types/src/lib.rs) defines generic
`Exporter<T>`, `ExporterProvider<T>`, `Importer<T>`, and `ImporterProvider<T>`.
NVTX already uses these implementations through
[`NvtxEventEntity`](../../integrations/nvtx/bridge/src/lib.rs), a transparent
wrapper whose stream name is `NvtxEvent`.

Consequently, the missing unification is primarily model membership, generated
context ownership, analysis dispatch, and viewer composition. A new codec or
one physical file containing every event type is not required by the issue.

The common runtime already owns batching, cancellation, flushing, and runtime
lifetime in [`observer.rs`](../../crates/instrumentation/src/observer.rs).
`EventSender<T>` deliberately does not retain the owning observer. This matters
for a process-global source hook: capturing an owning observer in the hook would
keep its exporter alive for the lifetime of the process.

## The schema path can already describe ordinary additional streams

The schema currently contains entities, records, and annotations; entities have
typed events with once or multiple cardinality. Relevant sources:

- [Schema](../../crates/schema/src/schema/mod.rs),
  [events](../../crates/schema/src/schema/event.rs), and
  [builder composition](../../crates/schema/src/builder/mod.rs).
- [YAML AST](../../crates/yaml/src/ast.rs) and
  [lowering](../../crates/yaml/src/lower.rs).
- [Rust observer generation](../../crates/instrumentation-build/src/runtime/context.rs).
- [Umbrella event generation](../../crates/instrumentation-build/src/model.rs).
- [Stored-event generation](../../crates/store-build/src/lib.rs).

The Rust generator already loops over `schema.entities()` to create observers,
exporter bounds, and collector dispatch. The store generator uses the same
entities for typed loading and model umbrella conversion. A normal repeated
event added to the schema therefore needs no custom transport implementation.

Both schema language generators consume this same schema:
[CXX](../../experimental/vibe/codegen/cpp/src/lib.rs) and
[PyO3](../../experimental/vibe/codegen/python/src/lib.rs). They live in a
separate Cargo workspace. Root workspace tests do not cover them.

Two constraints affect the design:

1. [Annotations](../../crates/schema/src/schema/annotations.rs) explicitly do
   not change the wire representation. Metadata may select interpretation or
   convenience APIs, but must not secretly inject payload types or change
   serialized event layouts.
2. [DataType](../../crates/schema/src/schema/data_type.rs) has records, options,
   lists, primitives, and dynamic records, but no general tagged union. The
   native NVTX message and payload enums cannot simply be named in YAML today.

There is also a useful precedent for approach 2:
[`quent-fsm`](../../crates/fsm/src/lib.rs) has a typed semantic specification,
builder, serialized constraint payload, and schema validator. YAML's `fsms:`
syntax lowers to that representation in the
[lowerer](../../crates/yaml/src/lower.rs). Reconstruction could use a similar
validated extension mechanism without treating incomplete NVTX captures as
strict FSM violations. This is a proposed reuse of the extension pattern, not
a claim that the existing FSM engine already implements NVTX semantics.

## NVTX special cases that remain

| Boundary | Current special handling | Consequence for #618 |
| --- | --- | --- |
| Legacy CXX generation | `model.nvtx`, extra observer, `_nvtx_pipeline`, hook installation in [cxx_bridge.rs](../../crates/codegen/src/cxx_bridge.rs) | Replace with schema context ownership and a separate source adapter; do not port this block into the new generator |
| NVTX vocabulary | [NvtxEvent](../../integrations/nvtx/events/src/lib.rs), [attributes](../../integrations/nvtx/events/src/attributes.rs), [payloads](../../integrations/nvtx/events/src/payload.rs) | Preserve raw handles, tags, values, and all 12 captured event variants |
| Reconstruction | [NvtxModelBuilder](../../integrations/nvtx/analyzer/src/model.rs) | Retain its semantics inside common analysis orchestration |
| Server import | `import_context_events` in [nvtx-server](../../integrations/nvtx/server/src/lib.rs) | Replace direct stream-directory and format handling with the common store |
| Server analysis | Private context cache, task limiter, reconstruction lifecycle in the same server file | Move ownership/loading to the shared context service; retain presentation adapters |
| Viewer generation | `NvtxRoutes`, a direct `nvtx-server` dependency, separate importer in [wrapper.rs](../../crates/open/src/wrapper.rs) | Delegate route composition to the model viewer |
| Viewer compatibility | Missing-package retries in [viewer.rs](../../crates/open/src/viewer.rs) | Preserve historical behavior under one legacy protocol; new event types must not add retry branches |

[PR #544](https://github.com/rapidsai/quent/pull/544) explicitly identifies its
default-on CXX NVTX wiring as temporary support for
`sirius-db/sirius#1439`. Its human-written scope excludes schema-driven parity;
that is assigned to #618. Its automated review summary is not an authoritative
description of the final YAML behavior.

## Reconstruction and presentation invariants

[`NvtxModelBuilder::build`](../../integrations/nvtx/analyzer/src/model.rs)
stably orders events, resolves registrations in a preliminary pass, and then
reconstructs ranges and resources. That two-pass behavior supports registrations
arriving after their uses. Open ranges retain `end: None`; orphan closes are
counted rather than turned into invented spans. Reused active IDs preserve the
displaced interval and record an anomaly.

The [resolution tables](../../integrations/nvtx/analyzer/src/tables.rs) scan
the complete timestamp-ordered capture. Repeated names use the last registration
in that order, including when it follows the event that references it. Strings
key by `(domain, handle)` and categories by `(domain, category)` within a
context. Domain inventory retains first observation separately from captured
creation/destruction times. Approach 2 must declare these policies explicitly;
changing to lookup-at-event-time would change current behavior.

The official [NVTX documentation](https://nvidia.github.io/NVTX/) distinguishes
thread-local nested push/pop ranges from start/end ranges that can cross
threads. A generic strict FSM is not an equivalent replacement.

Relevant regression suites already exist:

- [Range reconstruction](../../integrations/nvtx/analyzer/tests/reconstruction.rs).
- [Push/pop nesting](../../integrations/nvtx/analyzer/tests/pushpop.rs).
- [Name resolution](../../integrations/nvtx/analyzer/tests/resolution.rs).
- [Resource lifetimes](../../integrations/nvtx/analyzer/tests/resource.rs).
- [Capture round trip](../../integrations/nvtx/analyzer/tests/roundtrip.rs).
- [UI contracts and tests](../../integrations/nvtx/ui/src/lib.rs).
- [HTTP/cache behavior and tests](../../integrations/nvtx/server/src/lib.rs).

Current HTTP endpoints are:

```text
GET  /api/nvtx/contexts/{context_id}/catalog?query_start=...
POST /api/nvtx/contexts/{context_id}/viewport?query_start=...
```

Preserve missing-stream 404 versus present-empty success, redacted import
errors, selector/body limits, concurrent-miss coalescing, and bounded blocking
work. Catalogs also depend on the query time origin, not just context ID.

[PR #736](https://github.com/rapidsai/quent/pull/736), already present locally,
groups presentation domains by name while retaining their source domain IDs.
Shared analysis must preserve that distinction and existing relative-time,
category, color, anomaly, and statistics behavior.

## Common store gaps to account for

[`quent-store` filesystem loading](../../crates/store/src/event/filesystem.rs)
already supports a generated stream inventory, per-file format selection,
multiple files per stream, and umbrella event conversion. It validates model
identity against `model.qmi`.

However, `event_files` returns an empty collection for a missing/non-directory
stream as well as for an empty directory. Iteration alone cannot preserve the
NVTX HTTP distinction between absent and present-empty streams. Add generic
availability inspection rather than another NVTX-specific filesystem probe.

`load_model_events` concatenates streams. It does not provide a global
timestamp merge. NVTX's stable timestamp ordering and registration pass must
remain explicit when replacing its importer.

## What #699 changes

The complete paginated GitHub changed-file list and selected files at the
inspected PR head show:

- Legacy `crates/model`, `crates/model-macros`, `crates/codegen`, and
  `crates/stdlib` are removed, along with legacy consumers.
- `quent-analyzer` gains
  [ContextId/ContextInventory/ContextIndex][migration-context]. Contexts and
  analysis target entity IDs are explicitly separate.
- The simulator gains a schema-generated
  [stored-event crate][migration-store]. Its
  [viewer][migration-analyzer] loads events through `Store<Simulator>`.
- [The migrated server][migration-server] still separately constructs an
  NVTX filesystem importer and NVTX routes.
- [The migrated open wrapper][migration-wrapper] still contains NVTX-specific
  dependencies and route setup. #699 does not resolve #618 by itself.

An engine can aggregate several process contexts. Combining their NVTX raw
handles into one unqualified reducer would mix unrelated domains, thread IDs,
and range IDs. Preserve context identity before engine-level aggregation, even
though live multi-context hook routing belongs to another issue.

## Source attachment is a separate workstream

[Injection](../../integrations/nvtx/injection/src/init.rs) stores its hook in a
`OnceLock`; installation is one-shot and returns `AlreadyInstalled` on a second
attempt. Capture is currently Linux 64-bit only. The vocabulary and replay
analysis need not inherit that platform dependency.

The current [injection manifest](../../integrations/nvtx/injection/Cargo.toml)
declares only `rlib`, although nearby comments describe a cdylib attach mode.
Verify the actual consumer/link setup when choosing capture smoke tests; do
not infer that a normal package build produces a loadable injection library.
The callback initializer also services each NVTX-using image, so moving the
adapter must preserve capture from instrumented shared libraries.

[Issue #696](https://github.com/rapidsai/quent/issues/696) explicitly owns
multi-context registration/unregistration, scoped routing, lifecycle ownership,
and replaying process metadata for later contexts. #618 should provide a typed
sink seam that #696 can use. It should not claim to solve #696 by creating
more observers or by silently ignoring repeated hook installation.

[Issue #374](https://github.com/rapidsai/quent/issues/374) separately tracks
coexistence with other tools. NVTX payload-extension capture is also outside
this task: `PayloadExtensionEvent` is vocabulary only and is not wired into
the current `NvtxEvent` capture surface.

## Provenance and verification limits

[`model.qmi`](../../crates/build-info/src/lib.rs) records model/analyzer source
and Quent Git pins. It is not currently a serialized model schema or an
event-stream plugin manifest. `quent-open` builds the producing analyzer from
those pins. Historical formats can remain supported through their historical
viewer path without teaching the new importer every old event layout.

This research used local source inspection and read-only GitHub queries.
Selected #699 files were read remotely at the pinned head; that branch was not
checked out or compiled. No implementation or application test changes were
made, and no runtime regression claims follow from this research.

[migration-context]: https://github.com/johanpel/quent/blob/db49fbb6115c0b22f688bd9417a192fb5526993b/crates/analyzer/src/context.rs
[migration-store]: https://github.com/johanpel/quent/blob/db49fbb6115c0b22f688bd9417a192fb5526993b/experimental/vibe/simulator/store/src/lib.rs
[migration-analyzer]: https://github.com/johanpel/quent/blob/db49fbb6115c0b22f688bd9417a192fb5526993b/experimental/vibe/simulator/analyzer/src/lib.rs
[migration-server]: https://github.com/johanpel/quent/blob/db49fbb6115c0b22f688bd9417a192fb5526993b/experimental/vibe/simulator/server/src/main.rs
[migration-wrapper]: https://github.com/johanpel/quent/blob/db49fbb6115c0b22f688bd9417a192fb5526993b/crates/open/src/wrapper.rs
