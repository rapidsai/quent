// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Hand-written, framework-free reconstruction core for captured NVTX events.
//!
//! Turns a stream of verbatim [`NvtxEvent`](nvtx_events::NvtxEvent)s — carried in
//! Quent's [`Event`](quent_events::Event) envelope — into an in-memory
//! [`NvtxModel`] of plain [`NvtxSpan`]s.
//!
//! The core is deliberately **off** the shared analysis framework: it does not
//! depend on `quent-analyzer` or `quent-model`, builds no `RtFsm`, and uses none
//! of the `model!` / `fsm!` / `entity!` macro DSL. It defines its own span type
//! instead. That independence is what makes it **tolerant by construction** —
//! foreign telemetry is untrusted and frequently malformed, so:
//!
//! - out-of-order events are replayed in timestamp order,
//! - duplicate timestamps reconstruct deterministically, preserving arrival order,
//! - zero-duration spans are legal, and out-of-order pairs are clamped rather than rejected,
//! - a range that is never closed is closed at trace end and flagged synthetic,
//! - an orphan close with no matching open is logged and skipped.
//!
//! No anomaly in the event stream aborts reconstruction or panics.

mod error;
mod model;
mod span;

pub use error::{NvtxModelError, NvtxModelResult};
pub use model::{NvtxModel, NvtxModelBuilder};
pub use span::{NvtxSpan, SpanId, SpanKind};

// Re-exported so consumers can read span attributes without depending on the
// vocabulary crate directly. Carried verbatim, exactly as captured.
pub use nvtx_events::{NvtxColor, NvtxPayload};
