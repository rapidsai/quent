// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Captured subset of `nvtxEventAttributes_t`.
//!
//! These types mirror the raw NVTX attribute members verbatim (message, color,
//! category, payload). Handles are never resolved at capture time — the analyzer
//! resolves registered strings, domains, and categories from the event stream in
//! a later phase.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::payload::NvtxPayload;

/// A message attached to an NVTX event.
///
/// Registered messages keep only their raw handle, never resolved at capture
/// time. The analyzer maps [`NvtxMessage::RegisteredHandle`] back to its string
/// from the captured [`RegisterString`](crate::NvtxEvent::RegisterString) events.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum NvtxMessage {
    /// An immediate string message, copied verbatim at capture.
    String(String),
    /// A handle to a previously registered string; resolved in the analyzer.
    RegisteredHandle(u64),
}

/// A verbatim NVTX color attribute: the raw `nvtxColorType_t` tag paired with
/// the raw color value (e.g. `NVTX_COLOR_ARGB`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct NvtxColor {
    /// Raw `nvtxColorType_t` tag.
    pub color_type: i32,
    /// Raw color value (e.g. packed ARGB).
    pub value: u32,
}

/// Captured subset of `nvtxEventAttributes_t`.
///
/// Only the members Quent reconstructs from are retained (`category`, `color`,
/// `message`, `payload`); all are stored verbatim with no capture-time
/// resolution or decoding.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct NvtxEventAttributes {
    /// Raw category id (`0` = none). Namespaced by domain in the analyzer.
    pub category: u32,
    /// Optional color attribute.
    pub color: Option<NvtxColor>,
    /// Optional message (immediate string or registered handle).
    pub message: Option<NvtxMessage>,
    /// Optional payload union value from the core `nvtxEventAttributes`.
    pub payload: Option<NvtxPayload>,
}
