// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Hand-written, framework-free reconstruction core for captured NVTX events.
//!
//! Turns a stream of verbatim [`NvtxEvent`](nvtx_events::NvtxEvent)s — carried in
//! Quent's [`Event`](quent_events::Event) envelope — into an in-memory
//! [`NvtxModel`] of plain [`NvtxSpan`]s.
//!
//! A capture is a partial observation: it watches a process that was already
//! running and keeps running afterwards. Incomplete pairs are therefore ordinary
//! input, not errors — and this layer resolves what the stream referenced
//! without substituting anything for what it never said. An open with no close
//! keeps everything the open stated and has [`end`](NvtxSpan::end) `None`, so
//! what to bound it at is the consuming analysis's decision rather than one made
//! here. A close with no open produces no span at all, because the closing
//! events carry only a correlation key — no name, no attributes — and is counted
//! in [`ReconstructionAnomalies`] instead.
//!
//! Events may arrive out of timestamp order, and a label's registration may
//! arrive after the range using it or never; replay sorts first and resolves
//! names in a prior pass. See [`NvtxModelBuilder::build`].
//!
//! The crate defines its own span type and depends on neither the shared
//! analyzer nor the shared model crate, which is what lets the above states be
//! representable at all.

mod anomalies;
mod model;
mod ranges;
mod resource;
mod span;
mod stats;
mod tables;

pub use anomalies::ReconstructionAnomalies;
pub use model::{NvtxModel, NvtxModelBuilder};
pub use span::{NvtxCategory, NvtxDomain, NvtxMark, NvtxSpan, NvtxThread, SpanId, SpanKind};
pub use stats::{RangeStats, StatsKey};

// Re-exported so consumers can read span attributes without depending on the
// vocabulary crate directly. Carried verbatim, exactly as captured.
pub use nvtx_events::{NvtxColor, NvtxPayload};
