// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The two-pass reconstruction entry point.
//!
//! Walking skeleton: the public surface and the span store exist, so the
//! reconstruction tests compile and fail on their assertions rather than on
//! missing symbols. The replay itself lands next.

use nvtx_bridge::NvtxEventEntity;
use quent_events::Event;

use crate::error::NvtxModelResult;
use crate::span::NvtxSpan;

/// An in-memory model reconstructed from a captured NVTX event stream.
#[derive(Debug, Default)]
pub struct NvtxModel {
    spans: Vec<NvtxSpan>,
}

impl NvtxModel {
    /// Every reconstructed span, in the order it was completed.
    pub fn spans(&self) -> &[NvtxSpan] {
        &self.spans
    }
}

/// Builds an [`NvtxModel`] from a captured event stream.
#[derive(Debug, Default)]
pub struct NvtxModelBuilder;

impl NvtxModelBuilder {
    /// Reconstruct a model from a captured NVTX event stream.
    ///
    /// # Errors
    ///
    /// Returns [`NvtxModelError`](crate::NvtxModelError) only when the stream
    /// itself cannot be obtained. Stream *anomalies* are tolerated, never
    /// returned as errors.
    pub fn build(
        events: impl IntoIterator<Item = Event<NvtxEventEntity>>,
    ) -> NvtxModelResult<NvtxModel> {
        // Skeleton: drain the stream, reconstruct nothing yet.
        let _ = events.into_iter().count();
        Ok(NvtxModel { spans: Vec::new() })
    }
}
