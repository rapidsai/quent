// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The reconstruction core's error type.
//!
//! Deliberately small: tolerance is *by construction*, so stream anomalies
//! (orphan ends, unclosed ranges, out-of-order or duplicate timestamps) are
//! logged with [`tracing::warn!`] and reconstruction continues. They are never
//! modelled as error variants. Only failures to *obtain* the event stream at all
//! — a decode/import failure on the replay path — are errors.

use thiserror::Error;

/// A failure that prevents reconstruction from producing a model at all.
#[derive(Debug, Error)]
pub enum NvtxModelError {
    /// A captured event could not be decoded off the wire or off disk.
    #[error("failed to decode a captured NVTX event: {0}")]
    Decode(String),
    /// Any other error, wrapped verbatim.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl NvtxModelError {
    /// Wrap an arbitrary error as [`NvtxModelError::Other`].
    pub fn other<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Other(Box::new(error))
    }
}

/// The crate-wide result alias.
pub type NvtxModelResult<T> = std::result::Result<T, NvtxModelError>;
