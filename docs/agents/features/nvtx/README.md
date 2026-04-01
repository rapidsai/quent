# NVTX Injection for Quent

This directory contains design documents for integrating NVTX (NVIDIA Tools
Extension) with Quent. The goal is to build a custom NVTX injection library
that captures NVTX API calls from instrumented applications and converts them
into Quent events for analysis and visualization.

## Documents

- [nvtx-injection-mechanism.md](./nvtx-injection-mechanism.md) — How NVTX injection libraries work
- [injection-architecture.md](./injection-architecture.md) — Stateless forwarder design (resolved Q1)
- [event-transport.md](./event-transport.md) — Reuse Quent Context + exporter (resolved Q2)
- [event-mapping.md](./event-mapping.md) — Mapping NVTX concepts to Quent modeling primitives
- [push-pop-ranges.md](./push-pop-ranges.md) — Push/Pop ranges mapped to Quent Traces
- [start-end-ranges.md](./start-end-ranges.md) — Start/End ranges mapped to a new FSM type
- [domains-and-categories.md](./domains-and-categories.md) — NVTX domains and categories as attributes
- [payload-extension.md](./payload-extension.md) — Quent-NVTX correlation via payload extension
- [installation.md](./installation.md) — Programmatic injection via install()
- [event-types.md](./event-types.md) — NvtxEvent enum, per-variant fields, From<NvtxEvent> bound
- [analyzer-integration.md](./analyzer-integration.md) — NvtxModelBuilder, event routing, FSM correlation
- [open-questions.md](./open-questions.md) — Unresolved design decisions
