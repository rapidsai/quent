// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The thin bridge that lets captured NVTX events flow through Quent's typed
//! event pipeline.
//!
//! [`NvtxCapture`] queues owned NVTX records and forwards them through an ordinary
//! Quent observer on a worker thread. [`NvtxEventEntity`] adapts the records to
//! Quent's [`EventPayload`] contract.
//!
//! See `integrations/nvtx/example` for a complete, runnable capture.

use nvtx_events::NvtxEvent;
use quent_events::EventPayload;
use serde::{Deserialize, Serialize};

mod capture;
mod convert;
pub use capture::{CaptureError, NvtxCapture};

/// A `#[serde(transparent)]` newtype over [`NvtxEvent`] implementing
/// [`EventPayload`], naming the `"NvtxEvent"` entity stream. Transparent, so its
/// serialized form is identical to a bare [`NvtxEvent`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NvtxEventEntity(pub NvtxEvent);

impl EventPayload for NvtxEventEntity {
    const NAME: &'static str = "NvtxEvent";
}

impl From<NvtxEvent> for NvtxEventEntity {
    fn from(event: NvtxEvent) -> Self {
        Self(event)
    }
}
