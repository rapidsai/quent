// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX event attributes, payload, and message types.
//!
//! These mirror the fields of `nvtxEventAttributes_t` from the NVTX C API.

use serde::{Deserialize, Serialize};

/// Attributes from `nvtxEventAttributes_t`.
#[derive(Debug, Deserialize, Serialize)]
pub struct NvtxAttributes {
    /// User-defined category ID (0 = no category).
    pub category_id: u32,
    /// ARGB color value, present if `colorType != NVTX_COLOR_UNKNOWN`.
    pub color: Option<u32>,
    /// Typed payload value.
    pub payload: Option<NvtxPayload>,
    /// Event message (ascii string or registered handle reference).
    pub message: Option<NvtxMessage>,
}

/// A typed payload from `nvtxEventAttributes_t`.
#[derive(Debug, Deserialize, Serialize)]
pub enum NvtxPayload {
    U64(u64),
    I64(i64),
    F64(f64),
    U32(u32),
    I32(i32),
    F32(f32),
}

/// An event message, either an inline string or a reference to a
/// previously registered string handle.
#[derive(Debug, Deserialize, Serialize)]
pub enum NvtxMessage {
    /// An ASCII (or converted wchar_t) string.
    Ascii(String),
    /// A reference to a registered string handle. The analyzer resolves
    /// this using prior `RegisterString` events.
    RegisteredHandle(u64),
}
