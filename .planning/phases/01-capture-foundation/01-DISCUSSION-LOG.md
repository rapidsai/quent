# Phase 1: Capture Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-13
**Phase:** 1-capture-foundation
**Areas discussed:** PR #87 adoption & layout, NVTX bindings & injection, Hot-path hand-off & overflow, Test app & harness shape

---

## PR #87 Adoption & Layout

### How to take on PR #87's code
| Option | Description | Selected |
|--------|-------------|----------|
| Rebase & adopt as base | Build directly on PR #87's `nvtx` branch, keeping commits/authorship | |
| Adopt then extend in our commits | Take the files, continue in our own phase commits | |
| Reference-only rebuild | Treat as design reference, re-implement from scratch | ✓ |
| Coordinate with Johan first | Align on ownership before building | |

**User's choice:** Reference-only rebuild
**Notes:** PR #87 design validated as sound during review; rebuild tracks its event vocabulary closely but is our own code.

### Where the NVTX crates live
| Option | Description | Selected |
|--------|-------------|----------|
| Adopt integrations/nvtx/ tree | New top-level integration crates, separable | ✓ |
| Under crates/ | Alongside existing application-agnostic crates | |
| Under domains/nvtx/ | Mirror domains/query_engine/ | |

**User's choice:** Adopt integrations/nvtx/ tree

### Payload-extension capture scope (CAP-03)
| Option | Description | Selected |
|--------|-------------|----------|
| Yes — capture verbatim now | Wire payload callbacks in Phase 1 | |
| Model now, defer emission | Keep types, don't emit (PR #87 style) | |
| Let me check NVTX first | Flag for researcher | ✓ |

**User's choice:** Let me check NVTX first (→ R-01)
**Notes:** Assess libcudf/cuCascade payload surface + NVTX payload API cost before committing.

---

## NVTX Bindings & Injection

### NVTX header source
| Option | Description | Selected |
|--------|-------------|----------|
| Git dependency (as PR #87) | NVIDIA/NVTX via git rev | |
| Vendor headers in-repo | Copy headers, offline CI | |
| crates.io nvtx-sys | Published crate | |
| Let researcher compare | Defer with criteria | ✓ |

**User's choice:** Let researcher compare (→ R-02)

### macOS support scope
| Option | Description | Selected |
|--------|-------------|----------|
| Linux-first, keep macOS compiling | Test Linux, macOS compiles | |
| Full Linux + macOS parity | CI on both | |
| Linux-only for now | Drop macOS codepaths | ✓ |

**User's choice:** Linux-only for now

### Attach mechanism
| Option | Description | Selected |
|--------|-------------|----------|
| Both, prove runtime path | Runtime cdylib + link-time | |
| Link-time only for Phase 1 | PR #87 strong-symbol only | |
| Runtime path primary | NVTX_INJECTION64_PATH cdylib primary | |
| Let researcher assess | Confirm real attach + build order | ✓ |

**User's choice:** Let researcher assess (→ R-03)

---

## Hot-Path Hand-off & Overflow

### Overflow policy under overload
| Option | Description | Selected |
|--------|-------------|----------|
| Bounded queue, drop + count | Bounded ring, drop + counter | |
| Rely on unbounded EventSender | Straight through (PR #87) | |
| Bounded, block briefly then drop | Park then drop | |
| Let researcher recommend | Survey profiler designs | ✓ |

**User's choice:** Let researcher recommend (→ R-04)
**Notes:** Hard constraint recorded: bounded + non-blocking (CAP-05). EventSender is unbounded tokio mpsc.

### Buffer/hand-off placement
| Option | Description | Selected |
|--------|-------------|----------|
| In the injection crate (sink-agnostic) | Injection owns buffer, generic sink | |
| In the quent-nvtx bridge | Thin injection, buffer in bridge | |
| Let researcher/planner decide | Defer placement | ✓ |

**User's choice:** Let researcher/planner decide
**Notes:** Constraint: keep injection separable/upstreamable (D-03).

### CAP-05 proof level
| Option | Description | Selected |
|--------|-------------|----------|
| CI stress test | Full high-frequency harness | |
| Benchmark + assertions | Micro-bench + drop-count test | |
| Design-level for now | Design + basic test, load → Phase 5 | ✓ |

**User's choice:** Design-level for now
**Notes:** Softens Phase 1 success-criterion 5; hard proof moves to Phase 5.

---

## Test App & Harness Shape

### Harness shape
| Option | Description | Selected |
|--------|-------------|----------|
| Standalone binary + subprocess harness | Deterministic app run as subprocess | |
| In-process test only (PR #87 style) | Single in-process test | |
| Let researcher/planner detail | Principle agreed, structure to planning | ✓ |

**User's choice:** Let researcher/planner detail
**Notes:** Principle locked — subprocess + deterministic standalone app (VAL-01 + VAL-02); structure to planning.

### Capture/assertion path
| Option | Description | Selected |
|--------|-------------|----------|
| ndjson exporter to file | Real exporter to file, harness reads back | ✓ |
| Custom test sink (stdout/JSON) | Lightweight test-only sink | |
| Let researcher/planner decide | Defer format | |

**User's choice:** ndjson exporter to file
**Notes:** Exercises real CAP-04 pipeline; human-readable.

### Coverage breadth
| Option | Description | Selected |
|--------|-------------|----------|
| Full single + multi-threaded | All event kinds + multiple threads | ✓ |
| Full coverage, single-threaded | All kinds, one thread | |
| Core kinds first | Common kinds now | |

**User's choice:** Full single + multi-threaded
**Notes:** De-risks Phase 2 cross-thread reconstruction.

---

## Claude's Discretion
- Hot-path buffer placement (injection vs bridge) — researcher/planner, constrained by separability.
- Exact test-app/harness file structure — planning.

## Deferred Ideas
- macOS injection support → later phase (D-04).
- High-frequency load/stress validation → Phase 5 (D-08).
- Payload-extension decode/render → v2 (unchanged); payload capture itself is R-01 (under research).
