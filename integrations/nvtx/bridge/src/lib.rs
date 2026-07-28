// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The thin bridge that lets captured NVTX events flow through Quent's typed
//! event pipeline.
//!
//! It is one adapter: [`NvtxEventEntity`], a newtype over [`NvtxEvent`]
//! implementing Quent's [`EntityEvent`]. The orphan rule forbids that impl in
//! either of *their* crates — the events crate stays Quent-agnostic for
//! upstreaming — so it lives here.
//!
//! See `integrations/nvtx/example` for a complete, runnable capture.

use nvtx_events::NvtxEvent;
use quent_events::EntityEvent;
use serde::{Deserialize, Serialize};

/// A `#[serde(transparent)]` newtype over [`NvtxEvent`] implementing
/// [`EntityEvent`], naming the `"NvtxEvent"` entity stream. Transparent, so its
/// serialized form is identical to a bare [`NvtxEvent`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NvtxEventEntity(pub NvtxEvent);

impl EntityEvent for NvtxEventEntity {
    const NAME: &'static str = "NvtxEvent";
}

impl From<NvtxEvent> for NvtxEventEntity {
    fn from(event: NvtxEvent) -> Self {
        Self(event)
    }
}
