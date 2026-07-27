---
phase: 01
slug: capture-foundation
status: verified
threats_open: 0
asvs_level: 1
created: 2026-07-14
---

# Phase 01 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Verification method: each declared mitigation confirmed against implementation
> code (file:line), not documentation or intent. Implementation files are
> read-only; no code was modified by this audit.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| NVTX C caller → Rust callbacks | Instrumented app / NVTX runtime invokes `extern "C"` subscribers installed in the CORE/CORE2 tables | Caller-owned `const char*` messages, `nvtxEventAttributes_t`/`nvtxResourceAttributes_t` structs, raw handles |
| cdylib load → process | NVTX `dlopen`s the `.so` via `NVTX_INJECTION64_PATH`; ELF `.init_array` reads env and installs the pipeline | `QUENT_NVTX_OUTPUT_DIR`, `QUENT_NVTX_SESSION` env vars |
| Injection hook → drain thread | Bounded lock-free ring hand-off (app producer → drain consumer) | `Event<NvtxEventEntity>` values |
| Drain → filesystem | ndjson exporter writes captured events | Serialized NVTX events (opaque handles, verbatim strings) |
| Supply chain | Cargo dependency graph, NVTX git header source | crossbeam-queue, serde, uuid, thiserror; NVIDIA/NVTX pinned rev |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Evidence | Status |
|-----------|----------|-----------|-------------|------------|----------|--------|
| T-01-01 | Tampering | events ndjson deser | accept | Test-only fixtures; no untrusted network input | `events/Cargo.toml` (serde_json is a dev-dependency only); `payload.rs:87-144` round-trip tests | closed |
| T-01-02 | Info disclosure | raw u64 handles in vocabulary | accept | Opaque ids captured verbatim by design (D-01); not secrets | `payload.rs:22-50`; `convert.rs` verbatim handle capture; `init.rs:38-43` synthetic opaque handles | closed |
| T-01-SC | Tampering (supply chain) | events crate deps | mitigate | No new external packages; serde/uuid already workspace deps | `events/Cargo.toml` — only `serde.workspace`, `uuid.workspace`, dev `serde_json.workspace` | closed |
| T-02-01 | DoS | push/pop callbacks | mitigate | `catch_unwind` wraps every callback body; panic never crosses C ABI | `callbacks.rs:27` (push), `callbacks.rs:40` (pop) | closed |
| T-02-02 | Tampering/DoS | caller-owned `const char*` message args | mitigate | Message bytes copied into owned `String` before return | `convert.rs:418-427` (`copy_cstr`), `convert.rs:360-366` (ASCII decode copies in) | closed |
| T-02-03 | Info disclosure/DoS | reading `nvtxEventAttributes_t` past `size` | mitigate | Every member read gated on struct's declared `size` via `read_present` | `convert.rs:268-275` (`read_present` guard `offset + size_of::<T>() <= size`), `convert.rs:231-252` (`read_attributes`) | closed (see WR-02 residual) |
| T-02-SC | Tampering (supply chain) | NVTX git-dep + bindgen/cc/cargo_metadata | mitigate | NVTX pinned to concrete rev + allow-listed in deny.toml; build-deps optional/offline by default | `injection/Cargo.toml` (`rev = "7d113f...9e"`), `deny.toml:61` (`allow-git NVIDIA/NVTX`), `build.rs:11-21` (hermetic default) | closed |
| T-03-01 | DoS | unbounded allocation under high emission | mitigate | Bounded `ArrayQueue` + drop-and-count front; producer never blocks | `instrumentation/src/lib.rs:82` (`RING_CAPACITY`), `:106-110` (`push_or_drop`), `:190` (ring init) | closed |
| T-03-02 | DoS | producer blocking on backpressured sink | mitigate | Drain runs off app thread; non-blocking `EventSender::send`, never the collector's blocking mpsc | `instrumentation/src/lib.rs:118-133` (`drain_loop`), `:195-206` (drain thread), `:210-212` (hook only pushes ring) | closed |
| T-03-03 | Tampering | ndjson deser in harness | accept | Test-only file from same process run; not untrusted input | `instrumentation/src/lib.rs:326-335` (test-only importer) | closed |
| T-03-SC | Tampering (supply chain) | crossbeam-queue install | mitigate | Audited [OK]; crates.io registry allow-listed; no new blocking checkpoint | `instrumentation/Cargo.toml` (`crossbeam-queue = "0.3"`), `deny.toml:56` (`allow-registry crates.io`), no advisory ignore added | closed |
| T-04-01 | DoS | every new `extern "C"` callback | mitigate | `catch_unwind` at each new callback boundary (all 12 subscribers) | `callbacks.rs:27,40,51,73,89,98,109,122,137,147,162,175` — every callback body wrapped | closed (see note) |
| T-04-02 | Tampering/DoS | caller-owned `const char*` on marks/domains/registered strings | mitigate | Strings copied in-callback; registered strings captured once, handle-only thereafter | `convert.rs:149-157` (`register_string` copies once), `:367-369` (`RegisteredHandle` keeps raw handle, no deref), `:129-133,163-181` (name copies) | closed |
| T-04-03 | Info disclosure/DoS | reading attribute members past `size` on new kinds | mitigate | `size`/`version` honored in each conversion via `read_present` | `convert.rs:382-412` (`read_resource` guards every field on `size`), `:390` (`size` read at head) | closed (see WR-02 residual) |
| T-04-SC | Tampering (supply chain) | new packages in plan 04 | mitigate | No new packages introduced in plan 04 | `injection/Cargo.toml` / `instrumentation/Cargo.toml` unchanged dep sets vs. prior plans | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01 | T-01-01 | ndjson deserialization of `NvtxEvent` is exercised only by in-crate tests (`serde_json` is a dev-dependency); no runtime path deserializes untrusted or network input in Phase 1. | Phase plan author | 2026-07-14 |
| AR-02 | T-01-02 | Raw `u64` NVTX handles are opaque process-local identifiers captured verbatim (D-01), not secrets or capability tokens; disclosure carries no confidentiality impact. | Phase plan author | 2026-07-14 |
| AR-03 | T-02-04 | The capture `.so` is loaded only into processes that opt in via `NVTX_INJECTION64_PATH`; this is a documented, operator-controlled trust boundary, not an ambient elevation. | Phase plan author | 2026-07-14 |
| AR-04 | T-02-05 | The strong-symbol override of NVTX's weak symbol is gated behind the non-default `static-injection` feature (`symbol.c` compiled only then); the primary cdylib path never links it. | Phase plan author | 2026-07-14 |
| AR-05 | T-03-03 | The e2e harness deserializes an ndjson file produced by the same test run; it is trusted, same-process data, not external input. | Phase plan author | 2026-07-14 |

