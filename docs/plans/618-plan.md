<!--
SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
SPDX-License-Identifier: Apache-2.0
-->

# Issue 618: two approaches to shared event analysis

Status: design discussion for
[issue #618](https://github.com/rapidsai/quent/issues/618), prepared 2026-09-17.
This proposal contains research and two implementation plans. It does not
implement the issue or select a final architecture.

The current preference is **approach 2: declarative reconstruction**. The aim
is to get feedback on its scope, schema design, and implementation cost before
handing the selected plan to sol for implementation.

## Documents

| Document | Purpose |
| --- | --- |
| [Approach 1: reusable reconstruction components](618-approach-1-components.md) | Share transport and analysis orchestration; keep NVTX interpretation in a reusable Rust component |
| [Approach 2: declarative reconstruction](618-approach-2-declarative.md) | Put reconstruction rules in the schema and execute them with shared analysis primitives |
| [Research](618-research.md) | Current code boundaries, source references, migration dependencies, and behavior to preserve |

## The difference, with an example

Both approaches consume the same typed events:

```text
10 ms: RangeStart { range_id: 42, name: "scan" }
25 ms: RangeEnd   { range_id: 42 }

Result: scan, start=10 ms, end=25 ms, duration=15 ms
```

In approach 1, the shared analyzer calls a Rust component whose code matches
starts and ends by `range_id`. The schema describes the events and selects
the component. The existing NVTX reconstruction implementation is reused.

In approach 2, the schema also declares which events open/close an interval,
which fields identify the pair, and what to do with incomplete captures. A
generic interval engine executes that declaration. Another source, such as
HTTP request start/end events, can reuse the engine by declaring its rules.

For nested NVTX push/pop events, approach 1 keeps per-thread/domain stack
logic in the component. Approach 2 declares the partition keys and stack
semantics and uses a generic stack engine. Its full NVTX plan also covers
registered names, resource lifetimes, marks, and anomaly accounting.

## Comparison

| Dimension | Approach 1 | Approach 2 |
| --- | --- | --- |
| Meaning of shared analysis | Shared loading, dispatch, lifecycle, and caching | Shared infrastructure plus reusable reconstruction algorithms driven by schema rules |
| Location of NVTX pairing rules | Rust reconstruction component | Validated schema declaration |
| Ordinary new event stream | Schema change and regeneration | Schema change and regeneration |
| New source using existing pairing/stack patterns | Reusable component or adapter using shared infrastructure | Schema rules and regeneration; no source-specific reducer |
| New analysis pattern | Add a reusable component | Extend the generic rule vocabulary and engine, then declare it in schemas |
| Existing NVTX logic | Production component | Differential test oracle during migration; replaced in the new production path |
| Schema work | Context-stream convention and optional component binding | Those stream conventions plus a typed, versioned reconstruction specification |
| Runtime work | Common service with component dispatch | Common service plus generic interval, stack, registry, inventory, and projection primitives |
| Main advantage | Smaller semantic migration; reuse tested reconstruction | New sources with familiar semantics require less custom analysis code |
| Main risk | Per-source interpretation continues to accumulate | A rule language can become too broad or subtly change existing semantics |
| Relative effort | Lower | Higher; parser, validation, engine, and differential testing are additional work |

These are alternatives, not a requirement to implement approach 1 first.
Approach 2 can stage shared infrastructure before switching reconstruction;
that staging does not commit the project to component-specific reducers.

## Requirements shared by both plans

- Use the same composed schema for Rust, CXX, Python, collector routing, and
  typed stored events. Extend the existing generic runtime and I/O backends.
- Keep source attachment separate. A schema stream does not claim the
  process-global NVTX hook. No-op and collector contexts never install it.
- Preserve NVTX capture, incomplete-range handling, ordering, name resolution,
  context isolation, HTTP contracts, and UI behavior.
- Load and cache through common context services. Preserve missing versus
  present-empty streams and keep source handle spaces isolated by context.
- Give `quent-open` one versioned model-viewer entrypoint for new artifacts.
  Preserve old pinned artifacts through the historical viewer path.
- Demonstrate extensibility with a second source. Approach 2 must additionally
  prove that a second source's ranges reconstruct using declarations alone.
- Leave live multi-context hook routing to
  [#696](https://github.com/rapidsai/quent/issues/696) and multi-tool
  coexistence to [#374](https://github.com/rapidsai/quent/issues/374).

## Shared proposals that need independent review

The approach decision concerns where reconstruction rules live. The following
choices can be discussed independently and applied to either plan:

| Proposal | Rationale and open question |
| --- | --- |
| Context-scoped schema entity with repeating events | Reuses current generation/store machinery; is this a suitable public representation of an auxiliary stream? |
| Lossless tagged records for NVTX message/payload values | Fits current schema types without adding general unions; is a new wire shape acceptable, or must current serialized bytes remain compatible? |
| Common context service and generated analysis dispatch | Removes independent loaders/caches; what is the narrowest useful shared API after #699? |
| Explicit source-attachment helper | Prevents collector/no-op contexts from claiming a global hook; migration recipes must preserve capture for former default-on CXX consumers |
| Optional viewer protocol in provenance | Separates one historical compatibility boundary from future event types; is this preferable to another capability-discovery mechanism? |

The tagged-record proposal preserves values, including raw float bits, but
changes how new captures encode them. Neither plan assumes a new analyzer can
directly decode every historical NVTX stream. Both require tests showing that
old captures still open through their recorded source pins.

## Feedback requested

1. Is approach 2 the right scope for #618, given the goal of extending analysis
   through schemas, or should the first delivery retain Rust components?
2. Does the bounded rule set in approach 2 cover the current NVTX behavior
   cleanly? Which operation or edge case would force source-specific logic?
3. Should structured YAML rules lower into a validated schema annotation, like
   existing semantic extensions, or become a first-class schema data type?
4. Are the proposed stream representation, wire-format boundary, and viewer
   protocol appropriate for consumers such as Sirius?
5. Are there existing analysis abstractions or migration changes that these
   plans should reuse before introducing new modules or crates?

## Implementation gate and handoff

At the latest check on 2026-09-17,
[PR #638](https://github.com/rapidsai/quent/pull/638) is merged and
[PR #699](https://github.com/rapidsai/quent/pull/699) remains open at
`db49fbb6115c0b22f688bd9417a192fb5526993b`.
The [migration freeze](../../AGENTS.md) remains active for implementation.
Documentation review can proceed now. Both plans target the merged #699
architecture, with an API/path refresh as their first implementation step.

After discussion, record the selected approach and any changes to the shared
proposals here. Hand that plan, the research, and the review decisions to sol.
Do not start implementation, silently switch approaches, or treat a partial
NVTX-only demonstration as completion of #618.

## Validation of this proposal

This is a documentation-only change. Validate Markdown, local document links,
and the commit diff. The implementation tests listed in the two plans are
future acceptance criteria, not tests already executed for this proposal.

```sh
rumdl check docs/plans/618*.md
git diff --check
```