---

## Residual Notes (from code review 01-REVIEW.md, weighed but non-blocking)

These are recorded for traceability. None reopens a declared threat at ASVS L1;
they are soundness/correctness edges, not the over-read or crash the register
describes.

| Ref | Bearing | Assessment |
|-----|---------|------------|
| WR-02 | T-02-03 / T-04-03 | The payload/identifier union is always read as a full 8-byte `u64` (`convert.rs:306-307,396-397`). This does **not** read past the struct's declared `size` — the NVTX union is 8 bytes wide, so `read_present::<u64>` stays within `size`. The declared over-read / info-disclosure threat is therefore genuinely closed. The residual is a *technically-UB read of uninitialized union padding* for 32-bit-tagged values (masked away by `as u32`), confined to the app's own passed struct; no cross-boundary disclosure. Tracked as a soundness follow-up, not a security blocker. |
| WR-04 | non-breaking coexistence constraint | `RangePush`/`RangePop` return a constant `0` instead of NVTX nesting depth (`callbacks.rs:36,43`). This bears on the project's "do not break the observed app" constraint but is a behavioral/correctness deviation, not a threat in this register. No mapped security threat; flagged for phase owner awareness. |

*Note on T-04-01: the register cited "14 total" callbacks; the implementation
has 12 `extern "C"` subscribers, and every one wraps its full body in
`catch_unwind`. The invariant (no panic crosses the C ABI at any callback) holds
across all subscribers; the count figure was a plan-time estimate.*

---

## Unregistered Flags

None. No `## Threat Flags` section appears in any Phase 01 SUMMARY; no new attack
surface was declared during implementation without a threat mapping.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-07-14 | 17 | 17 | 0 | gsd-security-auditor |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-07-14
